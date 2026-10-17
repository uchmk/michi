//! Who a worktree commits as (issue #53): the `user.name` and `user.email`
//! git resolves inside it. They can differ per worktree (a repository config
//! plus `config --worktree`), and then the commits of one session carry another
//! name than the next. michi only shows them; changing them is left to git.

use std::path::Path;

use crate::git;

/// `user.name` and `user.email` as git resolves them in one worktree; either
/// is empty when git has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

impl Identity {
    /// `Name <email>`, or what there is of it; empty when git has neither.
    pub fn label(&self) -> String {
        match (self.name.is_empty(), self.email.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.name.clone(),
            (true, false) => format!("<{}>", self.email),
            (false, false) => format!("{} <{}>", self.name, self.email),
        }
    }
}

/// The identity git resolves in the worktree `dir`.
pub fn read(dir: &Path) -> Result<Identity, git::Error> {
    let value = |key: &str| -> Result<String, git::Error> {
        let out = git::read_maybe(dir, &["config", "--get", key])?;
        Ok(out.map(|b| String::from_utf8_lossy(&b).trim().to_owned()).unwrap_or_default())
    };
    Ok(Identity { name: value("user.name")?, email: value("user.email")? })
}

/// What a row shows, one entry per worktree in the order given. Nothing when
/// they all commit as the same identity (there is nothing to tell apart); else
/// every worktree's `Name <email>`, with `None` for one that has neither.
pub fn labels(all: &[Identity]) -> Vec<Option<String>> {
    if all.windows(2).all(|w| w[0] == w[1]) {
        return vec![None; all.len()];
    }
    all.iter().map(|i| Some(i.label()).filter(|l| !l.is_empty())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn who(name: &str, email: &str) -> Identity {
        Identity { name: name.to_owned(), email: email.to_owned() }
    }

    #[test]
    fn label_names_what_there_is() {
        assert_eq!(who("Ann", "a@x.test").label(), "Ann <a@x.test>");
        assert_eq!(who("Ann", "").label(), "Ann");
        assert_eq!(who("", "a@x.test").label(), "<a@x.test>");
        assert_eq!(who("", "").label(), "");
    }

    #[test]
    fn nothing_is_shown_when_all_are_the_same() {
        assert_eq!(labels(&[]), Vec::<Option<String>>::new());
        assert_eq!(labels(&[who("A", "a@x.test")]), vec![None]);
        assert_eq!(labels(&[who("A", "a@x.test"), who("A", "a@x.test")]), vec![None, None]);
    }

    #[test]
    fn all_are_shown_when_one_differs() {
        let l = labels(&[who("A", "a@x.test"), who("B", "b@x.test"), who("", "")]);
        assert_eq!(l, vec![Some("A <a@x.test>".to_owned()), Some("B <b@x.test>".to_owned()), None]);
    }

    #[test]
    fn a_worktree_with_its_own_config_reads_its_own_identity() {
        let t = TempRepo::configured("identity");
        t.git(&t.repo, &["config", "user.name", "Main"]);
        t.git(&t.repo, &["config", "user.email", "main@x.test"]);
        let wt = t.root.join("wt-id");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "id", wt.to_str().unwrap(), "main"]);
        assert_eq!(read(&wt).unwrap(), who("Main", "main@x.test"));
        t.git(&t.repo, &["config", "extensions.worktreeConfig", "true"]);
        t.git(&wt, &["config", "--worktree", "user.name", "Other"]);
        t.git(&wt, &["config", "--worktree", "user.email", "other@x.test"]);
        assert_eq!(read(&wt).unwrap(), who("Other", "other@x.test"));
        assert_eq!(read(&t.repo).unwrap(), who("Main", "main@x.test"));
    }
}
