//! The widths the user dragged: the graph view's three columns, the graph's
//! author / date / commit columns and the worktree table's columns. Kept in
//! `layout.toml` beside keymap.toml (`key = width` lines), written on a
//! thread when a drag ends.

use std::path::{Path, PathBuf};

/// The worktree table's fixed columns: trunk, uncommitted, unseen, clash, buttons.
pub const TABLE: usize = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct Sizes {
    pub side: f32,
    pub detail: f32,
    /// The graph's width beside a wide diff.
    pub wide_graph: f32,
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
            side: 220.0,
            detail: 440.0,
            wide_graph: 360.0,
            author: 104.0,
            date: 92.0,
            hash: 64.0,
            table: [230.0, 180.0, 130.0, 200.0, 250.0],
            name_share: 0.45,
            file: None,
        }
    }
}

const KEYS: [&str; 6] = ["side", "detail", "wide_graph", "author", "date", "hash"];

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
            if !v.is_finite() || !(20.0..=2000.0).contains(&v) && !k.trim().starts_with("name_share") {
                continue;
            }
            match k.trim() {
                "side" => self.side = v,
                "detail" => self.detail = v,
                "wide_graph" => self.wide_graph = v,
                "author" => self.author = v,
                "date" => self.date = v,
                "hash" => self.hash = v,
                "name_share" if (0.2..=0.8).contains(&v) => self.name_share = v,
                k => {
                    if let Some(n) = k.strip_prefix("table_").and_then(|n| n.parse::<usize>().ok()).filter(|n| *n < TABLE) {
                        self.table[n] = v;
                    }
                }
            }
        }
    }

    fn text(&self) -> String {
        let named = [self.side, self.detail, self.wide_graph, self.author, self.date, self.hash];
        let mut out = String::new();
        for (k, v) in KEYS.iter().zip(named) {
            out += &format!("{k} = {v:.0}\n");
        }
        for (n, v) in self.table.iter().enumerate() {
            out += &format!("table_{n} = {v:.0}\n");
        }
        out + &format!("name_share = {:.2}\n", self.name_share)
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
        let s = Sizes { side: 300.0, table, name_share: 0.6, ..Sizes::default() };
        let mut back = Sizes::default();
        back.read(&s.text());
        assert_eq!(back, s);
        back.read("side = 5\ndetail = nope\nhash = 1e9\ntable_9 = 80\n");
        assert_eq!(back, s);
    }
}
