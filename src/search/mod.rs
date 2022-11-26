use std::cmp::Ordering;
use std::usize;

use crate::score::{has_match, locate_inner, score_inner, LocateResult, ScoreResult};

/// Collection of scores and the candidates they apply to
pub type ScoreResults = Vec<ScoreResult>;
/// Collection of scores, locations, and the candidates they apply to
pub type LocateResults = Vec<LocateResult>;

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

impl<'a, S: SearchItem> SearchItem for &'a S {
    fn as_str(&self) -> &str {
        <_ as SearchItem>::as_str(*self)
    }
}

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first)
pub fn search_score<T: SearchItem>(query: &str, candidates: &[T]) -> ScoreResults {
    search_internal(query, candidates, score_inner)
}

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first) with the locations
/// of the query in each candidate
pub fn search_locate<T: SearchItem>(query: &str, candidates: &[T]) -> LocateResults {
    search_internal(query, candidates, locate_inner)
}

fn search_internal<T, S>(
    query: &str,
    candidates: &[S],
    search_fn: fn(&str, &str, usize) -> T,
) -> Vec<T>
where
    T: PartialOrd + Sized + Send + 'static,
    S: SearchItem,
{
    let mut out = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| has_match(query, c))
        .fold(Vec::with_capacity(candidates.len()), |mut a, (i, c)| {
            a.push(search_fn(query, c.as_str(), i));
            a
        });

    out.sort_unstable_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Less));
    out
}
