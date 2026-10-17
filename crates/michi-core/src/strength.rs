//! How hard the window makes you say yes (#80): three steps. What can be
//! brought back asks for a plain yes; what cannot is asked in red; what
//! overwrites what the remote has is asked by name, typed out.

/// How strong a question is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Strength {
    /// Can be brought back: `y` or a click.
    #[default]
    Easy,
    /// Hard to bring back: the same yes, drawn in the error colour.
    Loud,
    /// Overwrites the remote: the name is typed, then Enter.
    Typed,
}

impl Strength {
    /// Whether the question is drawn in the error colour.
    pub fn is_red(self) -> bool {
        self != Strength::Easy
    }
}

/// Whether `typed` is `word`: ignoring spaces at the ends, nothing else.
pub fn typed_ok(word: &str, typed: &str) -> bool {
    !word.is_empty() && typed.trim() == word
}

/// A typed character into `typed`: control characters are dropped.
pub fn type_char(typed: &mut String, c: char) {
    if !c.is_control() {
        typed.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_whole_name_passes() {
        assert!(typed_ok("origin/feat", "origin/feat"));
        assert!(typed_ok("origin/feat", "  origin/feat "));
        assert!(!typed_ok("origin/feat", "origin/fea"));
        assert!(!typed_ok("origin/feat", "Origin/feat"));
        assert!(!typed_ok("", ""));
    }

    #[test]
    fn typing_drops_control_characters() {
        let mut t = String::new();
        for c in "a\nb\t/".chars() {
            type_char(&mut t, c);
        }
        assert_eq!(t, "ab/");
    }

    #[test]
    fn red_from_the_second_step() {
        assert!(!Strength::Easy.is_red());
        assert!(Strength::Loud.is_red() && Strength::Typed.is_red());
    }
}
