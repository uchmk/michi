//! `l` over the worktree table: land the selected worktree's branch on the
//! trunk without a pull request (take the trunk in, check, fast-forward the
//! trunk, clear the worktree away). The band asks how far to go (1 to 4); the
//! run stops at the first stage that fails and the toast names it.
//! `michi_core::land` does the work, on the act thread.

use michi_core::backup;
use michi_core::land::{Outcome, Request, Stage, Why};
use michi_core::paths;

use crate::act::{self, Task};
use crate::app::App;
use crate::picker::same;
use crate::work::{Band, Then};

impl App {
    /// `l`: asks how far to land the selected worktree's branch.
    pub(crate) fn ask_land(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch, doing, ahead) = (t.wt.path.clone(), t.wt.branch.clone(), t.doing, t.ahead_behind.map(|(a, _)| a));
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            return self.say(self.tr("act.no_trunk"));
        };
        let say = |a: &mut App, key: &'static str| {
            let text = a.tr(key).replace("{wt}", &act::name(&wt)).replace("{trunk}", &trunk);
            let text = text.replace("{doing}", &doing.map_or_else(String::new, |d| a.doing_word(d)));
            a.say_for(text, 6);
        };
        let Some(branch) = branch else { return say(self, "land.detached") };
        if branch == trunk {
            return say(self, "land.self");
        }
        if doing.is_some() {
            return say(self, "act.revert_doing");
        }
        if self.wips.iter().any(|w| same(&w.path, &wt) && !w.files.is_empty()) {
            return say(self, "land.changes");
        }
        if ahead == Some(0) {
            return say(self, "land.nothing");
        }
        let here = self.repo_top().is_some_and(|top| same(&top, &wt));
        let key = paths::key(&wt);
        let sessions: Vec<u64> = self
            .claude
            .iter()
            .flatten()
            .filter(|(p, _)| paths::key(p) == key)
            .flat_map(|(_, s)| s.iter())
            .filter(|s| s.is_agent())
            .map(|s| s.id)
            .collect();
        let name = act::name(&wt);
        let fill = |s: &str| s.replace("{trunk}", &trunk).replace("{branch}", &branch).replace("{wt}", &name);
        let keys = ["land.row_1", "land.row_2", "land.row_3", "land.row_4"];
        // The window's own worktree cannot be removed from here: no stage 4.
        let n = if here { 3 } else { 4 };
        let rows: Vec<(String, String)> = keys.iter().take(n).enumerate().map(|(i, k)| ((i + 1).to_string(), fill(self.tr(k)))).collect();
        let mut notes = Vec::new();
        if here {
            notes.push(self.tr("land.here").trim().to_owned());
        } else if !sessions.is_empty() {
            notes.push(self.tr("land.sessions").trim().to_owned());
        }
        if self.claude_working(&wt) {
            notes.push(self.tr("act.revert_claude").trim().to_owned());
        }
        let text = fill(self.tr("land.ask"));
        let task = Task::Land { req: Request { wt, branch, trunk }, until: Stage::CleanUp, sessions };
        self.band = Some(Band { rows, notes, parents: n as u8, ..Band::new(text, Then::Act(task)) });
    }

    /// What a landing says when it ends: the stages that went through, and
    /// where it stopped and why. A clash opens the `4` view.
    pub(crate) fn landed(&mut self, req: &Request, until: Stage, out: &Outcome) -> (String, u64) {
        let name = act::name(&req.wt);
        let fill = |s: &str| s.replace("{trunk}", &req.trunk).replace("{branch}", &req.branch).replace("{wt}", &name);
        let steps: Vec<String> = out
            .done
            .iter()
            .map(|st| match st {
                Stage::MergeIn if out.merged.is_some() => fill(self.tr("land.ok_merge")),
                Stage::MergeIn => fill(self.tr("land.ok_merge_none")),
                Stage::Verify if out.unchecked => self.tr("land.ok_verify_none").to_owned(),
                Stage::Verify => self.tr("land.ok_verify").to_owned(),
                Stage::FastForward => fill(self.tr("land.ok_ff")).replace("{id}", &out.landed.map_or_else(String::new, |o| o.short())),
                Stage::CleanUp => self.tr("land.ok_clean").to_owned(),
            })
            .collect();
        let steps = steps.join(", ");
        let kept = out.saved.first().map_or_else(String::new, |s| self.tr("land.kept").replace("{}", backup::short(s)));
        let Some((stage, why)) = &out.stopped else {
            let key = if until == Stage::CleanUp { "land.done" } else { "land.partway" };
            return (fill(self.tr(key)).replace("{steps}", &steps) + &kept, if kept.is_empty() { 8 } else { 14 });
        };
        let why_text = match why {
            Why::Conflict(files) => {
                self.show_conflicts(&req.wt);
                fill(self.tr("land.why_conflict")).replace("{files}", &self.few(files))
            }
            Why::Failed { code, tail } => self
                .tr("land.why_failed")
                .replace("{code}", &code.map_or_else(|| "?".to_owned(), |c| c.to_string()))
                .replace("{tail}", tail),
            Why::Refused(s) => s.clone(),
        };
        let done = if steps.is_empty() { self.tr("land.nothing_moved").to_owned() } else { steps };
        let stage_word = fill(self.tr(stage_key(*stage)));
        let text = fill(self.tr("land.stopped")).replace("{stage}", &stage_word).replace("{why}", &why_text).replace("{done}", &done);
        self.last_error = Some(text.clone());
        (text + &kept, 20)
    }
}

