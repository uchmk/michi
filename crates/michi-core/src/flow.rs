//! Gitflow with plain git (#42), without the git-flow extension. The flow's
//! branches are `main` and `develop` with `feature/`, `release/` and `hotfix/`
//! branches off them; the names come from the `gitflow.*` settings when they
//! are there (what `git flow init` writes) and from the defaults when not.
//!
//! * start: a feature or a release branches off `develop`, a hotfix off
//!   `main`; the worktree switches to the new branch.
//! * finish: a feature is merged (`--no-ff`) into `develop`; a release or a
//!   hotfix into `main`, tagged there, and then into `develop`. The branch is
//!   deleted last.
//!
//! Only the worktree it is asked in is touched: a branch another worktree has
//! checked out is never a target (its index and HEAD stay as they are), and a
//! worktree with uncommitted changes or in the middle of something is refused.
//! Every tip that moves or goes is kept under `refs/michi/backup/` first, and
//! a merge that would clash is refused before anything changes.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::oid::Oid;
use crate::paths;
use crate::tag;
use crate::worktree;

/// The three kinds of branch the flow starts and finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Feature,
    Release,
    Hotfix,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Feature, Kind::Release, Kind::Hotfix];

    /// The word for it, the same in every language (`feature`).
    pub fn word(self) -> &'static str {
        match self {
            Kind::Feature => "feature",
            Kind::Release => "release",
            Kind::Hotfix => "hotfix",
        }
    }
}

/// The names the flow uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The production branch (`gitflow.branch.master`).
    pub main: String,
    /// The integration branch (`gitflow.branch.develop`).
    pub develop: String,
    /// Each prefix ends with `/` (`gitflow.prefix.feature` …).
    pub feature: String,
    pub release: String,
    pub hotfix: String,
    /// Put in front of a release's or a hotfix's tag (`gitflow.prefix.versiontag`).
    pub version_tag: String,
}

impl Config {
    /// The defaults, `main` being the production branch.
    pub fn defaults(main: &str) -> Config {
        Config {
            main: main.to_owned(),
            develop: "develop".to_owned(),
            feature: "feature/".to_owned(),
            release: "release/".to_owned(),
            hotfix: "hotfix/".to_owned(),
            version_tag: String::new(),
        }
    }

    pub fn prefix(&self, kind: Kind) -> &str {
        match kind {
            Kind::Feature => &self.feature,
            Kind::Release => &self.release,
            Kind::Hotfix => &self.hotfix,
        }
    }

    /// The kind and the name of a flow branch (`feature/login` is a feature
    /// called `login`); `None` for any other branch or an empty name.
    pub fn parse<'a>(&self, branch: &'a str) -> Option<(Kind, &'a str)> {
        Kind::ALL.into_iter().find_map(|k| branch.strip_prefix(self.prefix(k)).filter(|n| !n.is_empty()).map(|n| (k, n)))
    }

    /// The branch a new one of `kind` starts from.
    pub fn base(&self, kind: Kind) -> &str {
        match kind {
            Kind::Feature | Kind::Release => &self.develop,
            Kind::Hotfix => &self.main,
        }
    }

    /// The branches a finished one of `kind` is merged into, in order.
    pub fn targets(&self, kind: Kind) -> Vec<&str> {
        match kind {
            Kind::Feature => vec![&self.develop],
            Kind::Release | Kind::Hotfix => vec![&self.main, &self.develop],
        }
    }

    /// The tag a finished release or hotfix called `name` gets; none for a feature.
    pub fn tag(&self, kind: Kind, name: &str) -> Option<String> {
        (kind != Kind::Feature).then(|| format!("{}{name}", self.version_tag))
    }
}

/// What the repository of `dir` says about the flow: `gitflow.*` where set,
/// the defaults where not (`master` as the production branch when there is
/// no `main` but there is a `master`).
pub fn config(dir: &Path) -> Result<Config, git::Error> {
    let get = |key: &str| -> Result<Option<String>, git::Error> {
        let out = git::read_maybe(dir, &["config", "--get", key])?;
        Ok(out.map(|o| String::from_utf8_lossy(&o).trim().to_owned()).filter(|v| !v.is_empty()))
    };
    let exists = |name: &str| -> Result<bool, git::Error> { branch_exists(dir, name) };
    let main = match get("gitflow.branch.master")? {
        Some(m) => m,
        None if !exists("main")? && exists("master")? => "master".to_owned(),
        None => "main".to_owned(),
    };
    let mut c = Config::defaults(&main);
    if let Some(d) = get("gitflow.branch.develop")? {
        c.develop = d;
    }
    // A prefix is written with or without the closing slash; it is one here.
    let slashed = |p: String| if p.ends_with('/') { p } else { format!("{p}/") };
    if let Some(p) = get("gitflow.prefix.feature")? {
        c.feature = slashed(p);
    }
    if let Some(p) = get("gitflow.prefix.release")? {
        c.release = slashed(p);
    }
    if let Some(p) = get("gitflow.prefix.hotfix")? {
        c.hotfix = slashed(p);
    }
    if let Some(p) = get("gitflow.prefix.versiontag")? {
        c.version_tag = p;
    }
    Ok(c)
}

