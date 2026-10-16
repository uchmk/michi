//! The branch review's notes on lines (`J` `K` `n` `g`): kept in the review,
//! put away when it closes, handed to the branch's Claude through tsumugi.

use std::path::{Path, PathBuf};

use michi_core::claude::State;
use michi_core::notes::Note;
use michi_core::paths;

use crate::act::Task;
use crate::app::App;

impl App {
    /// The notes of the review being opened come back.
    pub(crate) fn notes_back(&mut self, path: &Path) -> Vec<Note> {
        let key = paths::key(path);
        match self.kept_notes.iter().position(|(p, _)| paths::key(p) == key) {
            Some(i) => self.kept_notes.remove(i).1,
            None => Vec::new(),
        }
    }

    /// The notes of the review being closed are put away.
    pub(crate) fn notes_away(&mut self, path: PathBuf, notes: Vec<Note>) {
        if !notes.is_empty() {
            self.kept_notes.push((path, notes));
        }
    }

    /// The notes were handed over: they are gone from the review and the shelf.
    pub(crate) fn notes_sent(&mut self, wt: &Path) {
        let key = paths::key(wt);
        self.kept_notes.retain(|(p, _)| paths::key(p) != key);
        if let Some(r) = self.review.as_mut().filter(|r| paths::key(&r.path) == key) {
            r.notes.clear();
        }
    }

    /// `n`: writes the note for the cursor's line.
    pub(crate) fn review_note(&mut self) {
        let Some(r) = self.review.as_mut() else { return };
        if !r.write_note() {
            let text = self.tr("review.no_line");
            self.say(text);
        }
    }

    /// `g`: hands the notes to the Claude in the review's worktree.
    pub(crate) fn review_hand(&mut self) {
        let Some(r) = &self.review else { return };
        if r.notes.is_empty() {
            let text = self.tr("review.no_notes");
            return self.say(text);
        }
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            let text = self.tr("act.no_trunk");
            return self.say(text);
        };
        let (wt, branch, notes) = (r.path.clone(), r.branch.clone(), r.notes.clone());
        let key = paths::key(&wt);
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key).map(|(_, s)| s);
        let agents = || sessions.into_iter().flatten().filter(|s| s.is_agent());
        let session = agents().find(|s| s.state != State::Running).or_else(|| agents().next()).map(|s| s.id);
        self.act(Task::HandNotes { wt, branch, trunk, notes, session });
    }
}
