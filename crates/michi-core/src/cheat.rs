//! The Git flow sheet: for each thing a person wants to do, a small flowchart
//! (nodes laid out by hand on a grid, edges between them) and the commands of
//! its nodes. The words are in English and Japanese beside the data, so a
//! sheet is one place to read and test; the drawing is the `michi` crate's.
//!
//! Every [`Step`] has a `key` slot for the michi key that does the same thing
//! (a keymap `run` word or a key spelled for a reader). It is empty until the
//! keys are sorted out; [`Flow::steps_of`] and the tests read it as it is.

/// A text in both languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct T {
    pub en: &'static str,
    pub ja: &'static str,
}

impl T {
    pub fn get(&self, lang: &str) -> &'static str {
        if lang == "ja" {
            self.ja
        } else {
            self.en
        }
    }
}

const fn t(en: &'static str, ja: &'static str) -> T {
    T { en, ja }
}

/// What a node is, which decides how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Start,
    Action,
    Decision,
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub id: u8,
    pub kind: Kind,
    /// Column and row on the grid.
    pub at: (u8, u8),
    pub label: T,
}

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: u8,
    pub to: u8,
    pub label: Option<T>,
}

/// A command that belongs to a node.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub node: u8,
    pub command: &'static str,
    pub why: T,
    /// Throws work away or rewrites shared history: shown in a warning colour.
    pub danger: bool,
    /// The michi screen where the same is done (graph, changes, worktree, conflicts), when there is one.
    pub screen: Option<&'static str>,
    /// The michi key that does the same, when there is one.
    pub key: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    Basics,
    Conflict,
    Undo,
    Reset,
    RestoreSwitch,
    Place,
    Rescue,
}

impl Topic {
    pub const ALL: [Topic; 7] =
        [Topic::Basics, Topic::Conflict, Topic::Undo, Topic::Reset, Topic::RestoreSwitch, Topic::Place, Topic::Rescue];

    /// The short name on the tab.
    pub fn name(self) -> T {
        match self {
            Topic::Basics => t("Getting Started", "基本操作"),
            Topic::Conflict => t("Resolving Conflicts", "コンフリクト解決"),
            Topic::Undo => t("Undoing Changes", "変更の取り消し"),
            Topic::Reset => t("Reset & Discard", "リセットと破棄"),
            Topic::RestoreSwitch => t("Files & Branches", "ファイルとブランチ"),
            Topic::Place => t("Parallel Work", "並行作業"),
            Topic::Rescue => t("Troubleshooting", "トラブルシューティング"),
        }
    }

    pub fn flow(self) -> &'static Flow {
        match self {
            Topic::Basics => &BASICS,
            Topic::Conflict => &CONFLICT,
            Topic::Undo => &UNDO,
            Topic::Reset => &RESET,
            Topic::RestoreSwitch => &RESTORE,
            Topic::Place => &PLACE,
            Topic::Rescue => &RESCUE,
        }
    }
}

/// One sheet.
#[derive(Debug)]
pub struct Flow {
    pub topic: Topic,
    pub title: T,
    pub nodes: &'static [Node],
    pub edges: &'static [Edge],
    pub steps: &'static [Step],
    /// A comparison table, the first row being the heads (may be empty).
    pub table: &'static [&'static [T]],
    pub note: Option<T>,
}

impl Flow {
    pub fn node(&self, id: u8) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The steps of one node, in order.
    pub fn steps_of(&self, id: u8) -> impl Iterator<Item = &Step> {
        self.steps.iter().filter(move |s| s.node == id)
    }

    /// The nodes in reading order (row, then column), which is the order `j` and `k` walk.
    pub fn walk(&self) -> Vec<u8> {
        let mut v: Vec<&Node> = self.nodes.iter().collect();
        v.sort_by_key(|n| (n.at.1, n.at.0));
        v.into_iter().map(|n| n.id).collect()
    }

