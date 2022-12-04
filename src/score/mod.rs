use crate::Params;
use std::cmp::Ordering;

type ScoreMatrix = ndarray::Array2<f32>;

/// Result of querying the score against a candidate
#[derive(Copy, Clone, Debug)]
pub struct Score {
    pub index: usize,
    pub score: f32,
}

impl Score {
    pub const fn new(index: usize) -> Self {
        Self::with_score(index, Params::SCORE_MIN)
    }

    pub const fn with_score(index: usize, score: f32) -> Self {
        Self { index, score }
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.score
                .partial_cmp(&other.score)
                .unwrap_or(Ordering::Less)
                .reverse(),
        )
    }
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for Score {}

#[allow(dead_code)]
pub fn has_match(query: &str, candidate: &(impl crate::SearchItem + ?Sized)) -> bool {
    let mut chars = candidate.as_str().chars();
    query
        .chars()
        .all(|right| chars.any(|left| left.to_lowercase().eq(right.to_lowercase())))
}

pub fn score(query: &str, candidate: &str, index: usize) -> Score {
    let (q_len, c_len) = match Metric::classify(query, candidate) {
        Metric::Score(s) => return Score::with_score(index, s),
        Metric::Lengths(q, c) => (q, c),
    };

    let (best_score_overall, _) = {
        let match_bonuses = candidate_match_bonuses(candidate);

        // Matrix of the best score for each position ending in a match
        let mut best_score_w_ending = ScoreMatrix::zeros((q_len, c_len));
        // Matrix for the best score for each position.
        let mut best_score_overall = ScoreMatrix::zeros((q_len, c_len));

        for (i, q_char) in query.chars().enumerate() {
            let mut prev_score = Params::SCORE_MIN;
            let gap_score = if i == q_len - 1 {
                Params::SCORE_GAP_TRAILING
            } else {
                Params::SCORE_GAP_INNER
            };

            for (j, c_char) in candidate.chars().enumerate() {
                if q_char.to_lowercase().eq(c_char.to_lowercase()) {
                    // Get the score bonus for matching this char
                    let score = if i == 0 {
                        // Beginning of the query, penalty for leading gap
                        (j as f32).mul_add(Params::SCORE_GAP_LEADING, match_bonuses[j])
                    } else if j != 0 {
                        // Middle of both query and candidate
                        // Either give it the match bonus, or use the consecutive
                        // match (which wil always be higher, but doesn't stack
                        // with match bonus)
                        (best_score_overall[[i - 1, j - 1]] + match_bonuses[j]).max(
                            best_score_w_ending[[i - 1, j - 1]] + Params::SCORE_MATCH_CONSECUTIVE,
                        )
                    } else {
                        Params::SCORE_MIN
                    };

                    prev_score = score.max(prev_score + gap_score);
                    best_score_overall[[i, j]] = prev_score;
                    best_score_w_ending[[i, j]] = score;
                } else {
                    // Give the score penalty for the gap
                    prev_score += gap_score;
                    best_score_overall[[i, j]] = prev_score;
                    // We don't end in a match
                    best_score_w_ending[[i, j]] = Params::SCORE_MIN;
                }
            }
        }

        (best_score_overall, best_score_w_ending)
    };

    Score::with_score(index, best_score_overall[[q_len - 1, c_len - 1]])
}

enum Metric {
    Lengths(usize, usize),
    Score(f32),
}

impl Metric {
    fn classify(query: &str, candidate: &str) -> Self {
        if candidate.len() > Params::CANDIDATE_MAX_BYTES || query.is_empty() {
            // Candidate too long or query too short
            return Self::Score(Params::SCORE_MIN);
        }

        let q_len = query.chars().count();
        let c_len = candidate.chars().count();

        if q_len == c_len {
            // This is only called when there _is_ a match (candidate contains all
            // chars of query in the right order, so equal lengths mean equal
            // strings
            return Self::Score(Params::SCORE_MAX);
        }

        if c_len > Params::CANDIDATE_MAX_CHARS {
            // Too many characters
            return Self::Score(Params::SCORE_MIN);
        }

        Self::Lengths(q_len, c_len)
    }
}

fn candidate_match_bonuses(candidate: &str) -> Vec<f32> {
    let mut prev_char = '/';
    candidate
        .chars()
        .map(|current| {
            let s = character_match_bonus(current, prev_char);
            prev_char = current;
            s
        })
        .collect()
}

fn character_match_bonus(current: char, previous: char) -> f32 {
    if current.is_uppercase() && previous.is_lowercase() {
        return Params::SCORE_MATCH_CAPITAL;
    }

    match previous {
        '/' => Params::SCORE_MATCH_SLASH,
        '.' => Params::SCORE_MATCH_DOT,
        _ if is_separator(previous) => Params::SCORE_MATCH_WORD,
        _ => 0.0,
    }
}

