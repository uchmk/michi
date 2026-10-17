//! Landing a worktree's branch on the trunk without a pull request: take the
//! trunk in, check the result, fast-forward the trunk to it, and clear the
//! worktree away. One run goes as far as `until` says and stops at the first
//! stage that fails, naming it, so the person sees where it stopped and can
//! go on from there (every stage is safe to run again).
//!
//! Nothing here is lost: the trunk only ever moves forward (`--ff-only`), a
//! clash on the way in is left marked in the worktree for the conflict view,
//! and the branch goes with `branch::delete`, which keeps its tip under
//! `refs/michi/backup/` when no other ref has it.

use std::path::{Path, PathBuf};

use crate::branch;
use crate::conflict::{self, Outcome as Merged};
use crate::git;
use crate::oid::Oid;
use crate::tidy::{self, Item, Why as Listed};
use crate::worktree;

/// The stages, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Merge the trunk into the branch, in the branch's worktree.
    MergeIn,
    /// Run the repository's check on the result.
    Verify,
    /// Move the trunk forward to the branch (fast-forward only).
    FastForward,
    /// Take the worktree away and delete the branch.
    CleanUp,
}

impl Stage {
    pub const ALL: [Stage; 4] = [Stage::MergeIn, Stage::Verify, Stage::FastForward, Stage::CleanUp];

    /// The word for it, the same in every language.
    pub fn word(self) -> &'static str {
        match self {
            Stage::MergeIn => "merge",
            Stage::Verify => "verify",
            Stage::FastForward => "fast-forward",
            Stage::CleanUp => "clean up",
        }
    }
}

/// What to land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The worktree whose branch lands.
    pub wt: PathBuf,
    pub branch: String,
    /// The branch it lands on.
    pub trunk: String,
}

/// Why a stage stopped the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// The merge clashed in these files, left marked in the worktree.
    Conflict(Vec<String>),
    /// The check failed: its exit code and the end of what it wrote.
    Failed { code: Option<i32>, tail: String },
    /// Something is in the way, in git's or michi's words.
    Refused(String),
}

/// How far a run got.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The stages that went through, in order.
    pub done: Vec<Stage>,
    /// The new head when the merge made a commit.
    pub merged: Option<Oid>,
    /// The check was skipped: the repository has none.
    pub unchecked: bool,
    /// The trunk's tip after the fast-forward.
    pub landed: Option<Oid>,
    /// The backups the clean-up made.
    pub saved: Vec<String>,
    /// The stage that failed, and why; the run ended there.
    pub stopped: Option<(Stage, Why)>,
}

/// The command that checks the worktree at `wt`: `scripts/verify.sh` (run by
/// `bash`) or `scripts/verify.ps1` (by `pwsh`); `None` when it has neither.
pub fn verify_command(wt: &Path) -> Option<Vec<String>> {
    let has = |name: &str| wt.join("scripts").join(name).is_file();
    if has("verify.sh") {
        Some(vec!["bash".into(), "scripts/verify.sh".into()])
    } else if has("verify.ps1") {
        Some(["pwsh", "-NoProfile", "-File", "scripts/verify.ps1"].map(String::from).to_vec())
    } else {
        None
    }
}

