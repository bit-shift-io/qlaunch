use crate::model::AppRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    Prefix,
    Substring,
    Subsequence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchCandidate {
    pub app: AppRef,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub app: AppRef,
    pub label: String,
    pub kind: MatchKind,
}

pub fn search(candidates: &[SearchCandidate], query: &str) -> Vec<SearchHit> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return candidates
            .iter()
            .map(|c| hit(c, MatchKind::Prefix))
            .collect();
    }

    let mut hits: Vec<SearchHit> = candidates
        .iter()
        .filter_map(|c| {
            let label = c.label.to_lowercase();
            let kind = if label.starts_with(&query) {
                MatchKind::Prefix
            } else if label.contains(&query) {
                MatchKind::Substring
            } else if is_subsequence(&query, &label) {
                MatchKind::Subsequence
            } else {
                return None;
            };
            Some(hit(c, kind))
        })
        .collect();
    hits.sort_by_key(|h| (h.kind, h.label.len(), h.label.clone()));
    hits
}

fn hit(candidate: &SearchCandidate, kind: MatchKind) -> SearchHit {
    SearchHit {
        app: candidate.app.clone(),
        label: candidate.label.clone(),
        kind,
    }
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut wanted = needle.chars().peekable();
    for c in haystack.chars() {
        if wanted.peek() == Some(&c) {
            wanted.next();
        }
    }
    wanted.next().is_none()
}
