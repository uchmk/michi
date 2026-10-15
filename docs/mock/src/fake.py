"""The made-up part of the michi mock: three Claude Code sessions in worktrees
on top of the real tsumugi history, their uncommitted changes, and the merge
that conflicts. Times are JST on 2026-10-15."""

DAY = 1791990000  # 2026-10-15 00:00 JST


def T(h, m, s=0):
    return DAY + h * 3600 + m * 60 + s


def lines(text):
    out = text.strip("\n").split("\n")
    return out


def counts(diffs):
    files = []
    for path, d in diffs.items():
        add = sum(1 for l in d if l.startswith("+"))
        dele = sum(1 for l in d if l.startswith("-"))
        files.append([path, add, dele])
    return files


GITINFO = "crates/tsumugi/src/gitinfo.rs"
CLI = "crates/tsumugi/src/cli.rs"
MAIN = "crates/tsumugi/src/main.rs"
WT = "crates/tsumugi/src/worktree.rs"
TESTS = "crates/tsumugi/tests/cli.rs"

R1814 = ("| 18.14 | Move the window to the right of a second screen (or just away from the top left), make it narrower, wait a second, "
         "close it, start `tsumugi`; then maximize it, close it, start it again; then unplug the second screen (or put a nonsense "
         "`x = 99999` in `window.toml` in the state folder) and start it | It opens where and as big as it was left; then maximized, "
         "and the restore button gives the narrower size back at the same place; with the screen gone it opens where the system puts it, "
         "on a screen. A window torn off a tab opens where it was dropped, not at the saved place; with `[window] quake` set the saved place is not used |")
ROW_NEW = ("| 18.15 | In a repository, `tsumugi new --worktree try/wt --from origin/main -- claude` | A folder `<repo>-try-wt` beside it on a "
           "new branch `try/wt` from origin/main, and a tab there with Claude Code started; closing its last session asks whether to remove the worktree |")
ROW_FOCUS = ("| 18.15 | With the window behind another app, `tsumugi focus 2` | The window comes forward with session 2's tab chosen; "
             "with no window open, one starts on it |")
LOG_NEW = [
    "- `tsumugi new --worktree BRANCH [--from REF]` で、リポジトリの横に worktree を作ってそこでセッションを始められるようにした。",
    "  作った worktree は tsumugi のものとして覚え、最後のセッションを閉じたときに消すかを聞く。",
]
LOG_FOCUS = ["- `tsumugi focus N|NAME` で、窓を前に出してそのセッションのタブを選べるようにした（michi の worktree の表から呼ぶ）。"]

DOC_BASE = [
    "//! `tsumugi new` and `tsumugi attach`: start a session from a script or an",
    "//! agent, and open the window on one (v1-scope, the multiplexer's CLI).",
]
DOC_NEW = [
    "//! `tsumugi new` and `tsumugi attach`: start a session from a script or an",
    "//! agent (in a new worktree with `--worktree`), and open the window on one",
    "//! (v1-scope, the multiplexer's CLI).",
]
DOC_FOCUS = [
    "//! `tsumugi new`, `tsumugi attach` and `tsumugi focus`: start a session from a",
    "//! script or an agent, open the window on one, or bring it forward (v1-scope,",
    "//! the multiplexer's CLI).",
]
TEST_NEW = [
    "",
    "    #[test]",
    "    fn new_takes_a_worktree_and_where_it_starts() {",
    '        let n = parse_new(&words("--worktree fix/x --from origin/main -- claude")).unwrap();',
    '        assert_eq!((n.worktree.as_deref(), n.from.as_deref()), (Some("fix/x"), Some("origin/main")));',
    '        assert!(parse_new(&words("--from main")).is_err(), "--from without --worktree");',
    "    }",
]
TEST_FOCUS = [
    "",
    "    #[test]",
    "    fn focus_takes_one_session() {",
    '        assert_eq!(parse_focus(&words("3")).as_deref(), Ok("3"));',
    '        assert_eq!(parse_focus(&words("review")).as_deref(), Ok("review"));',
    '        assert!(parse_focus(&words("")).is_err());',
    '        assert!(parse_focus(&words("1 2")).is_err(), "one session at a time");',
    "    }",
]
TESTS_END = [
    '        assert!(parse_wait(&words("2 --state soon")).is_err());',
    '        assert_eq!(last_lines("a\\nb\\nc\\n\\n\\n", 2), "b\\nc\\n");',
    "    }",
]