/// Lands `req` as far as `until`. `before_clean_up` runs just before the
/// clean-up stage (the window closes the worktree's tabs there). Blocks: run
/// it off the UI thread.
pub fn run(dir: &Path, req: &Request, until: Stage, before_clean_up: impl FnOnce()) -> Outcome {
    let mut out = Outcome::default();
    let stop = |out: &mut Outcome, stage: Stage, why: Why| {
        out.stopped = Some((stage, why));
    };
    let trunk_wt = match preflight(dir, req) {
        Ok(t) => t,
        Err(why) => {
            stop(&mut out, Stage::MergeIn, why);
            return out;
        }
    };

    // 1. The trunk in.
    match conflict::merge(&req.wt, &req.trunk) {
        Ok(Merged::Made(new)) => out.merged = Some(new),
        Ok(Merged::Conflict(files)) => {
            stop(&mut out, Stage::MergeIn, Why::Conflict(files));
            return out;
        }
        Ok(_) => {}
        Err(e) => {
            stop(&mut out, Stage::MergeIn, Why::Refused(e.to_string()));
            return out;
        }
    }
    out.done.push(Stage::MergeIn);
    if until == Stage::MergeIn {
        return out;
    }

    // 2. The check.
    match verify_command(&req.wt) {
        None => out.unchecked = true,
        Some(cmd) => {
            if let Err(why) = verify(&req.wt, &cmd) {
                stop(&mut out, Stage::Verify, why);
                return out;
            }
        }
    }
    out.done.push(Stage::Verify);
    if until == Stage::Verify {
        return out;
    }

    // 3. The trunk forward.
    match fast_forward(dir, req, trunk_wt.as_deref()) {
        Ok(tip) => out.landed = Some(tip),
        Err(e) => {
            stop(&mut out, Stage::FastForward, Why::Refused(e.to_string()));
            return out;
        }
    }
    out.done.push(Stage::FastForward);
    if until == Stage::FastForward {
        return out;
    }

    // 4. The worktree and the branch away.
    before_clean_up();
    let item = Item {
        branch: Some(req.branch.clone()),
        tip: out.landed,
        worktree: Some(req.wt.clone()),
        why: Listed::Merged,
        lost: false,
        stay: None,
    };
    let gone = tidy::remove(dir, &[item]);
    out.saved = gone.saved;
    if let Some((_, why)) = gone.stayed.into_iter().next() {
        let why = if why.is_empty() { "the branch moved".to_owned() } else { why };
        stop(&mut out, Stage::CleanUp, Why::Refused(why));
        return out;
    }
    out.done.push(Stage::CleanUp);
    out
}

/// What must hold before anything moves; the worktree the trunk is checked
/// out in (`None`: no worktree has it).
fn preflight(dir: &Path, req: &Request) -> Result<Option<PathBuf>, Why> {
    let refuse = |e: git::Error| Why::Refused(e.to_string());
    let say = |s: String| Why::Refused(s);
    if req.branch == req.trunk {
        return Err(say(format!("{} is the trunk itself", req.trunk)));
    }
    let all = worktree::list(dir).map_err(refuse)?;
    let here = all.iter().find(|w| w.path == req.wt);
    if here.is_none_or(|w| w.branch.as_deref() != Some(req.branch.as_str())) {
        return Err(say(format!("{} is not on {}", req.wt.display(), req.branch)));
    }
    if let Some(d) = branch::doing(&req.wt) {
        return Err(say(format!("the worktree is in the middle of a {}", d.word())));
    }
    if !git::read_text(&req.wt, &["status", "--porcelain"]).map_err(refuse)?.trim().is_empty() {
        return Err(say("the worktree has uncommitted changes".to_owned()));
    }
    let trunk_ref = format!("refs/heads/{}", req.trunk);
    if git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("{trunk_ref}^{{commit}}")]).map_err(refuse)?.is_none() {
        return Err(say(format!("no branch {}", req.trunk)));
    }
    if git::check(dir, &["merge-base", "--is-ancestor", &req.branch, &trunk_ref]).map_err(refuse)? {
        return Err(say(format!("nothing to land: {} has no commit {} lacks", req.branch, req.trunk)));
    }
    let trunk_wt = all.iter().find(|w| w.branch.as_deref() == Some(req.trunk.as_str())).map(|w| w.path.clone());
    if let Some(t) = &trunk_wt {
        if let Some(d) = branch::doing(t) {
            return Err(say(format!("the worktree of {} is in the middle of a {}", req.trunk, d.word())));
        }
    }
    Ok(trunk_wt)
}

/// Runs the check in the worktree; its failure carries the end of its output.
fn verify(wt: &Path, cmd: &[String]) -> Result<(), Why> {
    let mut c = std::process::Command::new(&cmd[0]);
    c.args(&cmd[1..]).current_dir(wt).stdin(std::process::Stdio::null());
    git::hide_window(&mut c);
    let out = c.output().map_err(|e| Why::Refused(format!("{}: {e}", cmd[0])))?;
    if out.status.success() {
        return Ok(());
    }
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = lines[lines.len().saturating_sub(12)..].join("\n");
    Err(Why::Failed { code: out.status.code(), tail })
}

