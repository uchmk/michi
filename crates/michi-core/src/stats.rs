//! The statistics screen's numbers: commits per contributor and commits per
//! week or month, over a range of the history. One read of `git log` (names
//! and addresses through the mailmap, merges left out), then everything is
//! counted here, so the parsing and the counting are tested without a window.
//! A read, so it runs with the read-only environment of [`git::read`].

use std::collections::HashMap;
use std::path::Path;

use crate::git;

/// Most commits read; a longer history is counted from its newest.
const MAX_COMMITS: usize = 200_000;
/// The days of [`Range::Recent`].
pub const RECENT_DAYS: i64 = 90;
const DAY: i64 = 86_400;

/// Which part of the history is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    /// Every commit of every branch and tag.
    All,
    /// The last [`RECENT_DAYS`] days, of every branch and tag.
    Recent,
    /// What HEAD reaches: the branch the window is on.
    Branch,
}

impl Range {
    /// The next one, round: whole history, last 90 days, the branch.
    pub fn next(self) -> Range {
        match self {
            Range::All => Range::Recent,
            Range::Recent => Range::Branch,
            Range::Branch => Range::All,
        }
    }
}

/// What the activity bars count in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grain {
    Week,
    Month,
}

impl Grain {
    pub fn other(self) -> Grain {
        match self {
            Grain::Week => Grain::Month,
            Grain::Month => Grain::Week,
        }
    }
}

/// One contributor (one address, after the mailmap).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author {
    /// The name on their newest commit in the range.
    pub name: String,
    pub email: String,
    pub commits: usize,
    /// Their oldest and newest commit in the range, in seconds since the epoch.
    pub first: i64,
    pub last: i64,
}

/// A week or a month and the commits in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bucket {
    /// [`label`] turns it into text. A week is counted from its Monday.
    pub index: i64,
    pub commits: usize,
}

/// The numbers of a range.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub total: usize,
    /// Most commits first; on a tie, the one with the newest commit.
    pub authors: Vec<Author>,
    /// The commit times, newest first.
    pub times: Vec<i64>,
}

impl Stats {
    /// Commits per `grain`, oldest first, the empty ones in between kept as
    /// zeros so the bars keep their spacing.
    pub fn buckets(&self, grain: Grain) -> Vec<Bucket> {
        let mut counts: HashMap<i64, usize> = HashMap::new();
        for &t in &self.times {
            *counts.entry(index_of(grain, t)).or_default() += 1;
        }
        let (Some(&lo), Some(&hi)) = (counts.keys().min(), counts.keys().max()) else { return Vec::new() };
        (lo..=hi).map(|index| Bucket { index, commits: counts.get(&index).copied().unwrap_or(0) }).collect()
    }
}

/// The week (from the Monday) or the month `secs` falls in.
fn index_of(grain: Grain, secs: i64) -> i64 {
    let day = secs.div_euclid(DAY);
    match grain {
        // 1970-01-01 was a Thursday, so Monday 1969-12-29 starts week 0.
        Grain::Week => (day + 3).div_euclid(7),
        Grain::Month => {
            let (y, m, _) = civil(day);
            y * 12 + i64::from(m) - 1
        }
    }
}

/// `(year, month, day)` of the day count since 1970-01-01 (proleptic Gregorian).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The name of a bucket: `2026-10` for a month, `10-12` (month and day of its
/// Monday) for a week.
pub fn label(grain: Grain, index: i64) -> String {
    match grain {
        Grain::Month => format!("{}-{:02}", index.div_euclid(12), index.rem_euclid(12) + 1),
        Grain::Week => {
            let (_, m, d) = civil(index * 7 - 3);
            format!("{m:02}-{d:02}")
        }
    }
}

/// `2026-10-17` for a time in seconds since the epoch (UTC).
pub fn date(secs: i64) -> String {
    let (y, m, d) = civil(secs.div_euclid(DAY));
    format!("{y}-{m:02}-{d:02}")
}

/// Reads and counts `range` of the repository at `dir`; `now` is the time in
/// seconds since the epoch ([`Range::Recent`] counts back from it).
pub fn read(dir: &Path, range: Range, now: i64) -> Result<Stats, git::Error> {
    let limit = format!("-n{MAX_COMMITS}");
    let mut args = vec!["log", "--no-merges", "--format=%aN%x00%aE%x00%at", &limit];
    args.push(if range == Range::Branch { "HEAD" } else { "--all" });
    let since = (range == Range::Recent).then(|| now - RECENT_DAYS * DAY);
    match git::read_text(dir, &args) {
        Ok(text) => Ok(parse(&text, since)),
        // A repository with no commit yet has no HEAD to read.
        Err(git::Error::Git(_)) if range == Range::Branch => Ok(Stats::default()),
        Err(e) => Err(e),
    }
}