def pre(prefix, ls):
    return [prefix + l for l in ls]


c_fix = {
    GITINFO: lines(r'''
@@ -162,5 +162,8 @@ fn ahead_behind(cwd: &Path) -> Option<(usize, usize)> {
-/// The changes not committed in the repository `cwd` is in.
+/// The changes not committed in the repository `cwd` is in. Read only:
+/// `--no-optional-locks` keeps `git status` from refreshing the index, which
+/// takes `index.lock` and makes a commit running in the same worktree at that
+/// moment fail (#142).
 fn changes(cwd: &Path) -> Option<Changes> {
     let status = run(Command::new("git")
-        .args(["-c", "core.fsmonitor=false", "status", "--porcelain=v1", "--untracked-files=normal"])
+        .args(["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "--untracked-files=normal"])
         .current_dir(cwd))?;
'''),
}

c_new1 = {
    CLI: lines(r'''
@@ -1,4 +1,5 @@
 //! `tsumugi new` and `tsumugi attach`: start a session from a script or an
-//! agent, and open the window on one (v1-scope, the multiplexer's CLI).
+//! agent (in a new worktree with `--worktree`), and open the window on one
+//! (v1-scope, the multiplexer's CLI).

 use std::path::PathBuf;
@@ -18,9 +19,13 @@ pub struct New {
     /// Its place among the tabs (1 is the first), when it should not go last.
     pub at: Option<usize>,
+    /// A worktree to make beside the repository and start in (`--worktree`).
+    pub worktree: Option<String>,
+    /// What the worktree's new branch starts from (`--from`; HEAD when not given).
+    pub from: Option<String>,
     /// The command typed into the new shell (`claude`), from after `--`:
     /// its words, quoted there for the shell (one word is a line).
     pub command: Vec<String>,
 }

-pub const NEW_USAGE: &str = "tsumugi new [FOLDER] [--tag TAG]... [--name NAME] [--at N] [-- COMMAND ARGS...]";
+pub const NEW_USAGE: &str =
+    "tsumugi new [FOLDER] [--tag TAG]... [--name NAME] [--at N] [--worktree BRANCH [--from REF]] [-- COMMAND ARGS...]";

 pub fn parse_new(args: &[String]) -> Result<New, String> {
@@ -44,10 +50,21 @@ pub fn parse_new(args: &[String]) -> Result<New, String> {
                 None => return Err("--at takes a place, 1 for the first tab".into()),
             },
+            "--worktree" | "-w" => match it.next() {
+                Some(b) => out.worktree = Some(b.clone()),
+                None => return Err("--worktree takes a branch".into()),
+            },
+            "--from" => match it.next() {
+                Some(r) => out.from = Some(r.clone()),
+                None => return Err("--from takes a commit, a branch or a tag".into()),
+            },
             s if s.starts_with('-') => return Err(format!("unknown option `{s}`")),
             _ if out.folder.is_none() => out.folder = Some(PathBuf::from(a)),
             _ => return Err(format!("one folder only (`{a}` is a second); a command goes after --")),
         }
     }
+    if out.from.is_some() && out.worktree.is_none() {
+        return Err("--from goes with --worktree".into());
+    }
     Ok(out)
 }
'''),
}