const fn is_separator(character: char) -> bool {
    matches!(character, ' ' | '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(query: &str, candidate: &str) -> Score {
        super::score(query, candidate, 0)
    }

    #[test]
    fn exact_match() {
        assert!(has_match("query", "query"));
        assert!(has_match(
            "156aufsdn926f9=sdk/~']",
            "156aufsdn926f9=sdk/~']"
        ));
        assert!(has_match(
            "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
            "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
        ));
    }

    #[test]
    fn paratial_match() {
        assert!(has_match("ca", "candidate"));
        assert!(has_match("cat", "candidate"));
        assert!(has_match("ndt", "candidate"));
        assert!(has_match("nate", "candidate"));
        assert!(has_match("56aufn92=sd/~']", "156aufsdn926f9=sdk/~']"));
        assert!(has_match(
            "😨Ɣ·®x¯ÍĞɅƁƹ♺à☆ǈ´ƙÑ♫",
            "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
        ));
    }

    #[test]
    fn case_match() {
        assert!(has_match("QUERY", "query"));
        assert!(has_match("query", "QUERY"));
        assert!(has_match("QuEry", "query"));
        assert!(has_match("прописная буква", "ПРОПИСНАЯ БУКВА"))
    }

    #[test]
    fn empty_match() {
        assert!(has_match("", ""));
        assert!(has_match("", "candidate"));
        assert!(has_match("", "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"));
        assert!(has_match("", "прописная БУКВА"));
        assert!(has_match("", "a"));
        assert!(has_match("", "4561"));
    }

    #[test]
    fn bad_match() {
        assert!(!has_match("acb", "abc"));
        assert!(!has_match("a", ""));
        assert!(!has_match("abc", "def"));
        assert!(!has_match("😨Ɣ·®x¯ÍĞ.Ʌ", "5ù¨ȼ♕☩♘⚁^"));
        assert!(!has_match("прописная БУКВА", "прописнаяБУКВА"));
        assert!(!has_match("БУКВА прописная", "прописная БУКВА"));
    }

    #[test]
    fn score_pref_word_start() {
        assert!(score("amor", "app/models/order").score > score("amor", "app/models/zrder").score);
        assert!(score("amor", "app models-order").score > score("amor", "app models zrder").score);
        assert!(score("qart", "QuArTz").score > score("qart", "QuaRTz").score);
    }

    #[test]
    fn score_pref_consecutive_letters() {
        assert!(score("amo", "app/m/foo").score < score("amo", "app/models/foo").score);
    }

    #[test]
    fn score_pref_contiguous_vs_word() {
        assert!(score("gemfil", "Gemfile.lock").score < score("gemfil", "Gemfile").score);
    }

    #[test]
    fn score_pref_shorter() {
        assert!(score("abce", "abcdef").score > score("abce", "abc de").score);
        assert!(score("abc", "    a b c ").score > score("abc", " a  b  c ").score);
        assert!(score("abc", " a b c    ").score > score("abc", " a  b  c ").score);
        assert!(score("test", "tests").score > score("test", "testing").score);
    }

    #[test]
    fn score_prefer_start() {
        assert!(score("test", "testing").score > score("test", "/testing").score);
    }

    #[test]
    fn score_exact() {
        assert_eq!(Params::SCORE_MAX, score("query", "query").score);
        assert_eq!(
            Params::SCORE_MAX,
            score("156aufsdn926f9=sdk/~']", "156aufsdn926f9=sdk/~']").score
        );
        assert_eq!(
            Params::SCORE_MAX,
            score(
                "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
                "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
            )
            .score
        );
    }

    #[test]
    fn score_empty() {
        assert_eq!(Params::SCORE_MIN, score("", "").score);
        assert_eq!(Params::SCORE_MIN, score("", "candidate").score);
        assert_eq!(
            Params::SCORE_MIN,
            score("", "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫").score
        );
        assert_eq!(Params::SCORE_MIN, score("", "прописная БУКВА").score);
        assert_eq!(Params::SCORE_MIN, score("", "a").score);
        assert_eq!(Params::SCORE_MIN, score("", "4561").score);
    }

    #[test]
    fn score_gaps() {
        assert_eq!(Params::SCORE_GAP_LEADING, score("a", "*a").score);
        assert_eq!(Params::SCORE_GAP_LEADING * 2.0, score("a", "*ba").score);
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0 + Params::SCORE_GAP_TRAILING,
            score("a", "**a*").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0 + Params::SCORE_GAP_TRAILING * 2.0,
            score("a", "**a**").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0
                + Params::SCORE_MATCH_CONSECUTIVE
                + Params::SCORE_GAP_TRAILING * 2.0,
            score("aa", "**aa♺*").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0
                + Params::SCORE_GAP_INNER
                + Params::SCORE_MATCH_WORD
                + Params::SCORE_GAP_TRAILING * 2.0,
            score("ab", "**a-b♺*").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING
                + Params::SCORE_GAP_LEADING
                + Params::SCORE_GAP_INNER
                + Params::SCORE_GAP_TRAILING
                + Params::SCORE_GAP_TRAILING,
            score("aa", "**a♺a**").score
        );
    }

    #[test]
    fn score_consecutive() {
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_MATCH_CONSECUTIVE,
            score("aa", "*aa").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_MATCH_CONSECUTIVE * 2.0,
            score("aaa", "♫aaa").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_GAP_INNER + Params::SCORE_MATCH_CONSECUTIVE,
            score("aaa", "*a*aa").score
        );
    }

    #[test]
    fn score_slash() {
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_MATCH_SLASH,
            score("a", "/a").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0 + Params::SCORE_MATCH_SLASH,
            score("a", "*/a").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0
                + Params::SCORE_MATCH_SLASH
                + Params::SCORE_MATCH_CONSECUTIVE,
            score("aa", "a/aa").score
        );
    }

    #[test]
    fn score_capital() {
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_MATCH_CAPITAL,
            score("a", "bA").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0 + Params::SCORE_MATCH_CAPITAL,
            score("a", "baA").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 2.0
                + Params::SCORE_MATCH_CAPITAL
                + Params::SCORE_MATCH_CONSECUTIVE,
            score("aa", "😞aAa").score
        );
    }

    #[test]
    fn score_dot() {
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_MATCH_DOT,
            score("a", ".a").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING * 3.0 + Params::SCORE_MATCH_DOT,
            score("a", "*a.a").score
        );
        assert_eq!(
            Params::SCORE_GAP_LEADING + Params::SCORE_GAP_INNER + Params::SCORE_MATCH_DOT,
            score("a", "♫a.a").score
        );
    }
}
