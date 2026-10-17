//! The sizes the user dragged: the graph view's panes (as the ratios of
//! ito-layout's splits), the graph's author / date / commit columns and the worktree table's columns. Kept in
//! `layout.toml` beside keymap.toml (`key = width` lines), written on a
//! thread when a drag ends.

use std::path::{Path, PathBuf};

use ito_layout::Node;

/// The worktree table's fixed columns: trunk, uncommitted, unseen, review, clash, buttons.
pub const TABLE: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct Sizes {
    /// The side column's share of the whole width.
    pub side: f32,
    /// The graph's share of what the side column leaves.
    pub graph: f32,
    /// The graph's share of the width beside a wide diff (no side column).
    pub wide: f32,
    pub author: f32,
    pub date: f32,
    pub hash: f32,
    pub table: [f32; TABLE],
    /// The name column's share of the room the Claude column shares with it.
    pub name_share: f32,
    file: Option<PathBuf>,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            side: 0.16,
            graph: 0.63,
            wide: 0.28,
            author: 104.0,
            date: 92.0,
            hash: 64.0,
            table: [230.0, 180.0, 130.0, 130.0, 200.0, 250.0],
            name_share: 0.45,
            file: None,
        }
    }
}

const KEYS: [&str; 3] = ["author", "date", "hash"];

impl Sizes {
    /// Read `layout.toml` in `dir`; the defaults for what is missing or odd.
    pub fn load(dir: Option<&Path>) -> Self {
        let mut s = Self::default();
        let Some(dir) = dir else { return s };
        s.file = Some(dir.join("layout.toml"));
        if let Some(text) = s.file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()) {
            s.read(&text);
        }
        s
    }

    fn read(&mut self, text: &str) {
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let Ok(v) = v.trim().parse::<f32>() else { continue };
            let ratio = (0.05..=0.95).contains(&v);
            let wide = (20.0..=2000.0).contains(&v);
            match k.trim() {
                "side_ratio" if ratio => self.side = v,
                "graph_ratio" if ratio => self.graph = v,
                "wide_ratio" if ratio => self.wide = v,
                _ if !v.is_finite() => {}
                "author" if wide => self.author = v,
                "date" if wide => self.date = v,
                "hash" if wide => self.hash = v,
                "name_share" if (0.2..=0.8).contains(&v) => self.name_share = v,
                k if wide => {
                    if let Some(n) = k.strip_prefix("table_").and_then(|n| n.parse::<usize>().ok()).filter(|n| *n < TABLE) {
                        self.table[n] = v;
                    }
                }
                _ => {}
            }
        }
    }

    fn text(&self) -> String {
        let named = [self.author, self.date, self.hash];
        let mut out = format!(
            "side_ratio = {:.3}
graph_ratio = {:.3}
wide_ratio = {:.3}
",
            self.side, self.graph, self.wide
        );
        for (k, v) in KEYS.iter().zip(named) {
            out += &format!("{k} = {v:.0}\n");
        }
        for (n, v) in self.table.iter().enumerate() {
            out += &format!("table_{n} = {v:.0}\n");
        }
        out + &format!("name_share = {:.2}\n", self.name_share)
    }

    /// Take the ratios of the dragged split (`side` says there are three
    /// panes: side, graph, detail; else graph, detail).
    pub fn set_panes(&mut self, node: &Node<crate::ui::Col>, side: bool) {
        let Node::Split { ratio, second, .. } = node else { return };
        if !side {
            self.wide = *ratio;
        } else if let Node::Split { ratio: inner, .. } = &**second {
            (self.side, self.graph) = (*ratio, *inner);
        }
    }

    /// Write the file on a thread (nothing when there is no folder).
    pub fn save(&self) {
        let Some(file) = self.file.clone() else { return };
        let text = self.text();
        std::thread::spawn(move || {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = ito_common::write_atomic(&file, text);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_written_reads_back_and_odd_lines_are_left() {
        let mut table = Sizes::default().table;
        table[2] = 99.0;
        let s = Sizes { side: 0.3, table, name_share: 0.6, ..Sizes::default() };
        let mut back = Sizes::default();
        back.read(&s.text());
        assert_eq!(back, s);
        back.read("side = 5\ndetail = nope\nhash = 1e9\ntable_9 = 80\n");
        assert_eq!(back, s);
    }
}