c_focus1 = {
    CLI: lines(r'''
@@ -1,2 +1,3 @@
-//! `tsumugi new` and `tsumugi attach`: start a session from a script or an
-//! agent, and open the window on one (v1-scope, the multiplexer's CLI).
+//! `tsumugi new`, `tsumugi attach` and `tsumugi focus`: start a session from a
+//! script or an agent, open the window on one, or bring it forward (v1-scope,
+//! the multiplexer's CLI).

@@ -295,4 +296,5 @@ tsumugi read N|NAME [--lines K] [--all]  its last K lines (40), or its whole scrollback
 tsumugi close N|NAME                     end the session
+tsumugi focus N|NAME                     bring the window forward with its tab chosen
 tsumugi log N|NAME [PATH] [--stop]       write its text to PATH (Downloads) as it goes, or finish;
                                          prints the file (`-2` when PATH is there already)
@@ -360,2 +362,13 @@ pub fn parse_wait(args: &[String]) -> Result<Wait, String> {

+/// `tsumugi focus N|NAME`: the one session to bring forward.
+pub fn parse_focus(args: &[String]) -> Result<String, String> {
+    match args {
+        [one] => Ok(one.clone()),
+        [] => Err("focus takes a session, N or NAME".into()),
+        _ => Err("focus takes one session".into()),
+    }
+}
+
 #[cfg(test)]
 mod tests {
@@ -627,5 +640,13 @@ mod tests {
         assert!(parse_wait(&words("2 --state soon")).is_err());
         assert_eq!(last_lines("a\nb\nc\n\n\n", 2), "b\nc\n");
     }
+
+    #[test]
+    fn focus_takes_one_session() {
+        assert_eq!(parse_focus(&words("3")).as_deref(), Ok("3"));
+        assert_eq!(parse_focus(&words("review")).as_deref(), Ok("review"));
+        assert!(parse_focus(&words("")).is_err());
+        assert!(parse_focus(&words("1 2")).is_err(), "one session at a time");
+    }
 }
'''),
    MAIN: lines(r'''
@@ -473,9 +473,16 @@ fn remote(what: &str, args: &[String]) -> std::process::ExitCode {
         "close" => {
             client.attach(id).kill();
             match client.list() {
                 Ok(_) => ExitCode::SUCCESS,
                 Err(e) => fail(e.to_string()),
             }
         }
+        // The window brings itself forward when it hears its session was
+        // asked for; with no window open, one is started on the session.
+        "focus" => match client.focus(id) {
+            Ok(true) => ExitCode::SUCCESS,
+            Ok(false) => open_window_on(id),
+            Err(e) => fail(e.to_string()),
+        },
         _ => {
             let w = match cli::parse_wait(args) {
'''),
}

