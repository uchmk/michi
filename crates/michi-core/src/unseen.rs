//! The commits not seen yet: for each worktree but the trunk's, the commits
//! its `HEAD` has gained since the user last looked (docs/design.md: the blue
//! dot and the bold subject; the worktree table's count and its list of
//! unseen changes). Selecting one in the graph marks it seen.
//!
//! What has been seen is kept per repository in michi's state folder, outside
//! the repository: for each branch (or detached worktree) a base, the commit
//! everything up to which counts as seen, and the commits after it seen one
//! by one. The first time a worktree is seen, its `HEAD` is the base: only
//! what comes after counts. Once every commit after the base is seen, the
//! base moves up to the `HEAD`. The commits the trunk has never count: a
//! branch that takes in the trunk has nothing new from that.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::git;
use crate::oid::Oid;
use crate::paths;
use crate::track::Trees;

/// The most commits counted per worktree.
pub const MOST: usize = 200;

/// A commit not seen yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct New {
    pub id: Oid,
    /// The commit's time, Unix seconds.
    pub time: i64,
    pub subject: String,
}

/// A worktree's unseen commits, the newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    /// Its branch, else its folder's name.
    pub name: String,
    pub commits: Vec<New>,
    /// What its marks go by: its branch, or `@` and its folder.
    pub key: String,
}

/// Every worktree's unseen commits; a worktree not followed (the trunk's,
/// a bare one, one whose folder is gone) has no entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unseen {
    pub tracks: Vec<Track>,
}

impl Unseen {
    /// The unseen commits of the worktree at `path`; `None` when it is not
    /// followed.
    pub fn of(&self, path: &Path) -> Option<&[New]> {
        self.tracks.iter().find(|t| t.path == path).map(|t| t.commits.as_slice())
    }

    /// Every unseen commit (each once), with the worktree's name, the newest
    /// first.
    pub fn all(&self) -> Vec<(&str, &New)> {
        let mut once = HashSet::new();
        let mut out: Vec<(&str, &New)> = self
            .tracks
            .iter()
            .flat_map(|t| t.commits.iter().map(move |c| (t.name.as_str(), c)))
            .filter(|(_, c)| once.insert(c.id))
            .collect();
        out.sort_by_key(|n| std::cmp::Reverse(n.1.time));
        out
    }

    pub fn ids(&self) -> HashSet<Oid> {
        self.tracks.iter().flat_map(|t| t.commits.iter().map(|c| c.id)).collect()
    }

    /// Takes the commit `id` off every list (the window marks it seen before
    /// the watching thread has saved it).
    pub fn remove(&mut self, id: &Oid) -> bool {
        let mut gone = false;
        for t in &mut self.tracks {
            let before = t.commits.len();
            t.commits.retain(|c| c.id != *id);
            gone |= t.commits.len() != before;
        }
        gone
    }
}

/// What has been seen, as the state file keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marks {
    #[serde(default)]
    branches: BTreeMap<String, Mark>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Mark {
    /// Everything up to this commit is seen.
    base: String,
    /// Commits after the base seen one by one.
    #[serde(default)]
    seen: Vec<String>,
}