fn stage_key(s: Stage) -> &'static str {
    match s {
        Stage::MergeIn => "land.stage_merge",
        Stage::Verify => "land.stage_verify",
        Stage::FastForward => "land.stage_ff",
        Stage::CleanUp => "land.stage_clean",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use michi_core::oid::Oid;
    use michi_core::summary::{Head, Summary};

    use super::*;
    use crate::act::Acted;
    use crate::app::tests::{read, toast};
    use crate::app::Repo;
    use crate::graph::fixture;

    fn on_table() -> App {
        let mut a = read();
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(crate::repo::Answer::Trees(fixture::trees()));
        assert!(a.feed('3'));
        a
    }

    fn request() -> Request {
        Request { wt: PathBuf::from("C:/r/wt"), branch: "wip".into(), trunk: "main".into() }
    }

    fn oid(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    #[test]
    fn l_asks_how_far_and_the_digit_chooses() {
        let mut a = on_table();
        assert!(a.feed('l'));
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to land");
        assert!(a.band.is_none());
        assert!(a.feed('j'));
        assert!(a.feed('l'));
        let band = a.band.as_ref().expect("the band");
        assert_eq!(band.parents, 4);
        assert_eq!(band.rows[0], ("1".to_owned(), "Take main into wip".to_owned()));
        assert_eq!(band.parent_key(1), "band.land_merge");
        assert_eq!(band.parent_key(4), "band.land_all");
        assert!(a.feed('y'));
        assert!(a.band.is_some(), "y does not choose how far");
        assert!(a.feed('5'));
        assert!(a.band.is_some(), "nor a digit past the stages");
        assert!(a.feed('n'));
        assert!(a.band.is_none());
    }

    #[test]
    fn the_digit_sets_the_last_stage() {
        for (digit, stage) in [('1', Stage::MergeIn), ('2', Stage::Verify), ('3', Stage::FastForward), ('4', Stage::CleanUp)] {
            let mut a = on_table();
            a.feed('j');
            a.feed('l');
            let Some(Band { then: Then::Act(task), .. }) = a.band.as_mut() else { panic!() };
            let Task::Land { until, .. } = task else { panic!() };
            assert_eq!(*until, Stage::CleanUp);
            crate::work::set_land_stage(task, digit.to_digit(10).unwrap());
            let Task::Land { until, .. } = task else { panic!() };
            assert_eq!(*until, stage);
            // No reading thread here: the task goes nowhere, but the band is spent.
            assert!(a.feed(digit));
            assert!(a.band.is_none());
        }
    }

    #[test]
    fn what_is_in_the_way_is_said_before_the_band() {
        let mut a = on_table();
        a.feed('j');
        a.feed('j');
        a.feed('l');
        assert_eq!(toast(&a), "old is on no branch: switch it to one before landing it on main");
        let mut trees = fixture::trees();
        trees.trees[1].ahead_behind = Some((0, 2));
        a.take(crate::repo::Answer::Trees(trees));
        a.feed('k');
        a.feed('l');
        assert_eq!(toast(&a), "wt has no commit main lacks: there is nothing to land");
        let mut trees = fixture::trees();
        trees.trees[1].doing = Some(michi_core::branch::Doing::Rebase);
        a.take(crate::repo::Answer::Trees(trees));
        a.feed('l');
        assert_eq!(toast(&a), "wt is in the middle of a rebase: finish it first");
        assert!(a.band.is_none());
    }

    #[test]
    fn the_outcome_names_the_stage_it_stopped_at() {
        let mut a = on_table();
        let id = oid('a');
        let all = Outcome { done: Stage::ALL.to_vec(), merged: Some(id), unchecked: true, landed: Some(id), ..Outcome::default() };
        let (text, _) = a.landed(&request(), Stage::CleanUp, &all);
        assert_eq!(
            text,
            "Landed wip on main: main taken in, no check found (skipped), main moved forward to aaaaaaa, worktree and branch removed"
        );
        let failed = Outcome {
            done: vec![Stage::MergeIn],
            stopped: Some((Stage::Verify, Why::Failed { code: Some(3), tail: "bad thing".into() })),
            ..Outcome::default()
        };
        let (text, secs) = a.landed(&request(), Stage::CleanUp, &failed);
        assert_eq!(text, "Stopped at the check: it failed (exit 3):\nbad thing\nDone: main was in wip already");
        assert_eq!(secs, 20);
        assert!(a.last_error.is_some());
        let why = "the worktree has uncommitted changes".to_owned();
        let refused = Outcome { stopped: Some((Stage::MergeIn, Why::Refused(why))), ..Outcome::default() };
        let (text, _) = a.landed(&request(), Stage::CleanUp, &refused);
        assert!(text.starts_with("Stopped at taking main in: the worktree has uncommitted changes\nDone: nothing"), "{text}");
        let part = Outcome { done: vec![Stage::MergeIn], merged: Some(id), ..Outcome::default() };
        let (text, _) = a.landed(&request(), Stage::MergeIn, &part);
        assert_eq!(text, "wip: main taken in. Land it again to go on.");
        let task = Task::Land { req: request(), until: Stage::CleanUp, sessions: Vec::new() };
        a.act_done(task, Ok(Acted::Landed(all)));
        assert!(toast(&a).starts_with("Landed wip on main"), "{}", toast(&a));
    }

    #[test]
    fn a_clash_opens_the_conflict_view() {
        let mut a = on_table();
        let clash = Outcome { stopped: Some((Stage::MergeIn, Why::Conflict(vec!["a.txt".into()]))), ..Outcome::default() };
        let (text, _) = a.landed(&request(), Stage::CleanUp, &clash);
        assert!(text.contains("a.txt"), "{text}");
        assert!(a.view == crate::app::View::Conflict);
    }
}