    /// Columns and rows the grid needs.
    pub fn size(&self) -> (u8, u8) {
        let c = self.nodes.iter().map(|n| n.at.0).max().unwrap_or(0) + 1;
        let r = self.nodes.iter().map(|n| n.at.1).max().unwrap_or(0) + 1;
        (c, r)
    }
}

const fn node(id: u8, kind: Kind, col: u8, row: u8, label: T) -> Node {
    Node { id, kind, at: (col, row), label }
}

const fn edge(from: u8, to: u8) -> Edge {
    Edge { from, to, label: None }
}

const fn edge_l(from: u8, to: u8, label: T) -> Edge {
    Edge { from, to, label: Some(label) }
}

const fn step(node: u8, command: &'static str, why: T) -> Step {
    Step { node, command, why, danger: false, screen: None, key: None }
}

const fn risky(node: u8, command: &'static str, why: T) -> Step {
    Step { node, command, why, danger: true, screen: None, key: None }
}

use Kind::{Action, Decision, Start};

// ---------------------------------------------------------------- 1. basics

static BASICS: Flow = Flow {
    topic: Topic::Basics,
    title: t("Basics: create, edit, commit, push, PR, merge", "基本: 作る・直す・コミット・push・PR・マージ"),
    nodes: &[
        node(0, Start, 1, 0, t("Start working", "作業を始める")),
        node(1, Decision, 1, 1, t("Where does the repository begin?", "リポジトリはどこから？")),
        node(2, Action, 0, 2, t("Made on GitHub: clone", "GitHub で作った: clone")),
        node(3, Action, 1, 2, t("Local folder: init", "手元のフォルダ: init")),
        node(4, Action, 2, 2, t("Already cloned: new branch", "clone 済み: ブランチを切る")),
        node(5, Action, 1, 3, t("Edit files", "ファイルを直す")),
        node(6, Action, 1, 4, t("Stage and commit", "add して commit")),
        node(7, Action, 1, 5, t("Push the branch", "ブランチを push")),
        node(8, Action, 1, 6, t("Open a PR", "PR を作る")),
        node(9, Decision, 1, 7, t("Review passed?", "レビューは通った？")),
        node(10, Action, 1, 8, t("Merge the PR", "PR をマージ")),
        node(11, Action, 1, 9, t("Bring main up to date", "main を最新にする")),
    ],
    edges: &[
        edge(0, 1),
        edge_l(1, 2, t("GitHub first", "GitHub が先")),
        edge_l(1, 3, t("local first", "手元が先")),
        edge_l(1, 4, t("clone exists", "clone がある")),
        edge(2, 5),
        edge(3, 5),
        edge(4, 5),
        edge(5, 6),
        edge(6, 7),
        edge(7, 8),
        edge(8, 9),
        edge_l(9, 10, t("yes", "はい")),
        edge_l(9, 5, t("no: fix", "いいえ: 直す")),
        edge(10, 11),
    ],
    steps: &[
        step(2, "git clone <url>", t("Copy the remote repository here", "リモートのリポジトリを手元に写す")),
        step(2, "git clone <url> <folder>", t("Choose the folder name", "フォルダー名を指定する")),
        step(3, "git init -b main", t("Make a repository in the current folder", "今のフォルダーをリポジトリにする")),
        step(3, "git remote add origin <url>", t("Tell it where the remote is", "リモートの場所を登録する")),
        step(3, "git push -u origin main", t("First push; -u remembers the pair", "最初の push。-u で組を覚える")),
        step(4, "git switch main && git pull", t("Start from the latest main", "最新の main から始める")),
        step(4, "git switch -c feature/<name>", t("Work on a branch of your own", "自分のブランチで作業する")),
        step(5, "git status", t("What changed", "何が変わったか")),
        step(5, "git diff", t("Read the changes before staging", "add の前に変更を読む")),
        step(6, "git add <file>", t("Put the file in the next commit", "次のコミットに入れる")),
        step(6, "git add -p", t("Choose hunks one by one", "変更を塊ごとに選ぶ")),
        step(6, "git commit -m \"message\"", t("Make the commit", "コミットを作る")),
        step(7, "git push -u origin HEAD", t("First push of the branch", "ブランチの最初の push")),
        step(7, "git push", t("Later pushes", "2 回目以降の push")),
        step(8, "gh pr create --fill", t("Open a pull request (or use the browser)", "PR を作る（ブラウザでもよい）")),
        step(9, "gh pr view --web", t("Read the review", "レビューを見る")),
        step(10, "gh pr merge --merge", t("Merge with a merge commit", "merge コミットでマージする")),
        step(11, "git switch main && git pull", t("Take in the merge", "マージを取り込む")),
        step(11, "git branch -d feature/<name>", t("Delete the finished branch", "終わったブランチを消す")),
    ],
    table: &[],
    note: Some(t(
        "The three starts (GitHub first, local first, clone exists) join at \"Edit files\".",
        "3 つの入口（GitHub が先・手元が先・clone 済み）は「ファイルを直す」で合流します。",
    )),
};

