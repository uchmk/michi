//! Git's English refusals as a cause and a next step (issue #108). The text
//! git wrote is matched against a table; the window says what happened, why,
//! and what to do next, and keeps git's own words as the detail. Login and
//! network trouble is `auth.rs`'s, so it is not here.

use crate::git;

/// One refusal git is known to make.
pub struct Rule {
    /// Every one of these must be in git's text (compared in lower case).
    pub all: &'static [&'static str],
    /// At least one of these must be too, when there are any.
    pub any: &'static [&'static str],
    /// The i18n key of what happened and why.
    pub cause: &'static str,
    /// The i18n key of what to do next.
    pub next: &'static str,
}

const fn rule(all: &'static [&'static str], any: &'static [&'static str], cause: &'static str, next: &'static str) -> Rule {
    Rule { all, any, cause, next }
}

/// Most specific first: the first rule that fits is the one.
pub const RULES: &[Rule] = &[
    rule(
        &[],
        &["tip of your current branch is behind", "non-fast-forward", "fetch first", "stale info"],
        "gerr.non_ff.cause",
        "gerr.non_ff.next",
    ),
    rule(&["your local changes to the following files would be overwritten"], &[], "gerr.dirty.cause", "gerr.dirty.next"),
    rule(&["untracked working tree files would be overwritten"], &[], "gerr.untracked.cause", "gerr.untracked.next"),
    rule(&["you have unmerged paths"], &[], "gerr.unmerged.cause", "gerr.unmerged.next"),
    rule(
        &[],
        &["you need to resolve your current index first", "merging is not possible because you have unmerged files", "needs merge"],
        "gerr.unmerged.cause",
        "gerr.unmerged.next",
    ),
    rule(
        &[],
        &["automatic merge failed", "conflict (content)", "conflict (add/add)", "conflict (modify/delete)"],
        "gerr.conflict.cause",
        "gerr.conflict.next",
    ),
    rule(&["could not apply"], &[], "gerr.conflict.cause", "gerr.conflict.next"),
    rule(
        &[],
        &["rebase-merge", "rebase-apply", "it seems that there is already a rebase-merge directory"],
        "gerr.rebasing.cause",
        "gerr.rebasing.next",
    ),
    rule(&["is not fully merged"], &[], "gerr.unmerged_branch.cause", "gerr.unmerged_branch.next"),
    rule(&[], &["is already checked out at", "is already used by worktree at"], "gerr.checked_out.cause", "gerr.checked_out.next"),
    rule(&["a branch named", "already exists"], &[], "gerr.branch_exists.cause", "gerr.branch_exists.next"),
    rule(&["already exists"], &[], "gerr.path_exists.cause", "gerr.path_exists.next"),
    rule(&[], &["index.lock", "another git process seems to be running"], "gerr.locked.cause", "gerr.locked.next"),
    rule(&["has no upstream branch"], &[], "gerr.no_upstream.cause", "gerr.no_upstream.next"),
    rule(
        &[],
        &["no tracking information for the current branch", "no upstream configured"],
        "gerr.no_upstream.cause",
        "gerr.no_upstream.next",
    ),
    rule(
        &[],
        &["please tell me who you are", "unable to auto-detect email address", "empty ident name"],
        "gerr.identity.cause",
        "gerr.identity.next",
    ),
    rule(&[], &["nothing to commit", "nothing added to commit", "no changes added to commit"], "gerr.nothing.cause", "gerr.nothing.next"),
    rule(&["aborting commit due to empty commit message"], &[], "gerr.empty_message.cause", "gerr.empty_message.next"),
    rule(&[], &["protected branch", "gh006", "gh013"], "gerr.protected.cause", "gerr.protected.next"),
    rule(&[], &["hook declined", "hook failed", "pre-commit hook", "commit-msg hook"], "gerr.hook.cause", "gerr.hook.next"),
    rule(&[], &["file size limit", "exceeds github's"], "gerr.large.cause", "gerr.large.next"),
    rule(&["dubious ownership"], &[], "gerr.ownership.cause", "gerr.ownership.next"),
    rule(&["you are not currently on a branch"], &[], "gerr.detached.cause", "gerr.detached.next"),
    rule(
        &[],
        &["bad revision", "unknown revision", "not a valid object name", "invalid reference", "couldn't find remote ref"],
        "gerr.no_ref.cause",
        "gerr.no_ref.next",
    ),
    rule(&["did not match any file"], &[], "gerr.no_path.cause", "gerr.no_path.next"),
    rule(&["contains modified or untracked files"], &[], "gerr.tree_dirty.cause", "gerr.tree_dirty.next"),
];

/// The rule for what git said, if it is a known refusal.
pub fn explain(said: &str) -> Option<&'static Rule> {
    let s = said.to_ascii_lowercase();
    RULES.iter().find(|r| r.all.iter().all(|w| s.contains(w)) && (r.any.is_empty() || r.any.iter().any(|w| s.contains(w))))
}

