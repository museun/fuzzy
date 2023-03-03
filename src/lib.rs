// based on https://github.com/jmaargh/fzyr but with a lot of changes
// original license: MIT License

pub fn search<T>(query: &str, candidates: &[T]) -> Vec<Score>
where
    T: SearchItem,
{
    let mut out = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let mut iter = c.as_str().chars();
            query
                .chars()
                .all(|left| iter.any(|right| right.to_lowercase().eq(left.to_lowercase())))
        })
        .fold(Vec::with_capacity(candidates.len()), |mut a, (i, c)| {
            a.push(score(query, c.as_str(), i));
            a
        });

    out.sort_unstable_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Less));
    out
}

#[derive(Copy, Clone, Debug)]
pub struct Score {
    pub index: usize,
    pub score: f32,
}

impl PartialOrd for Score {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let cmp = self
            .score
            .partial_cmp(&other.score)
            .unwrap_or(std::cmp::Ordering::Less)
            .reverse();
        Some(cmp)
    }
}

impl PartialEq for Score {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.score.total_cmp(&other.score).is_eq()
    }
}

impl Eq for Score {}

pub trait SearchItem {
    fn as_str(&self) -> &str;
}

impl SearchItem for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl SearchItem for String {
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a> SearchItem for std::borrow::Cow<'a, str> {
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a, S: SearchItem> SearchItem for &'a S {
    fn as_str(&self) -> &str {
        <_ as SearchItem>::as_str(*self)
    }
}

type ScoreMatrix = ndarray::Array2<f32>;

fn score(query: &str, candidate: &str, index: usize) -> Score {
    let (q, c) = match Metric::classify(query, candidate) {
        Metric::Lengths(q, c) => (q, c),
        Metric::Score(score) => return Score { index, score },
    };

    let match_bonuses = candidate_match_bonuses(candidate);

    let (mut ending, mut overall) = (ScoreMatrix::zeros((q, c)), ScoreMatrix::zeros((q, c)));

    for (i, left) in query.chars().enumerate() {
        let mut score = params::SCORE_MIN;
        let gap = if i == q - 1 {
            params::SCORE_GAP_TRAILING
        } else {
            params::SCORE_GAP_INNER
        };

        for (j, ch) in candidate.chars().enumerate() {
            if left.to_lowercase().eq(ch.to_lowercase()) {
                let res = if i == 0 {
                    (j as f32).mul_add(params::SCORE_GAP_LEADING, match_bonuses[j])
                } else if j != 0 {
                    let ending = ending[[i - 1, j - 1]] + params::SCORE_MATCH_CONSECUTIVE;
                    let overall = overall[[i - 1, j - 1]] + match_bonuses[j];
                    overall.max(ending)
                } else {
                    params::SCORE_MIN
                };

                score = score.max(res + gap);
                (ending[[i, j]], overall[[i, j]]) = (score, score);
            } else {
                score += gap;
                (ending[[i, j]], overall[[i, j]]) = (params::SCORE_MIN, score);
            }
        }
    }

    Score {
        index,
        score: overall[[q - 1, c - 1]],
    }
}

enum Metric {
    Lengths(usize, usize),
    Score(f32),
}

impl Metric {
    fn classify(query: &str, candidate: &str) -> Self {
        if candidate.len() > params::CANDIDATE_MAX_BYTES || query.is_empty() {
            return Self::Score(params::SCORE_MIN);
        }

        let q = query.chars().count();
        let c = candidate.chars().count();

        if q == c {
            return Self::Score(params::SCORE_MAX);
        }

        if c > params::CANDIDATE_MAX_CHARS {
            return Self::Score(params::SCORE_MIN);
        }

        Self::Lengths(q, c)
    }
}

fn candidate_match_bonuses(candidate: &str) -> Vec<f32> {
    let mut old = '/';
    candidate
        .chars()
        .map(|ch| {
            let s = character_match_bonus(ch, old);
            old = ch;
            s
        })
        .collect()
}

#[inline(always)]
fn character_match_bonus(current: char, previous: char) -> f32 {
    if current.is_uppercase() && previous.is_lowercase() {
        return params::SCORE_MATCH_CAPITAL;
    }

    match previous {
        '/' => params::SCORE_MATCH_SLASH,
        '.' => params::SCORE_MATCH_DOT,
        _ if is_separator(previous) => params::SCORE_MATCH_WORD,
        _ => 0.0,
    }
}

#[inline(always)]
const fn is_separator(ch: char) -> bool {
    matches!(ch, ' ' | '-' | '_')
}

mod params {
    pub const SCORE_MIN: f32 = f32::NEG_INFINITY;
    pub const SCORE_MAX: f32 = f32::INFINITY;

    pub const SCORE_GAP_LEADING: f32 = -0.005;
    pub const SCORE_GAP_INNER: f32 = -0.01;
    pub const SCORE_GAP_TRAILING: f32 = -0.005;

    pub const SCORE_MATCH_CONSECUTIVE: f32 = 1.0;
    pub const SCORE_MATCH_SLASH: f32 = 0.9;
    pub const SCORE_MATCH_WORD: f32 = 0.8;
    pub const SCORE_MATCH_CAPITAL: f32 = 0.7;
    pub const SCORE_MATCH_DOT: f32 = 0.6;

    pub const CANDIDATE_MAX_BYTES: usize = 2048;
    pub const CANDIDATE_MAX_CHARS: usize = 1024;
}