// -------------------------------------------------------------- 2. conflict

static CONFLICT: Flow = Flow {
    topic: Topic::Conflict,
    title: t("Conflict: resolve or give up", "コンフリクト: 解消するか、やめるか"),
    nodes: &[
        node(0, Start, 1, 0, t("merge / rebase / pull stops with CONFLICT", "merge・rebase・pull が CONFLICT で止まる")),
        node(1, Action, 1, 1, t("Find the files in conflict", "衝突したファイルを探す")),
        node(2, Decision, 1, 2, t("Resolve it or give up?", "解消する？やめる？")),
        node(3, Action, 0, 3, t("Abort: back to before", "中止して元に戻す")),
        node(4, Action, 2, 3, t("Edit the file, keep what you want", "ファイルを直して残す方を決める")),
        node(5, Action, 2, 4, t("Mark it resolved", "解消済みの印をつける")),
        node(6, Decision, 2, 5, t("What was running?", "何の途中だった？")),
        node(7, Action, 1, 6, t("merge", "merge")),
        node(8, Action, 2, 6, t("rebase", "rebase")),
        node(9, Action, 3, 6, t("cherry-pick / revert / stash pop", "cherry-pick・revert・stash pop")),
    ],
    edges: &[
        edge(0, 1),
        edge(1, 2),
        edge_l(2, 3, t("give up", "やめる")),
        edge_l(2, 4, t("resolve", "解消する")),
        edge(4, 5),
        edge(5, 6),
        edge(6, 7),
        edge(6, 8),
        edge(6, 9),
    ],
    steps: &[
        step(1, "git status", t("Lists \"both modified\" files", "both modified のファイルが並ぶ")),
        step(1, "git diff --name-only --diff-filter=U", t("Only the unmerged files", "未マージのファイルだけ")),
        step(3, "git merge --abort", t("Stop a merge", "merge をやめる")),
        step(3, "git rebase --abort", t("Stop a rebase", "rebase をやめる")),
        step(3, "git cherry-pick --abort", t("Stop a cherry-pick", "cherry-pick をやめる")),
        step(
            4,
            "git restore --ours -- <file>",
            t("Take our side whole (ours = the branch you are on)", "自分側をそのまま取る（ours = 今いるブランチ）"),
        ),
        step(
            4,
            "git restore --theirs -- <file>",
            t("Take their side whole (during a rebase the two swap)", "相手側をそのまま取る（rebase 中は ours/theirs が逆）"),
        ),
        step(
            4,
            "git config rerere.enabled true",
            t("Remember resolutions and reuse them next time", "解消のしかたを覚えさせ、次から自動で使う"),
        ),
        step(5, "git add <file>", t("Say this file is resolved", "このファイルは解消したと伝える")),
        step(7, "git merge --continue", t("Finish the merge commit", "merge コミットを作って終える")),
        step(8, "git rebase --continue", t("Go on to the next commit", "次のコミットへ進む")),
        step(9, "git cherry-pick --continue", t("Finish the cherry-pick", "cherry-pick を終える")),
        step(
            9,
            "git stash drop",
            t(
                "After `stash pop` conflicted the stash is kept: drop it once resolved",
                "stash pop が衝突すると stash は残る。解消後に捨てる",
            ),
        ),
    ],
    table: &[],
    note: Some(t(
        "The marks in the file are <<<<<<< (ours), ======= and >>>>>>> (theirs). Remove the marks, keep the lines you want.",
        "ファイルの中の目印は <<<<<<<（ours）・=======・>>>>>>>（theirs）。目印を消して、残したい行だけにします。",
    )),
};