/// Moves the trunk to the branch's tip, only forward: in the worktree that has
/// the trunk checked out, else by the ref alone. The trunk's new tip.
fn fast_forward(dir: &Path, req: &Request, trunk_wt: Option<&Path>) -> Result<Oid, git::Error> {
    let tip = |name: &str| -> Result<Oid, git::Error> {
        git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}^{{commit}}")])?
            .and_then(|o| Oid::parse(String::from_utf8_lossy(&o).trim()))
            .ok_or_else(|| git::Error::Git(format!("no branch {name}")))
    };
    let (old, new) = (tip(&req.trunk)?, tip(&req.branch)?);
    if !git::check(dir, &["merge-base", "--is-ancestor", &old.to_string(), &new.to_string()])? {
        return Err(git::Error::Git(format!("{} moved on and is no longer behind {}: take it in again", req.trunk, req.branch)));
    }
    match trunk_wt {
        Some(t) => branch::write(t, &["merge", "--ff-only", &req.branch])?,
        None => branch::write(dir, &["update-ref", &format!("refs/heads/{}", req.trunk), &new.to_string(), &old.to_string()])?,
    }
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    /// `main` (a commit `m` on top of `first`), and `feat` in a worktree with
    /// a commit `f` on top of `first`.
    fn setup(name: &str) -> (TempRepo, Request) {
        let t = TempRepo::configured(name);
        let wt = t.root.join("feat");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "feat", &wt.to_string_lossy()]);
        t.configure(&wt);
        t.write(&t.repo, "m.txt", "m\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "m"]);
        t.write(&wt, "f.txt", "f\n");
        t.git(&wt, &["add", "."]);
        t.git(&wt, &["commit", "-q", "-m", "f"]);
        let req = Request { wt, branch: "feat".into(), trunk: "main".into() };
        (t, req)
    }

    fn rev(t: &TempRepo, dir: &Path, r: &str) -> String {
        t.out(dir, &["rev-parse", r])
    }

    #[test]
    fn it_lands_through_every_stage_and_clears_up() {
        let (t, req) = setup("land-all");
        let out = run(&t.repo, &req, Stage::CleanUp, || {});
        assert_eq!(out.stopped, None);
        assert_eq!(out.done, Stage::ALL);
        assert!(out.merged.is_some() && out.unchecked);
        assert_eq!(out.landed.map(|o| o.to_string()), Some(rev(&t, &t.repo, "main")));
        assert!(t.repo.join("f.txt").exists() && t.repo.join("m.txt").exists(), "the trunk's worktree moved too");
        assert!(!req.wt.exists());
        assert_eq!(t.out(&t.repo, &["branch", "--list", "feat"]), "");
        assert!(out.saved.is_empty(), "main has its commits: {:?}", out.saved);
    }

    #[test]
    fn it_stops_where_asked_and_goes_on_from_there() {
        let (t, req) = setup("land-until");
        let main = rev(&t, &t.repo, "main");
        let out = run(&t.repo, &req, Stage::MergeIn, || panic!("not yet"));
        assert_eq!(out.done, [Stage::MergeIn]);
        assert_eq!(rev(&t, &t.repo, "main"), main, "the trunk has not moved");
        assert_eq!(t.out(&req.wt, &["rev-list", "--count", "main..HEAD"]), "2");
        let out = run(&t.repo, &req, Stage::FastForward, || panic!("not yet"));
        assert_eq!(out.done, [Stage::MergeIn, Stage::Verify, Stage::FastForward]);
        assert_eq!(out.merged, None, "the trunk was in already");
        assert_eq!(rev(&t, &t.repo, "main"), rev(&t, &req.wt, "HEAD"));
        assert!(req.wt.exists());
    }

    #[test]
    fn a_clash_stops_at_the_merge_and_stays_marked() {
        let (t, req) = setup("land-clash");
        t.write(&t.repo, "x.txt", "main\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "x on main"]);
        t.write(&req.wt, "x.txt", "feat\n");
        t.git(&req.wt, &["add", "."]);
        t.git(&req.wt, &["commit", "-q", "-m", "x on feat"]);
        let main = rev(&t, &t.repo, "main");
        let out = run(&t.repo, &req, Stage::CleanUp, || panic!("no"));
        assert_eq!(out.stopped, Some((Stage::MergeIn, Why::Conflict(vec!["x.txt".into()]))));
        assert!(out.done.is_empty());
        assert_eq!(branch::doing(&req.wt), Some(branch::Doing::Merge));
        assert_eq!(rev(&t, &t.repo, "main"), main);
    }

    #[test]
    fn a_failing_check_stops_at_verify_with_its_tail() {
        let (t, req) = setup("land-verify");
        t.write(&req.wt, "scripts/verify.sh", "echo checking\necho bad thing >&2\nexit 3\n");
        t.git(&req.wt, &["add", "."]);
        t.git(&req.wt, &["commit", "-q", "-m", "verify"]);
        if std::process::Command::new("bash").arg("-c").arg("true").status().is_err() {
            return;
        }
        let main = rev(&t, &t.repo, "main");
        let out = run(&t.repo, &req, Stage::CleanUp, || panic!("no"));
        let Some((Stage::Verify, Why::Failed { code, tail })) = out.stopped else { panic!("{:?}", out.stopped) };
        assert_eq!(code, Some(3));
        assert!(tail.contains("checking") && tail.contains("bad thing"), "{tail}");
        assert_eq!(out.done, [Stage::MergeIn]);
        assert_eq!(rev(&t, &t.repo, "main"), main);
        assert!(req.wt.exists());
    }

    #[test]
    fn a_passing_check_goes_on() {
        let (t, req) = setup("land-verify-ok");
        t.write(&req.wt, "scripts/verify.sh", "echo ALL OK\n");
        t.git(&req.wt, &["add", "."]);
        t.git(&req.wt, &["commit", "-q", "-m", "verify"]);
        if std::process::Command::new("bash").arg("-c").arg("true").status().is_err() {
            return;
        }
        let out = run(&t.repo, &req, Stage::FastForward, || {});
        assert_eq!(out.stopped, None);
        assert!(!out.unchecked);
        assert_eq!(out.done.len(), 3);
    }

    #[test]
    fn a_trunk_that_moved_after_the_merge_stops_the_fast_forward() {
        let (t, req) = setup("land-moved");
        run(&t.repo, &req, Stage::Verify, || {});
        t.write(&t.repo, "late.txt", "late\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "late"]);
        let main = rev(&t, &t.repo, "main");
        let out = run(&t.repo, &req, Stage::FastForward, || {});
        // Running again takes the late commit in first, so it goes through.
        assert_eq!(out.stopped, None);
        assert_ne!(rev(&t, &t.repo, "main"), main);
        // Moved between the check and the move: refused, the trunk untouched.
        let (t, req) = setup("land-moved2");
        let trunk_wt = preflight(&t.repo, &req).unwrap();
        t.write(&t.repo, "late.txt", "late\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "late"]);
        let main = rev(&t, &t.repo, "main");
        let e = fast_forward(&t.repo, &req, trunk_wt.as_deref());
        assert!(e.is_err());
        assert_eq!(rev(&t, &t.repo, "main"), main);
    }

    #[test]
    fn it_refuses_what_is_in_the_way_before_anything_moves() {
        let (t, req) = setup("land-refuse");
        let main = rev(&t, &t.repo, "main");
        t.write(&req.wt, "dirty.txt", "d\n");
        let out = run(&t.repo, &req, Stage::CleanUp, || {});
        assert_eq!(out.stopped, Some((Stage::MergeIn, Why::Refused("the worktree has uncommitted changes".into()))));
        std::fs::remove_file(req.wt.join("dirty.txt")).unwrap();
        let same = Request { branch: "main".into(), ..req.clone() };
        assert!(matches!(run(&t.repo, &same, Stage::CleanUp, || {}).stopped, Some((Stage::MergeIn, Why::Refused(_)))));
        // Nothing to land: the trunk has it all.
        t.git(&t.repo, &["merge", "-q", "feat"]);
        let out = run(&t.repo, &req, Stage::CleanUp, || {});
        let Some((Stage::MergeIn, Why::Refused(why))) = out.stopped else { panic!() };
        assert!(why.starts_with("nothing to land"), "{why}");
        assert_ne!(rev(&t, &t.repo, "main"), main);
    }

    #[test]
    fn a_trunk_no_worktree_has_moves_by_its_ref() {
        let (t, req) = setup("land-bare-trunk");
        // Free `main` in the main worktree.
        t.git(&t.repo, &["switch", "-q", "--detach"]);
        let out = run(&t.repo, &req, Stage::FastForward, || {});
        assert_eq!(out.stopped, None);
        assert_eq!(rev(&t, &t.repo, "main"), rev(&t, &req.wt, "HEAD"));
    }
}
