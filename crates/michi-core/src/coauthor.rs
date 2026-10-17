//! `Co-Authored-By` on commits made in michi (issue #54): `[commit] co_author =
//! "Name <email>"` in config.toml adds that trailer to every commit michi
//! makes. Nothing is added unless it is written.

/// The trailer's value, `Name <email>`, or none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoAuthor(Option<String>);

impl CoAuthor {
    /// `Name <email>` with both parts; anything else is `None`.
    pub fn parse(text: &str) -> Option<CoAuthor> {
        let t = text.trim();
        if t.is_empty() {
            return Some(CoAuthor(None));
        }
        let (name, rest) = t.split_once('<')?;
        let email = rest.strip_suffix('>')?;
        let ok = !name.trim().is_empty() && !email.is_empty() && email.contains('@');
        let clean = |s: &str| !s.chars().any(|c| c == '<' || c == '>' || c.is_control());
        if ok && clean(name) && clean(email) && !email.contains(char::is_whitespace) {
            Some(CoAuthor(Some(format!("{} <{email}>", name.trim()))))
        } else {
            None
        }
    }

    /// Whether a trailer is added at all.
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// The value, `Name <email>`.
    pub fn value(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// `message` with the trailer at its end: after a blank line, or straight
    /// under the trailers the message already ends with. A message that has
    /// the line already is kept as it is.
    pub fn apply(&self, message: &str) -> String {
        let Some(v) = &self.0 else { return message.to_owned() };
        let line = format!("Co-Authored-By: {v}");
        let body = message.trim_end();
        if body.lines().any(|l| l.trim().eq_ignore_ascii_case(&line)) {
            return message.to_owned();
        }
        if body.is_empty() {
            return message.to_owned();
        }
        let last = body.rsplit("\n\n").next().unwrap_or("");
        let in_trailers = body.contains("\n\n") && last.lines().all(is_trailer);
        let sep = if in_trailers { "\n" } else { "\n\n" };
        format!("{body}{sep}{line}\n")
    }
}

/// `Token: value` with a token of letters and dashes.
fn is_trailer(line: &str) -> bool {
    line.split_once(": ").is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ca() -> CoAuthor {
        CoAuthor::parse("Claude <noreply@anthropic.com>").unwrap()
    }

    #[test]
    fn parse_wants_a_name_and_an_address() {
        assert_eq!(CoAuthor::parse("").unwrap(), CoAuthor::default());
        assert_eq!(CoAuthor::parse("  Ann  Lee <a@b.c> ").unwrap().value(), Some("Ann  Lee <a@b.c>"));
        for bad in ["Claude", "<a@b.c>", "Claude <nope>", "Claude <a@b.c", "A <a@b.c> B", "A <a b@c>", "A\n<a@b.c>"] {
            assert!(CoAuthor::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn unset_changes_nothing() {
        assert_eq!(CoAuthor::default().apply("Add a\n"), "Add a\n");
    }

    #[test]
    fn the_trailer_goes_after_a_blank_line() {
        assert_eq!(ca().apply("Add a\n"), "Add a\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n");
        assert_eq!(ca().apply("Add a\n\nbody\n\n"), "Add a\n\nbody\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n");
    }

    #[test]
    fn it_joins_trailers_already_there() {
        assert_eq!(
            ca().apply("Add a\n\nSigned-off-by: Me <m@x.y>\n"),
            "Add a\n\nSigned-off-by: Me <m@x.y>\nCo-Authored-By: Claude <noreply@anthropic.com>\n"
        );
    }

    #[test]
    fn it_is_not_added_twice() {
        let once = ca().apply("Add a\n");
        assert_eq!(ca().apply(&once), once);
        assert_eq!(
            ca().apply("x\n\nco-authored-by: Claude <noreply@anthropic.com>"),
            "x\n\nco-authored-by: Claude <noreply@anthropic.com>"
        );
    }

    #[test]
    fn an_empty_message_stays_empty() {
        assert_eq!(ca().apply("  \n"), "  \n");
    }
}
