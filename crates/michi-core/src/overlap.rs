//! New work (`N`): does the instruction look like it touches files that other
//! running worktrees are changing right now? Pure text matching of the file
//! and folder names written in the instruction against the changed files of
//! the other worktrees. It is only a hint (the form still starts), so it
//! prefers missing a vague mention to crying wolf.

/// A running worktree and the files it is changing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Busy {
    /// What to call it (its branch, or its folder when detached).
    pub label: String,
    /// The changed files, `/`-separated, relative to the worktree's top.
    pub files: Vec<String>,
}

/// One worktree the instruction seems to overlap with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlap {
    pub label: String,
    /// The changed files hit (at most [`MAX_FILES`], in the order given).
    pub files: Vec<String>,
    /// How many files were hit in all.
    pub total: usize,
}

/// The most files named per worktree.
pub const MAX_FILES: usize = 3;

/// The file and folder names written in `text`: words with a `/` or a file
/// extension (`app.rs`, `crates/x`, `src/`), `\` read as `/`, a leading `./`
/// and trailing sentence marks dropped, lower-cased. Only ASCII is read as
/// part of a name, so a name stuck to Japanese text is still found.
pub fn mentions(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || "._-/\\+@~".contains(c))) {
        let w = word.replace('\\', "/").to_ascii_lowercase();
        let w = w.trim_start_matches("./").trim_matches(|c| c == '.' || c == '-' || c == '_' || c == '~');
        let w = w.trim_start_matches('/').to_owned();
        if w.len() < if w.contains('/') { 3 } else { 5 } || w.contains("//") {
            continue;
        }
        let body = w.trim_end_matches('/');
        let has_ext =
            body.rsplit('/').next().is_some_and(|last| last.rsplit_once('.').is_some_and(|(a, e)| !a.is_empty() && !e.is_empty()));
        if (w.contains('/') || has_ext) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// Whether the mention `m` (see [`mentions`]) names `file` or a folder above it.
fn hits(m: &str, file: &str) -> bool {
    let file = file.to_ascii_lowercase();
    let dir = m.ends_with('/');
    let m = m.trim_end_matches('/');
    if m.is_empty() {
        return false;
    }
    if !dir && (file == m || file.ends_with(&format!("/{m}"))) {
        return true;
    }
    // A folder: the file lies under it, anywhere along the path.
    file.starts_with(&format!("{m}/")) || file.contains(&format!("/{m}/"))
}

/// The worktrees in `busy` whose changed files `text` names, by file or by
/// folder. Empty when nothing is written that looks like a path.
pub fn find(text: &str, busy: &[Busy]) -> Vec<Overlap> {
    let ms = mentions(text);
    if ms.is_empty() {
        return Vec::new();
    }
    busy.iter()
        .filter_map(|b| {
            let hit: Vec<&String> = b.files.iter().filter(|f| ms.iter().any(|m| hits(m, f))).collect();
            (!hit.is_empty()).then(|| Overlap {
                label: b.label.clone(),
                total: hit.len(),
                files: hit.into_iter().take(MAX_FILES).cloned().collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn busy(label: &str, files: &[&str]) -> Busy {
        Busy { label: label.into(), files: files.iter().map(|f| (*f).to_owned()).collect() }
    }

    #[test]
    fn names_are_picked_out_of_the_text() {
        assert_eq!(mentions("Fix `app.rs` and ./crates\\michi/src/, then.. done."), ["app.rs", "crates/michi/src/"]);
        assert_eq!(mentions("app.rsを直す。src/ui.rs、も"), ["app.rs", "src/ui.rs"]);
        assert!(mentions("make the graph faster, v1 is fine").is_empty(), "plain words are not names");
        assert!(mentions("a.rs").is_empty(), "too short");
        assert_eq!(mentions("x/y.rs X/Y.rs"), ["x/y.rs"]);
    }

    #[test]
    fn a_file_name_hits_the_file_at_any_depth() {
        let b = [busy("a", &["crates/michi/src/app.rs", "crates/michi/src/wapp.rs"])];
        let r = find("change app.rs please", &b);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].files, ["crates/michi/src/app.rs"], "wapp.rs is not app.rs");
        assert_eq!(find("see michi/src/app.rs", &b).len(), 1);
        assert!(find("see other/src/app.rs", &b).is_empty());
    }

    #[test]
    fn a_folder_hits_every_file_under_it() {
        let b = [busy("a", &["crates/michi/src/app.rs", "docs/x.md"]), busy("b", &["crates/michi-core/src/a.rs"])];
        let r = find("rework crates/michi/src", &b);
        assert_eq!(r.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(), ["a"], "michi-core is not michi");
        assert_eq!(find("tidy michi/src/", &b).len(), 1);
        assert_eq!(find("tidy docs/", &b)[0].files, ["docs/x.md"]);
    }

    #[test]
    fn the_hits_are_capped_but_counted() {
        let files: Vec<String> = (0..5).map(|n| format!("src/f{n}.rs")).collect();
        let b = [Busy { label: "a".into(), files }];
        let r = find("src/", &b);
        assert_eq!((r[0].files.len(), r[0].total), (MAX_FILES, 5));
    }

    #[test]
    fn nothing_written_nothing_found() {
        let b = [busy("a", &["src/app.rs"])];
        assert!(find("", &b).is_empty());
        assert!(find("refactor the app module", &b).is_empty());
        assert!(find("src/app.rs", &[]).is_empty());
    }
}