/// Counts the lines of `git log --format=%aN%x00%aE%x00%at` (newest first);
/// with `since`, commits written before it are left out.
pub fn parse(text: &str, since: Option<i64>) -> Stats {
    let mut stats = Stats::default();
    let mut by_email: HashMap<String, usize> = HashMap::new();
    for line in text.lines() {
        let mut f = line.splitn(3, '\0');
        let (Some(name), Some(email), Some(at)) = (f.next(), f.next(), f.next()) else { continue };
        let Ok(at) = at.trim().parse::<i64>() else { continue };
        if since.is_some_and(|s| at < s) {
            continue;
        }
        stats.total += 1;
        stats.times.push(at);
        let i = *by_email.entry(email.to_lowercase()).or_insert_with(|| {
            stats.authors.push(Author { name: name.to_owned(), email: email.to_owned(), commits: 0, first: at, last: at });
            stats.authors.len() - 1
        });
        let a = &mut stats.authors[i];
        a.commits += 1;
        a.first = a.first.min(at);
        a.last = a.last.max(at);
    }
    stats.times.sort_by_key(|&t| std::cmp::Reverse(t));
    // Stable: the authors were met newest first, so ties stay newest first.
    stats.authors.sort_by_key(|a| std::cmp::Reverse(a.commits));
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    const D: i64 = DAY;
    /// 2024-02-29, a leap day and a Thursday.
    const LEAP: i64 = 19_782;

    fn line(name: &str, email: &str, at: i64) -> String {
        format!("{name}\0{email}\0{at}\n")
    }

    #[test]
    fn civil_dates_and_labels() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(LEAP), (2024, 2, 29));
        assert_eq!(date(LEAP * D + 3_600), "2024-02-29");
        assert_eq!(label(Grain::Month, index_of(Grain::Month, LEAP * D)), "2024-02");
        // Its week starts Monday 02-26.
        assert_eq!(label(Grain::Week, index_of(Grain::Week, LEAP * D + 5)), "02-26");
        // Sunday and the next Monday are two weeks.
        let monday = LEAP - 3;
        assert_eq!(index_of(Grain::Week, (monday - 1) * D) + 1, index_of(Grain::Week, monday * D));
        assert_eq!(index_of(Grain::Week, (monday + 6) * D + D - 1), index_of(Grain::Week, monday * D));
    }

    #[test]
    fn authors_are_counted_by_address_most_first() {
        let text = [
            line("Ann", "ann@x.test", 500),
            line("Bob", "bob@x.test", 400),
            line("Ann B.", "ANN@x.test", 300),
            line("Cy", "cy@x.test", 200),
            line("Bob", "bob@x.test", 100),
            "garbage\n".to_owned(),
        ]
        .concat();
        let s = parse(&text, None);
        assert_eq!(s.total, 5);
        let who: Vec<_> = s.authors.iter().map(|a| (a.name.as_str(), a.commits)).collect();
        // Ann and Bob tie on two: the one with the newest commit is first.
        assert_eq!(who, [("Ann", 2), ("Bob", 2), ("Cy", 1)]);
        assert_eq!((s.authors[0].first, s.authors[0].last), (300, 500));
    }

    #[test]
    fn since_leaves_older_commits_out() {
        let text = [line("A", "a@x.test", 1000), line("A", "a@x.test", 10)].concat();
        let s = parse(&text, Some(500));
        assert_eq!((s.total, s.authors[0].commits), (1, 1));
        assert_eq!(parse("", None), Stats::default());
    }

    #[test]
    fn buckets_keep_the_empty_weeks_between() {
        let monday = LEAP - 3;
        let at = |day: i64| line("A", "a@x.test", day * D + 60);
        let text = [at(monday + 15), at(monday + 1), at(monday)].concat();
        let b = parse(&text, None).buckets(Grain::Week);
        assert_eq!(b.iter().map(|b| b.commits).collect::<Vec<_>>(), [2, 0, 1]);
        let m = parse(&text, None).buckets(Grain::Month);
        assert_eq!(m.iter().map(|b| b.commits).sum::<usize>(), 3);
        assert!(parse("", None).buckets(Grain::Week).is_empty());
    }

    #[test]
    fn read_counts_a_real_history_by_range_and_follows_the_mailmap() {
        let t = TempRepo::new("stats");
        t.write(&t.repo, "a.txt", "1");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "two"]);
        // A third commit under another address that the mailmap folds into "michi".
        t.write(&t.repo, ".mailmap", "michi <michi@example.invalid> <old@example.invalid>\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "three", "--author=Old <old@example.invalid>"]);
        t.git(&t.repo, &["checkout", "-q", "-b", "other"]);
        t.write(&t.repo, "b.txt", "2");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "four"]);
        t.git(&t.repo, &["checkout", "-q", "main"]);

        let all = read(&t.repo, Range::All, i64::MAX / 2).unwrap();
        assert_eq!(all.total, 4);
        assert_eq!(all.authors.len(), 1, "the mailmap merges the spellings: {:?}", all.authors);
        assert_eq!(all.authors[0].commits, 4);
        assert_eq!(read(&t.repo, Range::Branch, 0).unwrap().total, 3);
        // From far later nothing is recent; from a day after the newest, all is.
        assert_eq!(read(&t.repo, Range::Recent, i64::MAX / 2).unwrap().total, 0);
        assert_eq!(read(&t.repo, Range::Recent, all.times[0] + D).unwrap().total, 4);
    }

    #[test]
    fn a_repository_with_no_commit_has_no_numbers_on_any_range() {
        let t = TempRepo::new("stats-empty");
        let empty = t.root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        t.git(&empty, &["init", "-q", "-b", "main"]);
        for r in [Range::All, Range::Recent, Range::Branch] {
            assert_eq!(read(&empty, r, 0).unwrap().total, 0, "{r:?}");
        }
    }

    #[test]
    fn the_range_goes_round() {
        assert_eq!(Range::All.next().next().next(), Range::All);
        assert_eq!(Grain::Week.other().other(), Grain::Week);
    }
}
