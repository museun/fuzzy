mod score;
pub use score::Score;

mod search_item;
pub use search_item::SearchItem;

pub fn fuzzy_search<T>(query: &str, candidates: &[T]) -> Vec<Score>
where
    T: crate::SearchItem,
{
    let mut out = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let mut cand_iter = c.as_str().chars();
            query
                .chars()
                .all(|c| cand_iter.any(|c2| c2.to_lowercase().eq(c.to_lowercase())))
        })
        .fold(Vec::with_capacity(candidates.len()), |mut a, (i, c)| {
            a.push(crate::score::score(query, c.as_str(), i));
            a
        });

    out.sort_unstable_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Less));
    out
}

struct Params;
impl Params {
    pub(crate) const SCORE_MIN: f32 = f32::NEG_INFINITY;
    pub(crate) const SCORE_MAX: f32 = f32::INFINITY;

    pub(crate) const SCORE_GAP_LEADING: f32 = -0.005;
    pub(crate) const SCORE_GAP_INNER: f32 = -0.01;
    pub(crate) const SCORE_GAP_TRAILING: f32 = -0.005;

    pub(crate) const SCORE_MATCH_CONSECUTIVE: f32 = 1.0;
    pub(crate) const SCORE_MATCH_SLASH: f32 = 0.9;
    pub(crate) const SCORE_MATCH_WORD: f32 = 0.8;
    pub(crate) const SCORE_MATCH_CAPITAL: f32 = 0.7;
    pub(crate) const SCORE_MATCH_DOT: f32 = 0.6;

    pub(crate) const CANDIDATE_MAX_BYTES: usize = 2048;
    pub(crate) const CANDIDATE_MAX_CHARS: usize = 1024;
}
