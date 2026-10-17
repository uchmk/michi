//! Plain sentences for a beginner (#106, #107): where the worktree stands
//! right now, and what an operation is about to do. Each builder is a pure
//! function of what is already known about the repository and gives
//! [`Phrase`]s: an i18n key with its `{name}` values, so the window words them
//! in the language of common.toml and this module stays free of any language.
//! The words themselves are in michi's `i18n.rs` under the keys of [`KEYS`].

use crate::branch::Doing;

/// One sentence: an i18n key and the values for its `{name}` holes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phrase {
    pub key: &'static str,
    pub args: Vec<(&'static str, String)>,
}

impl Phrase {
    fn new(key: &'static str) -> Phrase {
        Phrase { key, args: Vec::new() }
    }

    fn with(mut self, name: &'static str, value: impl ToString) -> Phrase {
        self.args.push((name, value.to_string()));
        self
    }

    /// `template` (the words for [`Phrase::key`]) with the holes filled.
    pub fn fill(&self, template: &str) -> String {
        self.args.iter().fold(template.to_owned(), |t, (name, value)| t.replace(&format!("{{{name}}}"), value))
    }
}

/// How a branch stands against the remote branch it follows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Remote {
    /// It follows nothing.
    None,
    /// It followed a remote branch (the name) which is no more.
    Gone(String),
    /// It follows `name`: commits only it has (`ahead`), only `name` has (`behind`).
    Tracking { name: String, ahead: u32, behind: u32 },
}

/// What is known of the worktree for [`where_now`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Here {
    /// The branch; `None` when on no branch.
    pub branch: Option<String>,
    /// The short hash, for a worktree on no branch.
    pub head: String,
    /// In the middle of a merge, a rebase and the like.
    pub doing: Option<Doing>,
    /// Files with uncommitted changes (untracked ones too).
    pub changed: usize,
    pub remote: Remote,
}

/// What the worktree is in the middle of, as the key of its sentence.
fn doing_key(d: Doing) -> &'static str {
    match d {
        Doing::Merge => "plain.doing_merge",
        Doing::Rebase => "plain.doing_rebase",
        Doing::CherryPick => "plain.doing_cherry",
        Doing::Revert => "plain.doing_revert",
        Doing::Bisect => "plain.doing_bisect",
    }
}

/// The sentences for "where am I now": the branch, what is not committed,
/// how it stands against the remote, each with the key to press next.
pub fn where_now(h: &Here) -> Vec<Phrase> {
    let mut v = Vec::new();
    let branch = h.branch.clone().unwrap_or_default();
    if let Some(d) = h.doing {
        v.push(Phrase::new(doing_key(d)).with("branch", &branch).with("head", &h.head));
        return v;
    }
    match &h.branch {
        Some(b) => v.push(Phrase::new("plain.on").with("branch", b)),
        None => {
            v.push(Phrase::new("plain.detached").with("head", &h.head));
        }
    }
    v.push(match h.changed {
        0 => Phrase::new("plain.clean"),
        n => Phrase::new("plain.changed").with("n", n),
    });
    if h.branch.is_some() {
        v.push(match &h.remote {
            Remote::None => Phrase::new("plain.no_remote"),
            Remote::Gone(up) => Phrase::new("plain.gone").with("up", up),
            Remote::Tracking { name, ahead: 0, behind: 0 } => Phrase::new("plain.synced").with("up", name),
            Remote::Tracking { name, ahead, behind: 0 } => Phrase::new("plain.ahead").with("up", name).with("n", ahead),
            Remote::Tracking { name, ahead: 0, behind } => Phrase::new("plain.behind").with("up", name).with("n", behind),
            Remote::Tracking { name, ahead, behind } => Phrase::new("plain.diverged").with("up", name).with("a", ahead).with("b", behind),
        });
    }
    v
}

/// What a merge or a pick is expected to do about conflicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outlook {
    /// Not looked into.
    Unknown,
    /// It goes through.
    Clean,
    /// It stops on this many files.
    Conflicts(usize),
}

impl Outlook {
    fn phrase(self) -> Phrase {
        match self {
            Outlook::Unknown => Phrase::new("plain.outlook_unknown"),
            Outlook::Clean => Phrase::new("plain.outlook_clean"),
            Outlook::Conflicts(n) => Phrase::new("plain.outlook_conflicts").with("n", n),
        }
    }
}

