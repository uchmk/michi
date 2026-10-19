//! `michi mcp`: Claude Code reads how the worktrees stand, from the window
//! (#102). Read-only: no tool writes, and none moves the window.
//!
//! A windowed program has no standard input to speak MCP on, so the same exe
//! is the bridge, as kura's:
//!
//! ```text
//! Claude Code ──stdio (JSON-RPC)── michi mcp ──socket / named pipe── michi (the window)
//! ```
//!
//! The window listens on a per-user door; the first window to
//! open it holds it, the others try again every half minute. A request is
//! answered on the UI thread from the state already in memory.

use std::io;
use std::sync::mpsc::Receiver;

use ito_ipc::door::{self, Door};
use ito_mcp::serde_json::{json, Value};
use ito_mcp::{Server, Tool};
use serde::{Deserialize, Serialize};

/// The door: `\.\pipe\michi-<user>` on Windows; `MICHI_ADDRESS` names another.
/// The version is bumped when [`Request`] or the reply change shape.
const DOOR: Door<Request> = Door::new("michi", "MICHI_ADDRESS", 1);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Each worktree: branch, commits ahead of and behind the trunk, upstream,
    /// what it is in the middle of, uncommitted files.
    Worktrees,
    /// Which worktrees would conflict with which, and in which files.
    Conflicts,
}

/// One request handed to the UI thread, with where its answer goes.
pub type Ask = door::Ask<Request>;

/// Opens the door on a thread of its own and hands each request to the UI
/// thread through the returned channel, waking it with `ctx`.
pub fn listen(ctx: egui::Context) -> Receiver<Ask> {
    DOOR.listen(move || ctx.request_repaint())
}

/// Asks the running window, for `michi mcp`.
fn ask(request: Request) -> door::Reply {
    door::ask(&DOOR, &request)
}

/// The MCP server `michi mcp` runs on its standard input and output.
pub fn server() -> Server {
    Server::new("michi", env!("CARGO_PKG_VERSION"))
        .instructions(
            "michi is the Git client the user has open on a repository many Claude Code sessions work \
             in at once, each in its own worktree. michi_worktrees says how each worktree stands; \
             michi_conflicts says which ones would conflict if merged, and in which files. Both only \
             read: nothing here changes the repository.",
        )
        .tool(Tool::new(
            "michi_worktrees",
            "Every worktree of the repository the user's michi window shows: its folder, branch, commits ahead of \
             and behind the trunk (behind_main), its upstream, whether it is in the middle of a merge or rebase, \
             and its uncommitted files. Use it before starting work, to see what the other sessions have left.",
            |_| ask(Request::Worktrees),
        ))
        .tool(Tool::new(
            "michi_conflicts",
            "Which pairs of worktrees (and the trunk) would conflict if merged, from michi's prediction: the \
             kind (committed, or only with uncommitted changes) and the conflicted files with their hunk counts. \
             Use it to avoid editing a file another session is also changing.",
            |_| ask(Request::Conflicts),
        ))
}

/// `michi mcp`: serve until the client closes standard input.
pub fn run() -> ! {
    let code = match server().serve(io::stdin().lock(), io::stdout().lock()) {
        Ok(()) => 0,
        Err(_) => 1,
    };
    std::process::exit(code)
}

fn conflict_json(c: &michi_core::predict::Conflict) -> Value {
    json!({
        "places": c.places(),
        "files": c.files.iter().map(|f| json!({ "path": f.path, "hunks": f.hunks })).collect::<Vec<_>>(),
    })
}

/// The window's answer to a request, as pretty JSON.
pub fn answer_json(app: &crate::app::App, request: Request) -> String {
    use michi_core::clash::Outcome;
    let v = match request {
        Request::Worktrees => {
            let rows: Vec<Value> = app
                .trees
                .trees
                .iter()
                .map(|t| {
                    let files: Vec<String> =
                        app.wips.iter().find(|w| w.path == t.wt.path).map(|w| w.files.iter().map(|f| f.path.clone()).collect()).unwrap_or_default();
                    json!({
                        "path": t.wt.path.display().to_string(),
                        "branch": t.wt.branch,
                        "head": t.wt.head.get(..7).unwrap_or(&t.wt.head),
                        "ahead_of_main": t.ahead_behind.map(|(a, _)| a),
                        "behind_main": t.ahead_behind.map(|(_, b)| b),
                        "upstream": t.upstream.as_ref().map(|u| json!({ "name": u.name, "ahead": u.ahead, "behind": u.behind, "gone": u.gone })),
                        "doing": t.doing.map(|d| d.word()),
                        "locked": t.wt.locked.is_some(),
                        "uncommitted_files": files,
                    })
                })
                .collect();
            json!({ "trunk": app.trees.trunk, "worktrees": rows })
        }
        Request::Conflicts => {
            let c = &app.clashes;
            let mut pairs = Vec::new();
            for i in 0..c.sides.len() {
                for j in i + 1..c.sides.len() {
                    let (kind, conflict) = match c.between(i, j) {
                        Some(Outcome::Committed(x)) => ("committed", x),
                        Some(Outcome::Uncommitted(x)) => ("uncommitted", x),
                        _ => continue,
                    };
                    pairs.push(json!({ "a": c.sides[i].name, "b": c.sides[j].name, "kind": kind, "conflict": conflict_json(conflict) }));
                }
            }
            let branches: Vec<Value> = c
                .branches
                .iter()
                .filter_map(|b| b.outcome.conflict().map(|x| json!({ "branch": b.name, "conflict": conflict_json(x) })))
                .collect();
            json!({
                "sides": c.sides.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
                "conflicting_pairs": pairs,
                "branches_against_trunk": branches,
            })
        }
    };
    ito_mcp::serde_json::to_string_pretty(&v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_tools_and_none_writes() {
        let out = server().handle(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
        let v: Value = ito_mcp::serde_json::from_str(&out).unwrap();
        let names: Vec<&str> = v["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["michi_worktrees", "michi_conflicts"]);
    }

    /// With no window behind the door, the tool says so as a tool error.
    #[test]
    fn no_window_is_a_tool_error() {
        let dir = std::env::temp_dir().join(format!("michi-mcp-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("MICHI_ADDRESS", dir.join("sock"));
        let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "michi_worktrees", "arguments": {} } });
        let out = server().handle(&line.to_string()).unwrap();
        std::env::remove_var("MICHI_ADDRESS");
        let v: Value = ito_mcp::serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["isError"], true);
        assert!(v["result"]["content"][0]["text"].as_str().unwrap().contains("michi is not running"));
    }
}