fn branch_exists(dir: &Path, name: &str) -> Result<bool, git::Error> {
    Ok(git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}^{{commit}}")])?.is_some())
}

fn tip(dir: &Path, name: &str) -> Result<Oid, git::Error> {
    git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}^{{commit}}")])?
        .and_then(|o| Oid::parse(String::from_utf8_lossy(&o).trim()))
        .ok_or_else(|| git::Error::Git(format!("no branch '{name}'")))
}

/// The branch the worktree at `wt` is on.
pub fn current(wt: &Path) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(wt, &["symbolic-ref", "--short", "-q", "HEAD"])?.map(|o| String::from_utf8_lossy(&o).trim().to_owned()))
}

/// What stands in the way of the flow in the worktree at `wt`: something it
/// is in the middle of, or uncommitted changes (untracked files too).
fn ready(wt: &Path) -> Result<(), git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(git::Error::Git(format!("the worktree is in the middle of a {}", d.word())));
    }
    if !git::read_text(wt, &["status", "--porcelain"])?.is_empty() {
        return Err(git::Error::Git("the worktree has uncommitted changes: commit or put them away first".to_owned()));
    }
    Ok(())
}

/// Starts the branch of `kind` called `name` in the worktree at `wt`: the
/// branch `prefix + name`, off the kind's base, which the worktree then
/// switches to. The new branch's name.
pub fn start(wt: &Path, cfg: &Config, kind: Kind, name: &str) -> Result<String, git::Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(git::Error::Git("the branch needs a name".to_owned()));
    }
    ready(wt)?;
    let new = format!("{}{name}", cfg.prefix(kind));
    branch::check_name(wt, &new)?;
    if branch_exists(wt, &new)? {
        return Err(git::Error::Git(format!("the branch '{new}' is already there")));
    }
    let base = cfg.base(kind);
    let at = tip(wt, base).map_err(|_| git::Error::Git(format!("no branch '{base}' to start from")))?;
    branch::create(wt, &new, &at, Some(wt))?;
    Ok(new)
}

/// What a finish did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    pub kind: Kind,
    /// The branch that was finished (and is gone).
    pub branch: String,
    /// The branches it was merged into, in order; the worktree is on the last.
    pub into: Vec<String>,
    /// The tag made on the production branch (a release or a hotfix).
    pub tag: Option<String>,
    /// Where the finished branch's tip is kept (`refs/michi/backup/…`).
    pub saved: String,
}

/// What a finish of `branch` would do, decided before anything is touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub kind: Kind,
    pub name: String,
    pub into: Vec<String>,
    pub tag: Option<String>,
}

/// Plans the finish of `branch`; `None` when it is not a flow branch.
pub fn plan(cfg: &Config, branch: &str) -> Option<Plan> {
    let (kind, name) = cfg.parse(branch)?;
    Some(Plan { kind, name: name.to_owned(), into: cfg.targets(kind).into_iter().map(str::to_owned).collect(), tag: cfg.tag(kind, name) })
}