c_new2 = {
    WT: lines(r'''
@@ -45,12 +45,12 @@ fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
 }

-/// Make a worktree of the repository `folder` is in, on `branch` (made from
-/// the current commit when it does not exist yet); its folder.
-pub fn add(folder: &Path, branch: &str) -> Result<PathBuf, String> {
-    let path = make(folder, branch)?;
+/// Make a worktree of the repository `folder` is in, on `branch` (made from
+/// `from`, or the current commit, when it does not exist yet); its folder.
+pub fn add(folder: &Path, branch: &str, from: Option<&str>) -> Result<PathBuf, String> {
+    let path = make(folder, branch, from)?;
     remember(&path, true);
     Ok(path)
 }

 /// [`add`] without the list.
-fn make(folder: &Path, branch: &str) -> Result<PathBuf, String> {
+fn make(folder: &Path, branch: &str, from: Option<&str>) -> Result<PathBuf, String> {
@@ -61,5 +61,9 @@ fn make(folder: &Path, branch: &str) -> Result<PathBuf, String> {
     let target = path.to_string_lossy().into_owned();
     let exists = git(&root, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
-    let made =
-        if exists { git(&root, &["worktree", "add", &target, branch]) } else { git(&root, &["worktree", "add", "-b", branch, &target]) };
+    let made = match (exists, from) {
+        (true, Some(_)) => return Err(format!("{branch} is there already; --from is for a new branch")),
+        (true, None) => git(&root, &["worktree", "add", &target, branch]),
+        (false, from) => git(&root, &["worktree", "add", "-b", branch, &target, from.unwrap_or("HEAD")]),
+    };
     made.map_err(|e| format!("git worktree add: {e}"))?;
'''),
    MAIN: lines(r'''
@@ -2367,5 +2367,5 @@ fn start_in_worktree(&mut self, folder: PathBuf, branch: String) {
             std::thread::spawn(move || {
-                let _ = tx.send(worktree::add(&folder, &branch));
+                let _ = tx.send(worktree::add(&folder, &branch, None));
             });
@@ -4390,5 +4390,5 @@ fn start_parallel(&mut self, start: ParallelStart) {
                             let task = start.task.clone();
-                            let made = worktree::add(&start.folder, &parallel::branch(&stamp, k)).map(|path| (path, task));
+                            let made = worktree::add(&start.folder, &parallel::branch(&stamp, k), None).map(|path| (path, task));
                             tx.send(made)
'''),
    CLI: lines(r'''
@@ -640,5 +640,12 @@ mod tests {
         assert!(parse_wait(&words("2 --state soon")).is_err());
         assert_eq!(last_lines("a\nb\nc\n\n\n", 2), "b\nc\n");
     }
+
+    #[test]
+    fn new_takes_a_worktree_and_where_it_starts() {
+        let n = parse_new(&words("--worktree fix/x --from origin/main -- claude")).unwrap();
+        assert_eq!((n.worktree.as_deref(), n.from.as_deref()), (Some("fix/x"), Some("origin/main")));
+        assert!(parse_new(&words("--from main")).is_err(), "--from without --worktree");
+    }
 }
'''),
    "TESTING.md": [
        "@@ -559,3 +559,4 @@ ## 18. From a script",
        " " + R1814,
        "+" + ROW_NEW,
        " ",
        " ## 19. Keys through every screen",
    ],
}

c_focus2 = {
    "TESTING.md": [
        "@@ -559,3 +559,4 @@ ## 18. From a script",
        " " + R1814,
        "+" + ROW_FOCUS,
        " ",
        " ## 19. Keys through every screen",
    ],
    "CHANGELOG.md": [
        "@@ -6,4 +6,10 @@",
        " ## [未リリース]",
        " ",
        "+## [0.110.0] - 2026-10-15",
        "+",
        "+### 追加",
        "+",
        *pre("+", LOG_FOCUS),
        "+",
        " ## [0.109.6] - 2026-10-15",
        " ",
    ],
    "Cargo.toml": [
        "@@ -5,5 +5,5 @@ members = [\"crates/*\"]",
        " # One version for the workspace: it is what CHANGELOG.md and the tags follow.",
        " [workspace.package]",
        '-version = "0.109.6"',
        '+version = "0.110.0"',
        ' edition = "2021"',
        ' rust-version = "1.95"',
    ],
}

c_new3 = {
    TESTS: lines(r'''
@@ -708,3 +708,22 @@ fn env_reports_the_places_and_the_server() {
     let hidden = String::from_utf8(hidden.stdout).unwrap();
     assert!(hidden.contains("TSUMUGI_API_TOKEN=(hidden)") && !hidden.contains("hunter2"), "{hidden}");
 }
+
+#[test]
+fn new_worktree_makes_the_folder_beside_the_repository() {
+    let s = Server::new("worktree");
+    let repo = s.dir.join("repo");
+    std::fs::create_dir_all(&repo).unwrap();
+    let git = |args: &[&str]| assert!(Command::new("git").args(args).current_dir(&repo).status().unwrap().success(), "git {args:?}");
+    git(&["init", "-q", "-b", "main"]);
+    git(&["-c", "user.email=a@b", "-c", "user.name=t", "commit", "-q", "--allow-empty", "-m", "first"]);
+
+    let out = s.ok(&["new", repo.to_str().unwrap(), "--worktree", "try/wt"]);
+    let n: usize = out.trim().parse().unwrap_or_else(|_| panic!("a session number, not {out:?}"));
+    let made = s.dir.join("repo-try-wt");
+    assert!(made.join(".git").exists(), "{} was not made", made.display());
+    let ls = s.ok(&["ls", "--json"]);
+    assert!(ls.contains("repo-try-wt"), "the session starts in the worktree: {ls}");
+    s.ok(&["close", &n.to_string()]);
+}
'''),
    "CHANGELOG.md": [
        "@@ -6,4 +6,11 @@",
        " ## [未リリース]",
        " ",
        "+## [0.110.0] - 2026-10-15",
        "+",
        "+### 追加",
        "+",
        *pre("+", LOG_NEW),
        "+",
        " ## [0.109.6] - 2026-10-15",
        " ",
    ],
    "Cargo.toml": [
        "@@ -5,5 +5,5 @@ members = [\"crates/*\"]",
        " # One version for the workspace: it is what CHANGELOG.md and the tags follow.",
        " [workspace.package]",
        '-version = "0.109.6"',
        '+version = "0.110.0"',
        ' edition = "2021"',
        ' rust-version = "1.95"',
    ],
}