// ------------------------------------------------------------------ 3. undo

static UNDO: Flow = Flow {
    topic: Topic::Undo,
    title: t("Undo: pick by where the change is now", "戻す: 変更が今どこにあるかで選ぶ"),
    nodes: &[
        node(0, Start, 2, 0, t("I want to undo something", "何かを取り消したい")),
        node(1, Decision, 2, 1, t("Where is the change now?", "その変更は今どこ？")),
        node(2, Action, 0, 2, t("Edited, not added", "直しただけ（add 前）")),
        node(3, Action, 1, 2, t("Added (staged)", "add 済み")),
        node(4, Action, 2, 2, t("Committed, not pushed", "commit 済み・未 push")),
        node(5, Action, 3, 2, t("Already pushed", "push 済み")),
        node(6, Action, 4, 2, t("Lost commits or a branch", "コミットやブランチが消えた")),
    ],
    edges: &[
        edge(0, 1),
        edge_l(1, 2, t("working tree", "ワーキングツリー")),
        edge_l(1, 3, t("index", "ステージ")),
        edge_l(1, 4, t("local history", "手元の履歴")),
        edge_l(1, 5, t("shared history", "共有の履歴")),
        edge_l(1, 6, t("gone", "見えない")),
    ],
    steps: &[
        risky(2, "git restore <file>", t("Throw the edits of the file away (cannot come back)", "そのファイルの変更を捨てる（戻せない）")),
        step(2, "git stash push -m \"memo\"", t("Or keep them aside instead of throwing away", "捨てずに脇に置いておく")),
        step(
            3,
            "git restore --staged <file>",
            t("Take it out of the next commit, keep the edit", "次のコミットから外す。直した内容は残る"),
        ),
        step(4, "git commit --amend", t("Rewrite the last commit (message or files)", "直前のコミットを作り直す（メッセージやファイル）")),
        step(4, "git reset --soft HEAD~1", t("Undo the commit, keep it staged", "コミットだけ取り消す。add 済みのまま")),
        step(4, "git reset HEAD~1", t("Undo the commit, keep the edits unstaged (--mixed)", "コミットと add を取り消す。ファイルは残る")),
        risky(4, "git reset --hard HEAD~1", t("Undo the commit and throw the edits away", "コミットも変更も消す")),
        step(5, "git revert <commit>", t("Add a commit that cancels it; history stays", "打ち消すコミットを足す。履歴は書き換えない")),
        step(5, "git revert -m 1 <merge>", t("Cancel a merge commit (keep the first parent)", "merge コミットを打ち消す（第 1 親を残す）")),
        risky(
            5,
            "git push --force-with-lease",
            t("Only for your own branch after a reset; never plain --force", "自分のブランチを reset したあとだけ。--force 単体は使わない"),
        ),
        step(6, "git reflog", t("Every place HEAD was, newest first", "HEAD がいた場所の記録（新しい順）")),
        step(6, "git branch rescue <hash>", t("Put a branch on the lost commit", "消えたコミットにブランチを付けて救う")),
    ],
    table: &[],
    note: Some(t(
        "Rule of thumb: not pushed yet → reset/amend; already pushed → revert.",
        "目安: まだ push していない → reset・amend、push 済み → revert。",
    )),
};

// ----------------------------------------------------------------- 4. reset