/// Why the finish of the flow branch the worktree at `wt` is on cannot go
/// ahead, or the plan when it can: the worktree is clean, the targets are
/// there and not checked out in another worktree, the tag is free, and no
/// merge would clash. Nothing is changed.
pub fn check(wt: &Path, cfg: &Config) -> Result<(String, Plan), git::Error> {
    let branch = current(wt)?.ok_or_else(|| git::Error::Git("the worktree is on no branch".to_owned()))?;
    let plan = plan(cfg, &branch).ok_or_else(|| git::Error::Git(format!("'{branch}' is not a feature, release or hotfix branch")))?;
    ready(wt)?;
    let others = worktree::list(wt)?;
    for target in &plan.into {
        if !branch_exists(wt, target)? {
            return Err(git::Error::Git(format!("no branch '{target}' to merge into")));
        }
        if let Some(w) = others.iter().find(|w| w.branch.as_deref() == Some(target.as_str()) && paths::key(&w.path) != paths::key(wt)) {
            return Err(git::Error::Git(format!("'{target}' is checked out at '{}'", w.path.display())));
        }
        if !git::check(wt, &["merge-tree", "--write-tree", target, &branch])? {
            return Err(git::Error::Git(format!("merging '{branch}' into '{target}' would clash: merge it by hand")));
        }
    }
    if let Some(t) = &plan.tag {
        tag::check_name(wt, t)?;
        if git::read_maybe(wt, &["rev-parse", "--verify", "-q", &format!("refs/tags/{t}")])?.is_some() {
            return Err(git::Error::Git(format!("the tag '{t}' is already there")));
        }
    }
    Ok((branch, plan))
}