impl Marks {
    /// From the state file's text; a file that cannot be read starts afresh.
    pub fn parse(text: &str) -> Marks {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Marks the commit `id` seen in each worktree of `unseen` that has it;
    /// whether anything changed.
    pub fn see(&mut self, unseen: &Unseen, id: &Oid) -> bool {
        let mut changed = false;
        for t in unseen.tracks.iter().filter(|t| t.commits.iter().any(|c| c.id == *id)) {
            let Some(m) = self.branches.get_mut(&t.key) else { continue };
            let hex = id.to_string();
            if !m.seen.contains(&hex) {
                m.seen.push(hex);
                changed = true;
            }
        }
        changed
    }
}

/// The key a worktree's marks go by: its branch, or `@` and its folder.
fn key(branch: Option<&str>, path: &Path) -> String {
    match branch {
        Some(b) => b.to_owned(),
        None => format!("@{}", paths::key(path)),
    }
}

/// The commits already listed, by `(head, base, trunk)`: most stay where they
/// were from one reading to the next.
#[derive(Default)]
pub struct Cache(HashMap<(Oid, Oid, Option<Oid>), Vec<New>>);

/// The unseen commits of each worktree in `trees` (of the repository `dir` is
/// in), with `marks` brought up to date: a worktree seen the first time gets
/// its base. Returns whether `marks` changed (to be saved).
pub fn read(dir: &Path, trees: &Trees, marks: &mut Marks, cache: &mut Cache) -> (Unseen, bool) {
    let before = marks.clone();
    let mut tracks = Vec::new();
    let mut kept = HashMap::new();
    let mut keys = HashSet::new();
    for t in &trees.trees {
        let k = key(t.wt.branch.as_deref(), &t.wt.path);
        keys.insert(k.clone());
        if t.wt.bare || t.wt.prunable.is_some() || (t.wt.branch.is_some() && t.wt.branch == trees.trunk) {
            continue;
        }
        let Some(head) = Oid::parse(&t.wt.head).filter(|h| !h.is_zero()) else { continue };
        let mark = marks.branches.entry(k.clone()).or_insert_with(|| Mark { base: head.to_string(), seen: Vec::new() });
        let mut commits = match Oid::parse(&mark.base) {
            Some(base) if base == head => Vec::new(),
            Some(base) => {
                let at = (head, base, trees.trunk_head);
                let got = match cache.0.get(&at) {
                    Some(c) => Some(c.clone()),
                    None => after(dir, &head, &base, trees.trunk_head.as_ref()).ok(),
                };
                match got {
                    Some(c) => {
                        kept.insert(at, c.clone());
                        c
                    }
                    // The base is gone (the branch was made anew): start over from here.
                    None => {
                        *mark = Mark { base: head.to_string(), seen: Vec::new() };
                        Vec::new()
                    }
                }
            }
            None => {
                *mark = Mark { base: head.to_string(), seen: Vec::new() };
                Vec::new()
            }
        };
        let listed: HashSet<String> = commits.iter().map(|c| c.id.to_string()).collect();
        mark.seen.retain(|s| listed.contains(s));
        commits.retain(|c| !mark.seen.contains(&c.id.to_string()));
        if commits.is_empty() && mark.base != head.to_string() {
            *mark = Mark { base: head.to_string(), seen: Vec::new() };
        }
        let name = t.wt.branch.clone().unwrap_or_else(|| folder(&t.wt.path));
        tracks.push(Track { path: t.wt.path.clone(), name, commits, key: k });
    }
    // A worktree gone takes its marks along (the trunk's never had any).
    if !trees.trees.is_empty() {
        marks.branches.retain(|k, _| keys.contains(k));
    }
    cache.0 = kept;
    let changed = *marks != before;
    (Unseen { tracks }, changed)
}

fn folder(path: &Path) -> String {
    let p = path.to_string_lossy();
    p.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(&p).to_owned()
}

/// The commits `head` has that neither `base` nor the trunk has, the newest
/// first, [`MOST`] at most.
fn after(dir: &Path, head: &Oid, base: &Oid, trunk: Option<&Oid>) -> Result<Vec<New>, git::Error> {
    let most = format!("--max-count={MOST}");
    let (head, base) = (head.to_string(), base.to_string());
    let trunk = trunk.map(Oid::to_string);
    let mut args = vec!["log", "--no-show-signature", "--format=%H%x00%ct%x00%s", &most, &head, "--not", &base];
    if let Some(t) = &trunk {
        args.push(t);
    }
    args.push("--");
    Ok(parse(&git::read(dir, &args)?))
}

/// `log --format=%H%x00%ct%x00%s`'s lines.
pub fn parse(out: &[u8]) -> Vec<New> {
    String::from_utf8_lossy(out)
        .lines()
        .filter_map(|line| {
            let mut f = line.splitn(3, '\0');
            let id = Oid::parse(f.next()?)?;
            let time = f.next()?.parse().ok()?;
            Some(New { id, time, subject: f.next().unwrap_or("").to_owned() })
        })
        .collect()
}

/// The state file's name for the repository whose common git folder is
/// `common`: its name and a hash of the whole path, so two clones of the same
/// name keep apart.
pub fn file_name(name: &str, common: &Path) -> String {
    // FNV-1a: the same on every machine and every Rust.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in paths::key(common).bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    format!("{safe}-{h:016x}.json")
}

/// The common git folder of the repository `dir` is in, in full.
pub fn common_dir(dir: &Path) -> Result<PathBuf, git::Error> {
    git::read_text(dir, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use crate::track;
    use crate::worktree;

    fn trees(r: &TempRepo) -> Trees {
        track::read(&r.repo, &worktree::list(&r.repo).unwrap(), &mut track::Cache::default()).unwrap()
    }

    fn subjects(u: &Unseen, path: &Path) -> Vec<String> {
        u.of(path).unwrap().iter().map(|c| c.subject.clone()).collect()
    }

    #[test]
    fn log_lines_are_read() {
        let a = "a".repeat(40);
        let out = format!("{a}\x001767225600\x00fix: the \x00 thing\nbad\n{}\x00x\x00y\n", "b".repeat(40));
        let n = parse(out.as_bytes());
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].id.to_string(), a);
        assert_eq!(n[0].time, 1_767_225_600);
        assert_eq!(n[0].subject, "fix: the \x00 thing");
    }