/// `p`: what a pull of `branch` from `up` is about to do. `changed`: the
/// uncommitted files, which are put away and brought back around it.
pub fn pull_plan(branch: &str, up: &str, ahead: u32, behind: u32, changed: usize) -> Vec<Phrase> {
    let mut v = Vec::new();
    v.push(match (ahead, behind) {
        (_, 0) => return vec![Phrase::new("plain.pull_nothing").with("up", up)],
        (0, n) => Phrase::new("plain.pull_ff").with("up", up).with("n", n).with("branch", branch),
        (a, b) => return vec![Phrase::new("plain.pull_diverged").with("up", up).with("a", a).with("b", b)],
    });
    if changed > 0 {
        v.push(Phrase::new("plain.carry").with("n", changed));
    }
    v
}

/// `m`: what taking `commits` commits of `rev` into `branch` is about to do
/// (`commits`: how many are new to the branch, when known).
pub fn merge_plan(rev: &str, branch: &str, commits: Option<u32>, outlook: Outlook) -> Vec<Phrase> {
    let first = match commits {
        Some(0) => return vec![Phrase::new("plain.merge_nothing").with("rev", rev).with("branch", branch)],
        Some(n) => Phrase::new("plain.merge_n").with("rev", rev).with("n", n).with("branch", branch),
        None => Phrase::new("plain.merge").with("rev", rev).with("branch", branch),
    };
    vec![first, outlook.phrase(), Phrase::new("plain.undo_merge")]
}

/// Cherry-pick: one commit copied onto `branch`.
pub fn pick_plan(id: &str, branch: &str, outlook: Outlook) -> Vec<Phrase> {
    vec![Phrase::new("plain.pick").with("id", id).with("branch", branch), outlook.phrase(), Phrase::new("plain.undo_pick")]
}

/// Switching to `name`: the files on disk change to its, the uncommitted
/// `changed` ones go with the worktree.
pub fn switch_plan(from: Option<&str>, name: &str, changed: usize) -> Vec<Phrase> {
    let mut v = vec![Phrase::new("plain.switch").with("from", from.unwrap_or("-")).with("name", name)];
    if changed > 0 {
        v.push(Phrase::new("plain.carry").with("n", changed));
    }
    v
}