w_fix = {
    GITINFO: lines(r'''
@@ -170,4 +170,4 @@ fn changes(cwd: &Path) -> Option<Changes> {
     let numstat = run(Command::new("git")
-        .args(["-c", "core.fsmonitor=false", "diff", "--numstat", "--no-ext-diff", "--no-textconv", "HEAD"])
+        .args(["--no-optional-locks", "-c", "core.fsmonitor=false", "diff", "--numstat", "--no-ext-diff", "--no-textconv", "HEAD"])
         .current_dir(cwd))?;
'''),
    "CHANGELOG.md": [
        "@@ -6,4 +6,11 @@",
        " ## [未リリース]",
        " ",
        "+## [0.109.7] - 2026-10-15",
        "+",
        "+### 修正",
        "+",
        "+- Claude Code が同じ worktree でコミットしている最中に、窓の状態の読み取り（`git status`）が `index.lock` を取って",
        "+  コミットを失敗させることがあったのを直した（#142。`--no-optional-locks` で読むだけにする）。",
        "+",
        " ## [0.109.6] - 2026-10-15",
        " ",
    ],
    "Cargo.toml": [
        "@@ -5,5 +5,5 @@ members = [\"crates/*\"]",
        " # One version for the workspace: it is what CHANGELOG.md and the tags follow.",
        " [workspace.package]",
        '-version = "0.109.6"',
        '+version = "0.109.7"',
        ' edition = "2021"',
        ' rust-version = "1.95"',
    ],
}

w_new = {
    MAIN: lines(r'''
@@ -197,5 +197,13 @@ fn new_cli(args: &[String]) -> std::process::ExitCode {
             return std::process::ExitCode::from(2);
         }
     };
+    // The worktree first, so the session starts in it and `ls` shows its folder.
+    let n = match n.worktree.clone() {
+        Some(branch) => match worktree::add(&n.folder_or_here(), &branch, n.from.as_deref()) {
+            Ok(path) => cli::New { folder: Some(path), ..n },
+            Err(e) => return fail(format!("tsumugi new: {e}")),
+        },
+        None => n,
+    };
     match connect(|| {}).and_then(|c| cli::new(&c, n)) {
         Ok(id) => {
'''),
}


def commit(h, p, t, s, b, refs, diffs, a="yuu06"):
    return {"h": h, "p": p, "a": a, "t": t, "s": s, "b": b, "r": refs, "f": counts(diffs), "d": diffs}


