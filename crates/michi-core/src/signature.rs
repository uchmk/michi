//! Whether a commit's signature holds, from `git log -1 --format=%G?`.
//! Nothing here sets signing up: it only reads what git says. Git runs gpg
//! (or ssh-keygen) only for a commit that carries a signature, so an
//! unsigned commit costs one quick call.

use std::path::Path;

use crate::git;
use crate::oid::Oid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Signature {
    /// Not signed.
    #[default]
    None,
    /// A good signature (`G`), or one that is good but whose key has not been
    /// trusted (`U`).
    Valid,
    /// A bad signature (`B`), or one by an expired or revoked key (`X`, `Y`,
    /// `R`).
    Invalid,
    /// Signed, but git could not check it: the key is missing, or no
    /// verifier is set up (`E`).
    Unchecked,
}

impl Signature {
    /// From the letter of `%G?`.
    pub fn from_letter(s: &str) -> Signature {
        match s.trim() {
            "G" | "U" => Signature::Valid,
            "B" | "X" | "Y" | "R" => Signature::Invalid,
            "E" => Signature::Unchecked,
            _ => Signature::None,
        }
    }
}

/// The signature state of the commit `id`. The mark is a hint, never an
/// error: when git fails, the commit shows as unsigned.
pub fn read(dir: &Path, id: &Oid) -> Signature {
    let id = id.to_string();
    match git::read_text(dir, &["log", "-1", "--no-walk", "--format=%G?", &id]) {
        Ok(out) => Signature::from_letter(&out),
        Err(_) => Signature::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn letters_map_to_states() {
        assert_eq!(Signature::from_letter("G\n"), Signature::Valid);
        assert_eq!(Signature::from_letter("U"), Signature::Valid);
        for l in ["B", "X", "Y", "R"] {
            assert_eq!(Signature::from_letter(l), Signature::Invalid);
        }
        assert_eq!(Signature::from_letter("E"), Signature::Unchecked);
        assert_eq!(Signature::from_letter("N\n"), Signature::None);
        assert_eq!(Signature::from_letter(""), Signature::None);
    }

    #[test]
    fn an_unsigned_commit_has_none() {
        let r = TempRepo::new("signature-none");
        let head = Oid::parse(r.out(&r.repo, &["rev-parse", "HEAD"]).trim()).unwrap();
        assert_eq!(read(&r.repo, &head), Signature::None);
        assert_eq!(read(&r.root.join("nowhere"), &head), Signature::None);
    }
}