/// [`explain`] for an error: all git said when it said more than a line.
pub fn explain_error(e: &git::Error) -> Option<&'static Rule> {
    match e {
        git::Error::Git(_) | git::Error::Told { .. } => explain(&e.all()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cause(said: &str) -> Option<&'static str> {
        explain(said).map(|r| r.cause)
    }

    /// Real texts, as git 2.4x writes them.
    #[test]
    fn real_git_texts_get_their_rule() {
        let table = [
            (
                "! [rejected]        main -> main (non-fast-forward)\nerror: failed to push some refs to 'origin'\nhint: Updates were rejected because the tip of your current branch is behind",
                "gerr.non_ff.cause",
            ),
            ("! [rejected]        main -> main (fetch first)", "gerr.non_ff.cause"),
            ("! [rejected]        main -> main (stale info)", "gerr.non_ff.cause"),
            (
                "error: Your local changes to the following files would be overwritten by merge:\n\ta.txt\nPlease commit your changes or stash them before you merge.\nAborting",
                "gerr.dirty.cause",
            ),
            (
                "error: Your local changes to the following files would be overwritten by checkout:\n\ta.txt",
                "gerr.dirty.cause",
            ),
            ("error: The following untracked working tree files would be overwritten by checkout:\n\tb.txt", "gerr.untracked.cause"),
            ("error: you need to resolve your current index first", "gerr.unmerged.cause"),
            ("fatal: Exiting because of an unresolved conflict.\nerror: you have unmerged paths", "gerr.unmerged.cause"),
            ("fatal: Exiting because of unfinished merge.\nerror: Merging is not possible because you have unmerged files.", "gerr.unmerged.cause"),
            ("CONFLICT (content): Merge conflict in a.txt\nAutomatic merge failed; fix conflicts and then commit the result.", "gerr.conflict.cause"),
            ("error: could not apply 1a2b3c4... msg", "gerr.conflict.cause"),
            ("error: The branch 'feat' is not fully merged.\nIf you are sure you want to delete it, run 'git branch -D feat'.", "gerr.unmerged_branch.cause"),
            ("fatal: 'feat' is already checked out at 'C:/dev/x'", "gerr.checked_out.cause"),
            ("fatal: 'feat' is already used by worktree at 'C:/dev/x'", "gerr.checked_out.cause"),
            ("fatal: a branch named 'feat' already exists", "gerr.branch_exists.cause"),
            ("fatal: 'C:/dev/x-feat' already exists", "gerr.path_exists.cause"),
            (
                "fatal: Unable to create 'C:/r/.git/index.lock': File exists.\n\nAnother git process seems to be running in this repository",
                "gerr.locked.cause",
            ),
            ("fatal: The current branch feat has no upstream branch.\nTo push the current branch and set the remote as upstream, use", "gerr.no_upstream.cause"),
            ("There is no tracking information for the current branch.", "gerr.no_upstream.cause"),
            ("Author identity unknown\n\n*** Please tell me who you are.\n\nRun\n  git config --global user.email", "gerr.identity.cause"),
            ("fatal: unable to auto-detect email address (got 'x@y.(none)')", "gerr.identity.cause"),
            ("On branch main\nnothing to commit, working tree clean", "gerr.nothing.cause"),
            ("Aborting commit due to empty commit message.", "gerr.empty_message.cause"),
            (
                "remote: error: GH006: Protected branch update failed for refs/heads/main.\n ! [remote rejected] main -> main (protected branch hook declined)",
                "gerr.protected.cause",
            ),
            ("error: failed to push some refs\n ! [remote rejected] main -> main (pre-receive hook declined)", "gerr.hook.cause"),
            ("husky - pre-commit hook exited with code 1 (error)", "gerr.hook.cause"),
            ("remote: error: File big.bin is 120.00 MB; this exceeds GitHub's file size limit of 100.00 MB", "gerr.large.cause"),
            ("fatal: detected dubious ownership in repository at 'C:/x'", "gerr.ownership.cause"),
            ("fatal: You are not currently on a branch.\nTo push the history leading to the current (detached HEAD)", "gerr.detached.cause"),
            ("fatal: bad revision 'nope'", "gerr.no_ref.cause"),
            ("fatal: ambiguous argument 'x': unknown revision or path not in the working tree.", "gerr.no_ref.cause"),
            ("fatal: Not a valid object name nope", "gerr.no_ref.cause"),
            ("fatal: couldn't find remote ref nope", "gerr.no_ref.cause"),
            ("error: pathspec 'x.txt' did not match any file(s) known to git", "gerr.no_path.cause"),
            ("fatal: 'C:/dev/x-feat' contains modified or untracked files, use --force to delete it", "gerr.tree_dirty.cause"),
        ];
        for (said, want) in table {
            assert_eq!(cause(said), Some(want), "{said}");
        }
    }

    #[test]
    fn what_is_not_known_is_left_as_git_said_it() {
        assert_eq!(cause("fatal: something never seen"), None);
        assert_eq!(cause(""), None);
        assert!(explain_error(&git::Error::NoGit).is_none());
        assert!(explain_error(&git::Error::Io("git: boom".into())).is_none());
    }

    #[test]
    fn a_told_error_is_read_whole() {
        let e = git::Error::Told {
            line: "error: failed to push some refs".into(),
            all: "error: failed to push\n ! [rejected] a -> a (fetch first)".into(),
        };
        assert_eq!(explain_error(&e).map(|r| r.cause), Some("gerr.non_ff.cause"));
    }

    #[test]
    fn a_real_git_refusal_is_explained() {
        use crate::testrepo::TempRepo;
        let t = TempRepo::configured("explain-real");
        let err = git::read(&t.repo, &["switch", "no-such-branch"]).unwrap_err();
        assert_eq!(explain_error(&err).map(|r| r.cause), Some("gerr.no_ref.cause"), "{err}");
        let err = git::read(&t.repo, &["branch", "main"]).unwrap_err();
        assert_eq!(explain_error(&err).map(|r| r.cause), Some("gerr.branch_exists.cause"), "{err}");
    }
}