/// Finishes the flow branch the worktree at `wt` is on (see the module). The
/// tips of the branch and its targets are kept first. When a step fails
/// after the first merge, what was done stays (the error says which step, the
/// backups hold the old tips).
pub fn finish(wt: &Path, cfg: &Config) -> Result<Finished, git::Error> {
    let (branch, plan) = check(wt, cfg)?;
    let saved = backup::save(wt, &tip(wt, &branch)?, &format!("before finishing {branch}"))?;
    for target in &plan.into {
        backup::save(wt, &tip(wt, target)?, &format!("before merging {branch} into {target}"))?;
    }
    for (n, target) in plan.into.iter().enumerate() {
        let step = |e: git::Error| git::Error::Git(format!("{target}: {e}"));
        branch::switch(wt, target).map_err(step)?;
        let message = format!("Merge branch '{branch}' into {target}");
        if let Err(e) = branch::write(wt, &["merge", "--no-ff", "-m", &message, &branch]) {
            let _ = branch::write(wt, &["merge", "--abort"]);
            let _ = branch::switch(wt, &branch);
            return Err(step(e));
        }
        if n == 0 {
            if let Some(t) = &plan.tag {
                let head = Oid::parse(&git::read_text(wt, &["rev-parse", "HEAD"])?).ok_or_else(|| git::Error::Io("no HEAD".to_owned()))?;
                tag::create(wt, t, &head, &format!("{} {}", capital(plan.kind.word()), plan.name)).map_err(step)?;
            }
        }
    }
    branch::write(wt, &["branch", "-d", &branch])?;
    Ok(Finished { kind: plan.kind, branch, into: plan.into, tag: plan.tag, saved })
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    /// A repository with `main` (a.txt = 1) and `develop` on top of it.
    fn flow(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        t.write(&t.repo, "a.txt", "1\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.git(&t.repo, &["branch", "develop"]);
        t
    }

    fn commit(t: &TempRepo, dir: &Path, file: &str, text: &str) {
        t.write(dir, file, text);
        t.git(dir, &["add", file]);
        t.git(dir, &["commit", "-q", "-m", file]);
    }

    #[test]
    fn defaults_and_gitflow_settings_are_read() {
        let t = flow("flow-config");
        let c = config(&t.repo).unwrap();
        assert_eq!(c, Config::defaults("main"));
        t.git(&t.repo, &["config", "gitflow.branch.master", "prod"]);
        t.git(&t.repo, &["config", "gitflow.branch.develop", "dev"]);
        t.git(&t.repo, &["config", "gitflow.prefix.feature", "feat"]);
        t.git(&t.repo, &["config", "gitflow.prefix.versiontag", "v"]);
        let c = config(&t.repo).unwrap();
        assert_eq!((c.main.as_str(), c.develop.as_str(), c.feature.as_str(), c.version_tag.as_str()), ("prod", "dev", "feat/", "v"));
        assert_eq!(c.release, "release/");
    }

    #[test]
    fn master_is_the_production_branch_when_there_is_no_main() {
        let t = flow("flow-master");
        t.git(&t.repo, &["branch", "-m", "main", "master"]);
        assert_eq!(config(&t.repo).unwrap().main, "master");
    }

    #[test]
    fn branches_are_parsed_and_planned() {
        let c = Config { version_tag: "v".into(), ..Config::defaults("main") };
        assert_eq!(c.parse("feature/login"), Some((Kind::Feature, "login")));
        assert_eq!(c.parse("release/1.2.0"), Some((Kind::Release, "1.2.0")));
        assert_eq!(c.parse("hotfix/x/y"), Some((Kind::Hotfix, "x/y")));
        assert_eq!(c.parse("feature/"), None);
        assert_eq!(c.parse("main"), None);
        assert_eq!(c.parse("features/x"), None);
        assert_eq!((c.base(Kind::Feature), c.base(Kind::Release), c.base(Kind::Hotfix)), ("develop", "develop", "main"));
        let p = plan(&c, "release/1.2.0").unwrap();
        assert_eq!((p.into, p.tag), (vec!["main".to_owned(), "develop".to_owned()], Some("v1.2.0".to_owned())));
        let p = plan(&c, "feature/login").unwrap();
        assert_eq!((p.into, p.tag), (vec!["develop".to_owned()], None));
        assert_eq!(plan(&c, "main"), None);
    }

    #[test]
    fn start_branches_off_the_base_and_switches() {
        let t = flow("flow-start");
        let c = config(&t.repo).unwrap();
        commit(&t, &t.repo, "main-only.txt", "m\n");
        // develop is one behind main now; a feature starts at develop, a hotfix at main.
        assert_eq!(start(&t.repo, &c, Kind::Hotfix, "1.0.1").unwrap(), "hotfix/1.0.1");
        assert_eq!(current(&t.repo).unwrap().as_deref(), Some("hotfix/1.0.1"));
        assert!(t.repo.join("main-only.txt").exists());
        t.git(&t.repo, &["switch", "-q", "main"]);
        assert_eq!(start(&t.repo, &c, Kind::Feature, "login").unwrap(), "feature/login");
        assert!(!t.repo.join("main-only.txt").exists());
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), t.out(&t.repo, &["rev-parse", "develop"]));
    }

    #[test]
    fn start_refuses_a_dirty_worktree_a_taken_name_and_a_missing_base() {
        let t = flow("flow-start-no");
        let c = config(&t.repo).unwrap();
        t.write(&t.repo, "a.txt", "edited\n");
        let e = start(&t.repo, &c, Kind::Feature, "x").unwrap_err().to_string();
        assert!(e.contains("uncommitted"), "{e}");
        t.git(&t.repo, &["checkout", "-q", "a.txt"]);
        t.write(&t.repo, "new.txt", "n\n");
        assert!(start(&t.repo, &c, Kind::Feature, "x").is_err(), "untracked files count");
        std::fs::remove_file(t.repo.join("new.txt")).unwrap();
        t.git(&t.repo, &["branch", "feature/x"]);
        assert!(start(&t.repo, &c, Kind::Feature, "x").unwrap_err().to_string().contains("already"));
        assert!(start(&t.repo, &c, Kind::Feature, " ").is_err());
        assert!(start(&t.repo, &c, Kind::Feature, "bad name").is_err());
        t.git(&t.repo, &["branch", "-D", "develop"]);
        assert!(start(&t.repo, &c, Kind::Release, "1").unwrap_err().to_string().contains("develop"));
        assert_eq!(current(&t.repo).unwrap().as_deref(), Some("main"));
    }

    #[test]
    fn a_feature_is_merged_into_develop_with_a_merge_commit_and_goes() {
        let t = flow("flow-feature");
        let c = config(&t.repo).unwrap();
        start(&t.repo, &c, Kind::Feature, "login").unwrap();
        commit(&t, &t.repo, "login.txt", "l\n");
        let before_main = t.out(&t.repo, &["rev-parse", "main"]);
        let done = finish(&t.repo, &c).unwrap();
        assert_eq!(
            (done.kind, done.branch.as_str(), done.into.clone(), done.tag.clone()),
            (Kind::Feature, "feature/login", vec!["develop".to_owned()], None)
        );
        assert_eq!(current(&t.repo).unwrap().as_deref(), Some("develop"));
        assert!(t.repo.join("login.txt").exists());
        assert_eq!(t.out(&t.repo, &["rev-list", "--parents", "-1", "develop"]).split(' ').count(), 3, "a merge commit");
        assert_eq!(t.out(&t.repo, &["log", "-1", "--format=%s", "develop"]), "Merge branch 'feature/login' into develop");
        assert!(!branch_exists(&t.repo, "feature/login").unwrap());
        assert_eq!(t.out(&t.repo, &["rev-parse", "main"]), before_main);
        assert!(done.saved.starts_with(backup::PREFIX));
        // The branch's tip and develop's old tip are kept.
        let kept = t.out(&t.repo, &["for-each-ref", "--format=%(refname)", backup::PREFIX]);
        assert_eq!(kept.lines().count(), 2, "{kept}");
    }

    #[test]
    fn a_release_is_merged_into_main_tagged_and_merged_into_develop() {
        let t = flow("flow-release");
        let mut c = config(&t.repo).unwrap();
        c.version_tag = "v".into();
        start(&t.repo, &c, Kind::Release, "1.0.0").unwrap();
        commit(&t, &t.repo, "version.txt", "1.0.0\n");
        let done = finish(&t.repo, &c).unwrap();
        assert_eq!(done.into, vec!["main".to_owned(), "develop".to_owned()]);
        assert_eq!(done.tag.as_deref(), Some("v1.0.0"));
        assert_eq!(t.out(&t.repo, &["cat-file", "-t", "refs/tags/v1.0.0"]), "tag");
        assert_eq!(t.out(&t.repo, &["rev-parse", "v1.0.0^{commit}"]), t.out(&t.repo, &["rev-parse", "main"]));
        for b in ["main", "develop"] {
            assert_eq!(t.out(&t.repo, &["show", &format!("{b}:version.txt")]), "1.0.0", "{b}");
        }
        assert_eq!(current(&t.repo).unwrap().as_deref(), Some("develop"));
        assert!(!branch_exists(&t.repo, "release/1.0.0").unwrap());
    }

    #[test]
    fn a_hotfix_starts_at_main() {
        let t = flow("flow-hotfix");
        let c = config(&t.repo).unwrap();
        start(&t.repo, &c, Kind::Hotfix, "1.0.1").unwrap();
        commit(&t, &t.repo, "fix.txt", "f\n");
        let done = finish(&t.repo, &c).unwrap();
        assert_eq!(done.tag.as_deref(), Some("1.0.1"));
        assert!(t.out(&t.repo, &["show", "develop:fix.txt"]).starts_with('f'));
    }

    #[test]
    fn finish_refuses_and_changes_nothing() {
        let t = flow("flow-finish-no");
        let c = config(&t.repo).unwrap();
        // Not a flow branch.
        assert!(finish(&t.repo, &c).unwrap_err().to_string().contains("not a feature"));
        start(&t.repo, &c, Kind::Release, "1.0").unwrap();
        commit(&t, &t.repo, "r.txt", "r\n");
        // Uncommitted changes.
        t.write(&t.repo, "r.txt", "dirty\n");
        assert!(finish(&t.repo, &c).unwrap_err().to_string().contains("uncommitted"));
        t.git(&t.repo, &["checkout", "-q", "r.txt"]);
        // A tag that is taken.
        t.git(&t.repo, &["tag", "1.0"]);
        assert!(finish(&t.repo, &c).unwrap_err().to_string().contains("already"));
        t.git(&t.repo, &["tag", "-d", "1.0"]);
        // A target another worktree has checked out.
        let other = t.root.join("other");
        t.git(&t.repo, &["worktree", "add", "-q", other.to_str().unwrap(), "main"]);
        let e = finish(&t.repo, &c).unwrap_err().to_string();
        assert!(e.contains("checked out"), "{e}");
        assert_eq!(current(&other).unwrap().as_deref(), Some("main"));
        t.git(&t.repo, &["worktree", "remove", other.to_str().unwrap()]);
        // A clash.
        t.git(&t.repo, &["switch", "-q", "main"]);
        commit(&t, &t.repo, "r.txt", "main's\n");
        t.git(&t.repo, &["switch", "-q", "release/1.0"]);
        let e = finish(&t.repo, &c).unwrap_err().to_string();
        assert!(e.contains("clash"), "{e}");
        assert_eq!(current(&t.repo).unwrap().as_deref(), Some("release/1.0"));
        assert!(branch_exists(&t.repo, "release/1.0").unwrap());
        assert!(t.out(&t.repo, &["for-each-ref", backup::PREFIX]).is_empty(), "nothing was kept: nothing was done");
    }

    #[test]
    fn the_other_worktrees_head_and_index_are_left_alone() {
        let t = flow("flow-other");
        let c = config(&t.repo).unwrap();
        let other = t.root.join("other");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "side", other.to_str().unwrap(), "main"]);
        t.write(&other, "side.txt", "s\n");
        start(&t.repo, &c, Kind::Feature, "x").unwrap();
        commit(&t, &t.repo, "x.txt", "x\n");
        finish(&t.repo, &c).unwrap();
        assert_eq!(current(&other).unwrap().as_deref(), Some("side"));
        assert_eq!(t.out(&other, &["status", "--porcelain"]), "?? side.txt");
    }
}