COMMITS = [
    commit("71c4a2f", ["5a06f3d"], T(10, 9, 12), "tests/cli.rs: new --worktree prints the session number",
           "A test against a server of its own: a repository with one commit, `new --worktree try/wt`, and the folder `repo-try-wt` "
           "beside it; `ls --json` shows the session there. Row 18.15 stays for the asking when the last session closes.",
           ["feat/new-worktree"], c_new3),
    commit("e9d18b4", ["c27b5e9"], T(10, 6, 40), "TESTING: row 18.15 for tsumugi focus",
           "", ["feat/focus", "origin/feat/focus"], c_focus2),
    commit("5a06f3d", ["8d41e07"], T(10, 3, 5), "Create the worktree beside the repo and remember it as tsumugi's",
           "worktree::add takes where the new branch starts; the window's own \"new tab in a worktree\" passes None and keeps HEAD.",
           [], c_new2),
    commit("c27b5e9", ["bb3e879"], T(9, 58, 31), "Add tsumugi focus N|NAME",
           "Brings the window forward with the session's tab chosen, and starts the window on it when none is open. "
           "michi's worktree table calls it for 「タブへ」.",
           [], c_focus1),
    commit("8d41e07", ["bb3e879"], T(9, 55, 2), "tsumugi new takes --worktree BRANCH and --from REF",
           "The parsing only; the worktree is made in the next commit. --from without --worktree is refused.",
           [], c_new1),
    commit("3f9c2a1", ["bb3e879"], T(9, 52, 47), "Status polling runs git with --no-optional-locks (#142)",
           "The window asks git for the state of every session's folder every few seconds. `git status` refreshes the index and "
           "takes index.lock to do it, so a Claude Code committing in the same worktree at that moment failed with "
           "\"Unable to create index.lock\".",
           ["fix/142-optional-locks"], c_fix),
]

WIP = [
    {"wt": "fix/142-optional-locks", "t": T(10, 11, 30), "f": counts(w_fix), "d": w_fix},
    {"wt": "feat/new-worktree", "t": T(10, 10, 4), "f": counts(w_new), "d": w_new},
]

WORKTREES = [
    {"branch": "main", "path": r"C:\dev\tsumugi", "head": "bb3e879", "main": True, "ahead": 0, "behind": 0,
     "claude": None, "unseen": None},
    {"branch": "fix/142-optional-locks", "path": r"C:\dev\tsumugi-fix-142-optional-locks", "head": "3f9c2a1", "ahead": 1, "behind": 0,
     "claude": {"state": "run", "since": T(10, 8), "tab": 5, "title": "fix #142 index.lock",
                "line": "CHANGELOG.md に 0.109.7 の項目を足しています…"},
     "unseen": {"commits": 1, "files": 1}},
    {"branch": "feat/new-worktree", "path": r"C:\dev\tsumugi-feat-new-worktree", "head": "71c4a2f", "ahead": 3, "behind": 0,
     "claude": {"state": "wait", "since": T(10, 10), "tab": 6, "title": "new --worktree",
                "line": "Bash(cargo test -p tsumugi --test cli) を許可しますか？"},
     "unseen": {"commits": 2, "files": 7}},
    {"branch": "feat/focus", "path": r"C:\dev\tsumugi-feat-focus", "head": "e9d18b4", "ahead": 2, "behind": 0, "pushed": True,
     "claude": {"state": "done", "since": T(10, 6), "tab": 7, "title": "tsumugi focus",
                "line": "18.15 を足して 0.110.0 にしました。PR を作りますか？"},
     "unseen": {"commits": 0, "files": 0}},
]

PREDICT = {
    "feat/new-worktree|feat/focus": [["crates/tsumugi/src/cli.rs", 2], ["CHANGELOG.md", 1], ["TESTING.md", 1]],
}

STASH = ["stash@{0}: WIP on main: e21e189 try ito 0.34.3 locally"]

# The conflict scene: feat/new-worktree was merged into main (b4e7d10); now
# feat/focus is being merged and stops on four places.
MERGED = commit("b4e7d10", ["bb3e879", "71c4a2f"], T(10, 18, 20), "Merge branch 'feat/new-worktree'", "",
                ["HEAD -> main"], {
                    CLI: [], WT: [], TESTS: [], MAIN: [], "CHANGELOG.md": [], "TESTING.md": [], "Cargo.toml": []})
MERGED["f"] = [[CLI, 22, 2], [WT, 9, 6], [TESTS, 19, 0], [MAIN, 2, 2], ["CHANGELOG.md", 7, 0], ["TESTING.md", 1, 0], ["Cargo.toml", 1, 1]]
del MERGED["d"]

