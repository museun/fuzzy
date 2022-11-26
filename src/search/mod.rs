use std::cmp::Ordering;
use std::usize;

use crate::score::{has_match, locate_inner, score_inner, LocateResult, ScoreResult};

/// Collection of scores and the candidates they apply to
pub type ScoreResults = Vec<ScoreResult>;
/// Collection of scores, locations, and the candidates they apply to
pub type LocateResults = Vec<LocateResult>;

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first)
pub fn search_score<T: AsRef<str>>(query: &str, candidates: &[T]) -> ScoreResults {
    search_internal(query, candidates, score_inner)
}

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first) with the locations
/// of the query in each candidate
pub fn search_locate<T: AsRef<str>>(query: &str, candidates: &[T]) -> LocateResults {
    search_internal(query, candidates, locate_inner)
}

fn search_internal<T, S>(
    query: &str,
    candidates: &[S],
    search_fn: fn(&str, &str, usize) -> T,
) -> Vec<T>
where
    T: PartialOrd + Sized + Send + 'static,
    S: AsRef<str>,
{
    let mut out = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| has_match(&query, c))
        .fold(Vec::with_capacity(candidates.len()), |mut a, (i, c)| {
            a.push(search_fn(&query, c.as_ref(), 0 + i));
            a
        });

    out.sort_unstable_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Less));
    out
}