static RESET: Flow = Flow {
    topic: Topic::Reset,
    title: t("reset: three modes", "reset: 3 つのモード"),
    nodes: &[
        node(0, Start, 1, 0, t("git reset <mode> <commit>", "git reset <モード> <コミット>")),
        node(1, Decision, 1, 1, t("What happens to the changes?", "その後の変更をどうする？")),
        node(2, Action, 0, 2, t("--soft", "--soft")),
        node(3, Action, 1, 2, t("--mixed (default)", "--mixed（指定なし）")),
        node(4, Action, 2, 2, t("--hard", "--hard")),
        node(5, Action, 2, 3, t("Went wrong?", "やりすぎた？")),
    ],
    edges: &[
        edge(0, 1),
        edge_l(1, 2, t("keep staged", "add 済みのまま")),
        edge_l(1, 3, t("keep files, unstage", "ファイルは残す")),
        edge_l(1, 4, t("throw away", "全部消す")),
        edge(4, 5),
    ],
    steps: &[
        step(0, "git reset HEAD~1", t("One commit back (HEAD~2 for two, or a hash)", "1 つ前へ（2 つ前は HEAD~2、ハッシュでもよい）")),
        step(0, "git reset <file>", t("With a file it only unstages (same as restore --staged)", "ファイルを付けるとステージから外すだけ")),
        step(2, "git reset --soft HEAD~1", t("Use: only rewrite the commit message", "コミットメッセージだけ書き直したい時")),
        step(3, "git reset HEAD~1", t("Use: redo the commit and fix the files again", "コミットをやり直して、ファイルを直し直したい時")),
        risky(
            4,
            "git reset --hard HEAD~1",
            t("Use: wipe that commit and everything after it", "そのコミット以降をすべてなかったことにしたい時"),
        ),
        risky(5, "git reset --hard ORIG_HEAD", t("ORIG_HEAD is where HEAD was just before the reset", "ORIG_HEAD は reset の直前の HEAD")),
        step(5, "git reflog", t("If ORIG_HEAD was overwritten, find the place here", "上書きされたら reflog で探す")),
    ],
    table: &[
        &[
            t("Option", "オプション"),
            t("HEAD (history)", "HEAD（履歴）"),
            t("Staging area (added)", "ステージ（add 済み）"),
            t("Working tree (files)", "ワーキングツリー（実ファイル）"),
            t("When to use", "使う時"),
        ],
        &[
            t("--soft", "--soft"),
            t("moves back", "戻す"),
            t("kept (still added)", "そのまま（add 済み）"),
            t("kept", "そのまま"),
            t("rewrite only the commit message", "コミットメッセージだけ書き直したい"),
        ],
        &[
            t("--mixed (default)", "--mixed（既定）"),
            t("moves back", "戻す"),
            t("reset (back to before add)", "リセット（add 前に戻る）"),
            t("kept", "そのまま"),
            t("redo the commit, fix the files again", "コミットをやり直し、ファイルを直し直す"),
        ],
        &[
            t("--hard", "--hard"),
            t("moves back", "戻す"),
            t("reset", "リセット"),
            t("reset (changes are lost)", "リセット（変更が消える）"),
            t("wipe that commit and all after it", "そのコミット以降を完全になかったことにする"),
        ],
    ],
    note: Some(t(
        "Only --hard touches your files. The other two never lose a line.",
        "ファイルに触るのは --hard だけ。ほかの 2 つは 1 行も失いません。",
    )),
};

// --------------------------------------------------------- 5. restore/switch