    #[test]
    fn file_names_keep_two_clones_apart() {
        let a = file_name("michi", Path::new("C:/dev/michi/.git"));
        let b = file_name("michi", Path::new("C:/work/michi/.git"));
        assert!(a.starts_with("michi-") && a.ends_with(".json"));
        assert_ne!(a, b);
        assert_eq!(a, file_name("michi", Path::new("C:/dev/michi/.git")));
        assert!(file_name("a/b:c", Path::new("/x")).starts_with("a_b_c-"));
    }

    #[test]
    fn marks_survive_the_state_file() {
        let mut m = Marks::default();
        m.branches.insert("feat".into(), Mark { base: "a".repeat(40), seen: vec!["b".repeat(40)] });
        assert_eq!(Marks::parse(&m.to_json()), m);
        assert_eq!(Marks::parse("not json"), Marks::default());
    }

    #[test]
    fn new_commits_count_until_seen() {
        let r = TempRepo::new("unseen");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt"]);
        let wt = r.root.join("wt");
        let (mut marks, mut cache) = (Marks::default(), Cache::default());
        // The first look: nothing is new, and the trunk is not followed.
        let (u, changed) = read(&r.repo, &trees(&r), &mut marks, &mut cache);
        assert!(changed);
        assert_eq!(u.tracks.len(), 1);
        assert!(u.of(&r.repo).is_none());
        assert_eq!(subjects(&u, &wt), Vec::<String>::new());
        r.git(&wt, &["commit", "-q", "--allow-empty", "-m", "f1"]);
        r.git(&wt, &["commit", "-q", "--allow-empty", "-m", "f2"]);
        // The trunk's commits taken in are not new.
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "m1"]);
        r.git(&wt, &["merge", "-q", "--no-edit", "main"]);
        let (u, changed) = read(&r.repo, &trees(&r), &mut marks, &mut cache);
        assert!(!changed);
        assert_eq!(subjects(&u, &wt), ["Merge branch 'main' into feat", "f2", "f1"]);
        assert_eq!(u.all().len(), 3);
        // One seen: the others stay.
        let f2 = u.of(&wt).unwrap()[1].id;
        assert!(marks.see(&u, &f2));
        assert!(!marks.see(&u, &f2));
        let (u, _) = read(&r.repo, &trees(&r), &mut marks, &mut cache);
        assert_eq!(subjects(&u, &wt), ["Merge branch 'main' into feat", "f1"]);
        // Read again from a file.
        let mut marks = Marks::parse(&marks.to_json());
        let (u, _) = read(&r.repo, &trees(&r), &mut marks, &mut Cache::default());
        assert_eq!(subjects(&u, &wt), ["Merge branch 'main' into feat", "f1"]);
        // All seen: the base moves up.
        for c in u.of(&wt).unwrap().to_vec() {
            marks.see(&u, &c.id);
        }
        let (u, changed) = read(&r.repo, &trees(&r), &mut marks, &mut cache);
        assert!(changed);
        assert!(u.of(&wt).unwrap().is_empty());
        let head = r.out(&wt, &["rev-parse", "HEAD"]);
        assert_eq!(marks.branches["feat"], Mark { base: head, seen: Vec::new() });
    }

    #[test]
    fn a_branch_made_anew_starts_over_and_a_gone_worktree_goes() {
        let r = TempRepo::new("unseen-anew");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt"]);
        let wt = r.root.join("wt");
        let mut marks = Marks::default();
        // A base no commit has.
        marks.branches.insert("feat".into(), Mark { base: "1".repeat(40), seen: Vec::new() });
        marks.branches.insert("old".into(), Mark { base: "2".repeat(40), seen: Vec::new() });
        let (u, changed) = read(&r.repo, &trees(&r), &mut marks, &mut Cache::default());
        assert!(changed);
        assert!(u.of(&wt).unwrap().is_empty());
        assert_eq!(marks.branches["feat"].base, r.out(&wt, &["rev-parse", "HEAD"]));
        assert!(!marks.branches.contains_key("old"));
    }
}
