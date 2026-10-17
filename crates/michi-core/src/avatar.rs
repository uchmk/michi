//! Author avatars (issue #84): who a commit's author is on GitHub, read from
//! the author's e-mail address, and the letters and colour of the circle
//! drawn when there is no picture. Nothing here touches the network.
//!
//! Only a GitHub no-reply address (`1234+login@users.noreply.github.com` or
//! `login@users.noreply.github.com`) names a GitHub account for certain; any
//! other address gets the circle with initials, and is never sent anywhere.

/// A GitHub account an author's e-mail address names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhUser {
    /// The account's number, when the address has it (the stable name).
    pub id: Option<u64>,
    pub login: String,
}

const NOREPLY: &str = "@users.noreply.github.com";

impl GhUser {
    /// The account a GitHub no-reply `email` stands for.
    pub fn of_email(email: &str) -> Option<GhUser> {
        let e = email.trim();
        let cut = e.len().checked_sub(NOREPLY.len())?;
        if !e.is_char_boundary(cut) || !e[cut..].eq_ignore_ascii_case(NOREPLY) {
            return None;
        }
        let local = &e[..cut];
        let (id, login) = match local.split_once('+') {
            Some((id, login)) => (Some(id.parse::<u64>().ok()?), login),
            None => (None, local),
        };
        // `dependabot[bot]` is a login with a suffix; the rest is letters, digits and hyphens.
        let name = login.strip_suffix("[bot]").unwrap_or(login);
        let ok = !name.is_empty() && name.len() <= 39 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        ok.then(|| GhUser { id, login: login.to_owned() })
    }

    /// What names the picture in a cache: the number when known, else the login.
    pub fn key(&self) -> String {
        match self.id {
            Some(id) => format!("u{id}"),
            None => format!("l{}", self.login.to_ascii_lowercase()),
        }
    }

    /// Where the picture is, `px` pixels square.
    pub fn url(&self, px: u32) -> String {
        match self.id {
            Some(id) => format!("https://avatars.githubusercontent.com/u/{id}?s={px}&v=4"),
            None => format!("https://github.com/{}.png?size={px}", self.login.trim_end_matches("[bot]")),
        }
    }
}

/// The letters for the circle: the first letter of the first two words
/// ("Ann Lee" is "AL"), or the first two letters of a single word; one
/// letter for a name in a script without capitals or spaces. `?` for none.
pub fn initials(name: &str) -> String {
    let name = name.trim().trim_end_matches("[bot]");
    let words: Vec<&str> = name.split(|c: char| c.is_whitespace() || matches!(c, '-' | '_' | '.')).filter(|w| !w.is_empty()).collect();
    let up = |c: char| c.to_uppercase().collect::<String>();
    match words.as_slice() {
        [] => "?".to_owned(),
        [one] => {
            let mut it = one.chars();
            let first = it.next().map(up).unwrap_or_default();
            // Han, kana and hangul letters stand for a whole word each.
            if first.chars().next().is_some_and(|c| !c.is_ascii()) {
                return first;
            }
            format!("{first}{}", it.next().map(|c| c.to_lowercase().collect::<String>()).unwrap_or_default())
        }
        [a, b, ..] => format!("{}{}", a.chars().next().map(up).unwrap_or_default(), b.chars().next().map(up).unwrap_or_default()),
    }
}

/// A hue in `0.0..1.0` that depends only on `name`, so an author keeps the colour.
pub fn hue(name: &str) -> f32 {
    // FNV-1a: the same on every machine and run.
    let mut h: u32 = 0x811c_9dc5;
    for b in name.trim().to_lowercase().bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    (h % 360) as f32 / 360.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gh(id: Option<u64>, login: &str) -> Option<GhUser> {
        Some(GhUser { id, login: login.into() })
    }

    #[test]
    fn a_noreply_address_names_the_account() {
        assert_eq!(GhUser::of_email("1234+ann@users.noreply.github.com"), gh(Some(1234), "ann"));
        assert_eq!(GhUser::of_email("ann-lee@users.noreply.github.com"), gh(None, "ann-lee"));
        assert_eq!(GhUser::of_email(" 7+Ann@Users.NoReply.GitHub.com "), gh(Some(7), "Ann"));
        assert_eq!(GhUser::of_email("41898282+github-actions[bot]@users.noreply.github.com"), gh(Some(41898282), "github-actions[bot]"));
    }

    #[test]
    fn other_addresses_name_nobody() {
        for e in [
            "ann@example.com",
            "",
            "@users.noreply.github.com",
            "x+ann@users.noreply.github.com",
            "a b@users.noreply.github.com",
            "ann@github.com",
            "é",
        ] {
            assert_eq!(GhUser::of_email(e), None, "{e}");
        }
    }

    #[test]
    fn the_url_and_key_follow_the_number_when_known() {
        let a = GhUser::of_email("12+Ann@users.noreply.github.com").unwrap();
        assert_eq!(a.key(), "u12");
        assert_eq!(a.url(48), "https://avatars.githubusercontent.com/u/12?s=48&v=4");
        let b = GhUser::of_email("Ann@users.noreply.github.com").unwrap();
        assert_eq!(b.key(), "lann");
        assert_eq!(b.url(48), "https://github.com/Ann.png?size=48");
    }

    #[test]
    fn initials_are_one_or_two_letters() {
        assert_eq!(initials("Ann Lee"), "AL");
        assert_eq!(initials("ann"), "An");
        assert_eq!(initials("ann-lee"), "AL");
        assert_eq!(initials("a"), "A");
        assert_eq!(initials("山田 太郎"), "山太");
        assert_eq!(initials("山田"), "山");
        assert_eq!(initials("dependabot[bot]"), "De");
        assert_eq!(initials("  "), "?");
        assert_eq!(initials("Ann Bo Cy"), "AB");
    }

    #[test]
    fn a_hue_is_steady_and_in_range() {
        assert_eq!(hue("Ann"), hue(" ann "));
        assert_ne!(hue("Ann"), hue("Bob"));
        for n in ["", "x", "山田", "a very long name indeed"] {
            assert!((0.0..1.0).contains(&hue(n)));
        }
    }
}