static RESTORE: Flow = Flow {
    topic: Topic::RestoreSwitch,
    title: t(
        "Command guide: checkout was split into switch (branches) and restore (files)",
        "コマンドの対応表: checkout は switch（ブランチ）と restore（ファイル）に分かれた",
    ),
    nodes: &[
        node(0, Start, 1, 0, t("What do you want to move?", "何を動かしたい？")),
        node(1, Decision, 1, 1, t("Files or the branch?", "ファイル？ブランチ？")),
        node(2, Action, 0, 2, t("Discard edits of a file", "ファイルの変更を捨てる")),
        node(3, Action, 1, 2, t("Unstage a file", "ファイルを add 前に戻す")),
        node(4, Action, 2, 2, t("A file from another commit", "別のコミットのファイルを取る")),
        node(5, Action, 3, 2, t("Switch the branch", "ブランチを移る")),
    ],
    edges: &[
        edge(0, 1),
        edge_l(1, 2, t("file", "ファイル")),
        edge_l(1, 3, t("file", "ファイル")),
        edge_l(1, 4, t("file", "ファイル")),
        edge_l(1, 5, t("branch", "ブランチ")),
    ],
    steps: &[
        risky(
            2,
            "git restore <file>",
            t("New (2.23+). Old way: git checkout -- <file>", "新しい書き方（2.23〜）。旧: git checkout -- <file>"),
        ),
        step(3, "git restore --staged <file>", t("New. Old way: git reset HEAD <file>", "新。旧: git reset HEAD <file>")),
        step(
            4,
            "git restore --source=<commit> <file>",
            t("New. Old way: git checkout <commit> -- <file>", "新。旧: git checkout <commit> -- <file>"),
        ),
        step(5, "git switch <branch>", t("New. Old way: git checkout <branch>", "新。旧: git checkout <branch>")),
        step(5, "git switch -c <new-branch>", t("Make and move. Old way: git checkout -b", "作って移る。旧: git checkout -b")),
    ],
    table: &[
        &[t("Command", "コマンド"), t("Moves", "動かすもの"), t("Touches files?", "ファイルは？"), t("Note", "ひとこと")],
        &[
            t("switch", "switch"),
            t("HEAD (branch)", "HEAD（ブランチ）"),
            t("only to match the branch", "ブランチに合わせるだけ"),
            t("branches only", "ブランチ専用"),
        ],
        &[
            t("restore", "restore"),
            t("nothing", "動かさない"),
            t("yes: the files you name", "指定したファイルを書き換える"),
            t("files only", "ファイル専用"),
        ],
        &[
            t("checkout", "checkout"),
            t("both", "両方"),
            t("both", "両方"),
            t("one command, two jobs: easy to mix up", "1 つで 2 役。取り違えやすい"),
        ],
    ],
    note: Some(t(
        "checkout still works. switch and restore are the same jobs split in two, so the name tells you what happens.",
        "checkout は今も使えます。switch と restore は同じ仕事を 2 つに分けたもので、名前で何が起きるか分かります。",
    )),
};

// ---------------------------------------------------- 6. branch/stash/worktree