/// Every key the builders use: michi's words must have all of them.
pub const KEYS: &[&str] = &[
    "plain.on",
    "plain.detached",
    "plain.clean",
    "plain.changed",
    "plain.no_remote",
    "plain.gone",
    "plain.synced",
    "plain.ahead",
    "plain.behind",
    "plain.diverged",
    "plain.doing_merge",
    "plain.doing_rebase",
    "plain.doing_cherry",
    "plain.doing_revert",
    "plain.doing_bisect",
    "plain.outlook_unknown",
    "plain.outlook_clean",
    "plain.outlook_conflicts",
    "plain.pull_nothing",
    "plain.pull_ff",
    "plain.pull_diverged",
    "plain.carry",
    "plain.merge_nothing",
    "plain.merge_n",
    "plain.merge",
    "plain.undo_merge",
    "plain.pick",
    "plain.undo_pick",
    "plain.switch",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn here(branch: Option<&str>, changed: usize, remote: Remote) -> Here {
        Here { branch: branch.map(str::to_owned), head: "abc1234".into(), doing: None, changed, remote }
    }

    fn keys(v: &[Phrase]) -> Vec<&'static str> {
        v.iter().map(|p| p.key).collect()
    }

    fn tracking(ahead: u32, behind: u32) -> Remote {
        Remote::Tracking { name: "origin/feat".into(), ahead, behind }
    }

    #[test]
    fn a_clean_branch_that_matches_its_remote() {
        let v = where_now(&here(Some("feat"), 0, tracking(0, 0)));
        assert_eq!(keys(&v), ["plain.on", "plain.clean", "plain.synced"]);
        assert_eq!(v[0].args, [("branch", "feat".to_owned())]);
    }

    #[test]
    fn uncommitted_files_are_counted() {
        let v = where_now(&here(Some("feat"), 3, tracking(0, 0)));
        assert_eq!(v[1].key, "plain.changed");
        assert_eq!(v[1].fill("{n} files"), "3 files");
    }

    #[test]
    fn the_remote_is_told_in_each_way_it_can_stand() {
        let last = |r| where_now(&here(Some("f"), 0, r)).pop().unwrap();
        assert_eq!(last(Remote::None).key, "plain.no_remote");
        assert_eq!(last(Remote::Gone("origin/f".into())).key, "plain.gone");
        assert_eq!(last(tracking(2, 0)).key, "plain.ahead");
        assert_eq!(last(tracking(0, 4)).key, "plain.behind");
        let d = last(tracking(2, 4));
        assert_eq!(d.key, "plain.diverged");
        assert_eq!(d.fill("{up} {a} {b}"), "origin/feat 2 4");
    }

    #[test]
    fn no_branch_says_so_and_has_no_remote_sentence() {
        let v = where_now(&here(None, 1, Remote::None));
        assert_eq!(keys(&v), ["plain.detached", "plain.changed"]);
        assert_eq!(v[0].fill("{head}"), "abc1234");
    }

    #[test]
    fn the_middle_of_something_is_the_only_sentence() {
        for (d, key) in [
            (Doing::Merge, "plain.doing_merge"),
            (Doing::Rebase, "plain.doing_rebase"),
            (Doing::CherryPick, "plain.doing_cherry"),
            (Doing::Revert, "plain.doing_revert"),
            (Doing::Bisect, "plain.doing_bisect"),
        ] {
            let h = Here { doing: Some(d), ..here(Some("feat"), 5, tracking(1, 1)) };
            assert_eq!(keys(&where_now(&h)), [key]);
        }
    }

    #[test]
    fn a_pull_is_told_by_what_it_takes_in() {
        assert_eq!(keys(&pull_plan("f", "origin/f", 0, 0, 2)), ["plain.pull_nothing"]);
        assert_eq!(keys(&pull_plan("f", "origin/f", 1, 2, 0)), ["plain.pull_diverged"]);
        assert_eq!(keys(&pull_plan("f", "origin/f", 0, 3, 0)), ["plain.pull_ff"]);
        let v = pull_plan("f", "origin/f", 0, 3, 2);
        assert_eq!(keys(&v), ["plain.pull_ff", "plain.carry"]);
        assert_eq!(v[0].fill("{n} from {up} into {branch}"), "3 from origin/f into f");
    }

    #[test]
    fn a_merge_says_how_many_commits_and_what_may_conflict() {
        let v = merge_plan("main", "feat", Some(3), Outlook::Conflicts(2));
        assert_eq!(keys(&v), ["plain.merge_n", "plain.outlook_conflicts", "plain.undo_merge"]);
        assert_eq!(v[1].fill("{n}"), "2");
        assert_eq!(keys(&merge_plan("main", "feat", Some(0), Outlook::Clean)), ["plain.merge_nothing"]);
        assert_eq!(keys(&merge_plan("main", "feat", None, Outlook::Unknown))[..2], ["plain.merge", "plain.outlook_unknown"]);
        assert_eq!(merge_plan("m", "f", Some(1), Outlook::Clean)[1].key, "plain.outlook_clean");
    }

    #[test]
    fn a_pick_and_a_switch() {
        assert_eq!(keys(&pick_plan("abc", "feat", Outlook::Unknown)), ["plain.pick", "plain.outlook_unknown", "plain.undo_pick"]);
        assert_eq!(keys(&switch_plan(Some("a"), "b", 0)), ["plain.switch"]);
        let v = switch_plan(None, "b", 4);
        assert_eq!(keys(&v), ["plain.switch", "plain.carry"]);
        assert_eq!(v[0].fill("{from}>{name}"), "->b");
    }

    #[test]
    fn every_key_a_builder_gives_is_listed() {
        let mut given = Vec::new();
        for remote in [Remote::None, Remote::Gone("o".into()), tracking(0, 0), tracking(1, 0), tracking(0, 1), tracking(1, 1)] {
            for h in [here(Some("f"), 0, remote.clone()), here(Some("f"), 1, remote.clone()), here(None, 1, remote)] {
                given.extend(where_now(&h));
            }
        }
        for d in [Doing::Merge, Doing::Rebase, Doing::CherryPick, Doing::Revert, Doing::Bisect] {
            given.extend(where_now(&Here { doing: Some(d), ..here(Some("f"), 0, Remote::None) }));
        }
        for o in [Outlook::Unknown, Outlook::Clean, Outlook::Conflicts(1)] {
            given.extend(merge_plan("m", "f", Some(1), o));
            given.extend(merge_plan("m", "f", None, o));
            given.extend(pick_plan("a", "f", o));
        }
        given.extend(merge_plan("m", "f", Some(0), Outlook::Clean));
        given.extend(pull_plan("f", "o", 0, 0, 0));
        given.extend(pull_plan("f", "o", 1, 1, 0));
        given.extend(pull_plan("f", "o", 0, 1, 1));
        given.extend(switch_plan(Some("a"), "b", 1));
        for p in &given {
            assert!(KEYS.contains(&p.key), "{} is not in KEYS", p.key);
        }
        for k in KEYS {
            assert!(given.iter().any(|p| p.key == *k), "{k} is never given");
        }
    }
}