SIDE = {
    "ours": {"label": "自分", "ref": "main", "head": "b4e7d10"},
    "theirs": {"label": "相手", "ref": "feat/focus", "head": "e9d18b4"},
}

CONFLICT = {
    "into": "main", "from": "feat/focus", "head": "b4e7d10", "theirs": "e9d18b4", "base": "bb3e879",
    "started": T(10, 19, 2), "rerere_at": "10:05",
    "files": [
        {"path": CLI, "blocks": [0, 1]},
        {"path": "CHANGELOG.md", "blocks": [2]},
        {"path": "TESTING.md", "blocks": [3]},
    ],
    "clean": [["Cargo.toml", "両方が同じ版（0.110.0）に上げたので 1 つに"], [MAIN, "別の所の変更だけ"]],
    "blocks": [
        {"file": CLI, "line": 1, "before": [], "before_skip": 0,
         "ours": DOC_NEW, "base": DOC_BASE, "theirs": DOC_FOCUS,
         "after": ["", "use std::path::PathBuf;", "", "use ito_pane::Size;"], "after_skip": 633,
         "why_ours": ["8d41e07", "tsumugi new takes --worktree BRANCH and --from REF", "feat/new-worktree", "09:55"],
         "why_theirs": ["c27b5e9", "Add tsumugi focus N|NAME", "feat/focus", "09:58"],
         "hint": "両方が同じ 2 行（モジュールの説明）を書き換えています。片方を採るともう片方の説明が消えるので、合わせて手で直してください。",
         "state": "open"},
        {"file": CLI, "line": 641, "before": TESTS_END, "before_skip": 633,
         "ours": TEST_NEW, "base": [], "theirs": TEST_FOCUS,
         "after": ["}"], "after_skip": 0,
         "why_ours": ["5a06f3d", "Create the worktree beside the repo and remember it as tsumugi's", "feat/new-worktree", "10:03"],
         "why_theirs": ["c27b5e9", "Add tsumugi focus N|NAME", "feat/focus", "09:58"],
         "hint": "両方がテストの終わりに 1 つずつ足しています。",
         "state": "rerere", "result": TEST_NEW + TEST_FOCUS},
        {"file": "CHANGELOG.md", "line": 12, "before": ["## [未リリース]", "", "## [0.110.0] - 2026-10-15", "", "### 追加", ""],
         "before_skip": 5,
         "ours": LOG_NEW, "base": [], "theirs": LOG_FOCUS,
         "after": ["", "## [0.109.6] - 2026-10-15"], "after_skip": 1204,
         "why_ours": ["71c4a2f", "tests/cli.rs: new --worktree prints the session number", "feat/new-worktree", "10:09"],
         "why_theirs": ["e9d18b4", "TESTING: row 18.15 for tsumugi focus", "feat/focus", "10:06"],
         "hint": "base は空で、両方が同じ所に 1 項目ずつ足しています（見出しの「## [0.110.0]」は同じなので外に出しました）。「両方」でよさそうです。",
         "state": "open"},
        {"file": "TESTING.md", "line": 561, "before": [R1814], "before_skip": 559,
         "ours": [ROW_NEW], "base": [], "theirs": [ROW_FOCUS],
         "after": ["", "## 19. Keys through every screen"], "after_skip": 412,
         "why_ours": ["5a06f3d", "Create the worktree beside the repo and remember it as tsumugi's", "feat/new-worktree", "10:03"],
         "why_theirs": ["e9d18b4", "TESTING: row 18.15 for tsumugi focus", "feat/focus", "10:06"],
         "hint": "両方が行 18.15 を足しています。両方を採るなら、相手の行を 18.16 にします。",
         "renumber": ["| 18.15 |", "| 18.16 |"],
         "state": "open"},
    ],
}

FAKE = {
    "day": DAY, "now": T(10, 12), "now_merge": T(10, 21),
    "commits": COMMITS, "wip": WIP, "worktrees": WORKTREES, "predict": PREDICT, "stash": STASH,
    "merged": MERGED, "conflict": CONFLICT, "side": SIDE,
}