static PLACE: Flow = Flow {
    topic: Topic::Place,
    title: t("branch / stash / worktree", "branch・stash・worktree の使い分け"),
    nodes: &[
        node(0, Start, 1, 0, t("Something else came up", "別の用事が入った")),
        node(1, Decision, 1, 1, t("How is the current work?", "今の作業はどんな状態？")),
        node(2, Action, 0, 2, t("Good enough to commit", "コミットできる")),
        node(3, Action, 1, 2, t("Half done, back soon", "途中。すぐ戻る")),
        node(4, Action, 2, 2, t("Both must go on side by side", "並行して進めたい")),
    ],
    edges: &[
        edge(0, 1),
        edge_l(1, 2, t("commit it", "コミットして")),
        edge_l(1, 3, t("shelve it", "棚上げ")),
        edge_l(1, 4, t("two places", "場所を分ける")),
    ],
    steps: &[
        step(2, "git switch -c <topic>", t("branch: a name for a line of commits", "branch = コミットの列につける名前")),
        step(2, "git switch <other>", t("Go to the other work", "別の作業へ移る")),
        step(
            3,
            "git stash push -u -m \"wip\"",
            t("stash: park edits (and new files with -u)", "stash = 変更の一時置き場（-u で新規ファイルも）"),
        ),
        step(3, "git stash list", t("What is parked", "置いてあるもの")),
        step(3, "git stash pop", t("Bring the latest back and drop it", "最新を戻して消す")),
        step(3, "git stash apply stash@{1}", t("Bring one back and keep it", "指定を戻す。置き場には残る")),
        step(
            4,
            "git worktree add ../<repo>-<topic> -b <topic>",
            t("worktree: another folder on its own branch", "worktree = 別のフォルダーで別のブランチを開く"),
        ),
        step(4, "git worktree list", t("All the folders of this repository", "このリポジトリのフォルダー一覧")),
        step(4, "git worktree remove ../<repo>-<topic>", t("Done: remove the folder", "終わったらフォルダーを片づける")),
    ],
    table: &[
        &[t("", ""), t("branch", "branch"), t("stash", "stash"), t("worktree", "worktree")],
        &[
            t("What it is", "正体"),
            t("a name on commits", "コミットの名前"),
            t("a shelf for edits", "変更の棚"),
            t("another folder", "もう 1 つのフォルダー"),
        ],
        &[
            t("Files on disk", "ディスク上のファイル"),
            t("one set, swapped", "1 セットを入れ替え"),
            t("one set, cleared", "1 セットを空にする"),
            t("a separate set", "別のセット"),
        ],
        &[t("Side by side?", "同時に進められる？"), t("no", "いいえ"), t("no", "いいえ"), t("yes", "はい")],
        &[
            t("Best for", "向く場面"),
            t("every task", "作業ごと"),
            t("a short detour", "ちょっと寄り道"),
            t("long parallel work, Claude sessions", "長い並行作業・Claude のセッション"),
        ],
    ],
    note: Some(t(
        "A branch cannot be checked out in two worktrees at once.",
        "同じブランチを 2 つの worktree で同時に開くことはできません。",
    )),
};

// ------------------------------------------------------------ 7. rerere/reflog

