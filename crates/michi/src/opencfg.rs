//! `[open]` of michi's config.toml: the command lines that open a worktree's
//! folder in a terminal, an editor and the file manager
//! (`michi_core::openin`). A key that is not there, or is empty, keeps the
//! default.

use michi_core::openin::{self, Tool};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default)]
pub struct OpenCfg {
    pub terminal: String,
    pub editor: String,
    pub explorer: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    open: OpenCfg,
}

impl OpenCfg {
    /// `[open]` of the config.toml `text` (none: no file), with what to warn of.
    pub fn read(text: Option<&str>) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut o = match text.map(toml::from_str::<File>) {
            Some(Ok(f)) => f.open,
            Some(Err(e)) => {
                warnings.push(format!("config.toml: {}", e.message()));
                Self::default()
            }
            None => Self::default(),
        };
        for t in Tool::ALL {
            let line = o.slot(t);
            if !line.trim().is_empty() && openin::argv(line, std::path::Path::new("x")).is_err() {
                warnings.push(format!("config.toml: [open] `{}` is not a command line: {line}", t.key()));
                o.slot_mut(t).clear();
            }
        }
        (o, warnings)
    }

    fn slot(&self, t: Tool) -> &String {
        match t {
            Tool::Terminal => &self.terminal,
            Tool::Editor => &self.editor,
            Tool::Explorer => &self.explorer,
        }
    }

    fn slot_mut(&mut self, t: Tool) -> &mut String {
        match t {
            Tool::Terminal => &mut self.terminal,
            Tool::Editor => &mut self.editor,
            Tool::Explorer => &mut self.explorer,
        }
    }

    /// The command line for `t`: the one written, else the default.
    pub fn command(&self, t: Tool) -> String {
        let line = self.slot(t).trim();
        if line.is_empty() {
            openin::default_command(t).to_owned()
        } else {
            line.to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_not_written_keeps_the_default() {
        let (o, w) = OpenCfg::read(None);
        assert!(w.is_empty());
        assert_eq!(o.command(Tool::Terminal), "tsumugi new {path}");
        let (o, w) = OpenCfg::read(Some("[open]\neditor = \"zed {path}\"\nexplorer = \"\"\n"));
        assert!(w.is_empty());
        assert_eq!(o.command(Tool::Editor), "zed {path}");
        assert_eq!(o.command(Tool::Explorer), openin::default_command(Tool::Explorer));
    }

    #[test]
    fn a_bad_line_is_warned_of_and_set_back() {
        let (o, w) = OpenCfg::read(Some("[open]\neditor = \"a \\\"b\"\n"));
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(o.command(Tool::Editor), openin::default_command(Tool::Editor));
    }
}