static RESCUE: Flow = Flow {
    topic: Topic::Rescue,
    title: t("rerere and reflog", "rerere と reflog"),
    nodes: &[
        node(0, Start, 1, 0, t("Same conflict again, or something vanished", "同じ衝突が何度も／何かが消えた")),
        node(1, Decision, 1, 1, t("Which one?", "どちら？")),
        node(2, Action, 0, 2, t("rerere: reuse my resolutions", "rerere: 解消を再利用")),
        node(3, Action, 2, 2, t("reflog: find where HEAD was", "reflog: HEAD の履歴を探す")),
        node(4, Action, 2, 3, t("Bring it back", "取り戻す")),
    ],
    edges: &[edge(0, 1), edge_l(1, 2, t("conflicts", "衝突")), edge_l(1, 3, t("lost work", "消えた")), edge(3, 4)],
    steps: &[
        step(2, "git config --global rerere.enabled true", t("Turn it on once", "最初に 1 回だけ有効にする")),
        step(2, "git rerere status", t("Files whose resolution is being recorded", "解消を記録中のファイル")),
        step(2, "git rerere diff", t("What was recorded for the current conflict", "今の衝突で記録した内容")),
        step(2, "git rerere forget <path>", t("Forget a wrong resolution", "まちがった解消を忘れさせる")),
        step(3, "git reflog", t("Every place HEAD was, newest first (HEAD@{0})", "HEAD がいた場所の一覧（HEAD@{0} が最新）")),
        step(3, "git reflog show <branch>", t("The same for one branch", "ブランチ 1 つ分")),
        step(
            3,
            "git for-each-ref refs/michi/backup/",
            t("michi's own backups taken before risky moves", "michi が危ない操作の前に取った退避"),
        ),
        step(4, "git branch rescue <hash>", t("Safest: a branch on the lost commit", "いちばん安全: 消えたコミットにブランチを付ける")),
        step(4, "git cherry-pick <hash>", t("Take just that commit", "そのコミットだけ持ってくる")),
        risky(
            4,
            "git reset --hard HEAD@{2}",
            t("Move this branch back to that place (loses current edits)", "このブランチをその時点へ戻す（今の変更は消える）"),
        ),
    ],
    table: &[],
    note: Some(t(
        "reflog is local and kept for about 90 days; a commit seen there can still be saved.",
        "reflog は手元だけにあり、約 90 日残ります。そこに見えるコミットはまだ救えます。",
    )),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_diagram_is_wider_than_three_columns() {
        for t in Topic::ALL {
            assert!(t.flow().size().0 <= 3, "{:?}", t);
        }
    }

    #[test]
    fn every_step_belongs_to_a_node() {
        for topic in Topic::ALL {
            let f = topic.flow();
            assert_eq!(f.topic, topic);
            for s in f.steps {
                assert!(f.node(s.node).is_some(), "{:?}: step `{}` has no node {}", topic, s.command, s.node);
            }
        }
    }

    #[test]
    fn edges_join_real_nodes_and_nodes_have_unique_ids_and_places() {
        for topic in Topic::ALL {
            let f = topic.flow();
            for e in f.edges {
                assert!(f.node(e.from).is_some() && f.node(e.to).is_some(), "{topic:?}: edge {}->{}", e.from, e.to);
                assert_ne!(e.from, e.to);
            }
            let mut ids: Vec<u8> = f.nodes.iter().map(|n| n.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), f.nodes.len(), "{topic:?}: duplicate ids");
            let mut at: Vec<(u8, u8)> = f.nodes.iter().map(|n| n.at).collect();
            at.sort_unstable();
            at.dedup();
            assert_eq!(at.len(), f.nodes.len(), "{topic:?}: two nodes in one cell");
        }
    }

    #[test]
    fn a_decision_has_two_ways_out_and_everything_but_the_start_is_reached() {
        for topic in Topic::ALL {
            let f = topic.flow();
            for n in f.nodes.iter().filter(|n| n.kind == Decision) {
                assert!(f.edges.iter().filter(|e| e.from == n.id).count() >= 2, "{topic:?}: decision {} has one way out", n.id);
            }
            for n in f.nodes.iter().filter(|n| n.kind != Start) {
                assert!(f.edges.iter().any(|e| e.to == n.id), "{topic:?}: node {} is not reached", n.id);
            }
            assert_eq!(f.nodes.iter().filter(|n| n.kind == Start).count(), 1);
        }
    }

    #[test]
    fn every_action_with_a_command_slot_and_every_command_is_not_empty() {
        for topic in Topic::ALL {
            for s in topic.flow().steps {
                assert!(s.command.starts_with("git ") || s.command.starts_with("gh "), "{topic:?}: `{}`", s.command);
                assert!(!s.why.en.is_empty() && !s.why.ja.is_empty());
            }
        }
    }

    #[test]
    fn only_the_commands_that_lose_work_are_marked_dangerous() {
        let losing = ["reset --hard", "push --force", "restore <file>", "clean -f"];
        for topic in Topic::ALL {
            for s in topic.flow().steps {
                let lose = losing.iter().any(|l| s.command.contains(l)) && !s.command.contains("--staged");
                assert_eq!(s.danger, lose, "{topic:?}: `{}`", s.command);
            }
        }
    }

    #[test]
    fn a_table_is_rectangular() {
        for topic in Topic::ALL {
            let f = topic.flow();
            if let Some(head) = f.table.first() {
                assert!(f.table.iter().all(|r| r.len() == head.len()), "{topic:?}");
            }
        }
    }

    #[test]
    fn walk_is_reading_order() {
        let w = BASICS.walk();
        assert_eq!(w.len(), BASICS.nodes.len());
        assert_eq!(w[0], 0);
    }

    #[test]
    fn the_key_slot_is_empty_until_the_keys_are_sorted_out() {
        assert!(Topic::ALL.iter().all(|t| t.flow().steps.iter().all(|s| s.key.is_none() && s.screen.is_none())));
    }
}
