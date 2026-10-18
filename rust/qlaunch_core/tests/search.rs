use qlaunch_core::model::AppRef;
use qlaunch_core::search::{MatchKind, SearchCandidate, SearchHit, search};

fn candidate(package: &str, label: &str) -> SearchCandidate {
    SearchCandidate {
        app: AppRef {
            package: package.into(),
            activity: "a".into(),
        },
        label: label.into(),
    }
}

fn cands(labels: &[&str]) -> Vec<SearchCandidate> {
    labels.iter().map(|l| candidate("p", l)).collect()
}

fn summary(hits: &[SearchHit]) -> Vec<(&str, MatchKind)> {
    hits.iter().map(|h| (h.label.as_str(), h.kind)).collect()
}

#[test]
fn empty_or_whitespace_query_returns_every_candidate_in_input_order() {
    let cs = cands(&["Zeta", "Alpha", "Mid"]);
    let expected = vec![
        ("Zeta", MatchKind::Prefix),
        ("Alpha", MatchKind::Prefix),
        ("Mid", MatchKind::Prefix),
    ];
    assert_eq!(summary(&search(&cs, "")), expected);
    assert_eq!(summary(&search(&cs, "  \t ")), expected);
}

#[test]
fn trailing_space_is_trimmed_before_matching() {
    let cs = cands(&["Files", "Firefox", "Web Browser"]);
    assert_eq!(search(&cs, "fi "), search(&cs, "fi"));
    assert_eq!(search(&cs, "fi ").len(), 2);
}

#[test]
fn matching_is_case_insensitive_in_both_directions() {
    let cs = cands(&["Firefox"]);
    assert_eq!(
        summary(&search(&cs, "fir")),
        vec![("Firefox", MatchKind::Prefix)]
    );
    let cs = cands(&["firefox"]);
    assert_eq!(
        summary(&search(&cs, "FIR")),
        vec![("firefox", MatchKind::Prefix)]
    );
}

#[test]
fn ranks_prefix_above_substring_above_subsequence() {
    let cs = cands(&[
        "Frame Icon",
        "Wifi Hotspot",
        "Firefox",
        "Web Browser",
        "Files",
    ]);
    assert_eq!(
        summary(&search(&cs, "fi")),
        vec![
            ("Files", MatchKind::Prefix),
            ("Firefox", MatchKind::Prefix),
            ("Wifi Hotspot", MatchKind::Substring),
            ("Frame Icon", MatchKind::Subsequence),
        ]
    );
}

#[test]
fn ties_within_a_kind_break_by_label_length_then_label() {
    let cs = cands(&["Firefox", "Files", "Fig", "Fiz"]);
    assert_eq!(
        summary(&search(&cs, "fi")),
        vec![
            ("Fig", MatchKind::Prefix),
            ("Fiz", MatchKind::Prefix),
            ("Files", MatchKind::Prefix),
            ("Firefox", MatchKind::Prefix),
        ]
    );
}

#[test]
fn identical_labels_and_kinds_keep_input_order() {
    let cs = vec![candidate("first", "Dup"), candidate("second", "Dup")];
    let packages: Vec<String> = search(&cs, "du")
        .into_iter()
        .map(|h| h.app.package)
        .collect();
    assert_eq!(packages, vec!["first", "second"]);
}

#[test]
fn cjk_query_matches_cjk_label() {
    let cs = cands(&["Firefox", "微信", "日本語テキスト"]);
    assert_eq!(
        summary(&search(&cs, "微")),
        vec![("微信", MatchKind::Prefix)]
    );
    assert_eq!(
        summary(&search(&cs, "信")),
        vec![("微信", MatchKind::Substring)]
    );
    assert_eq!(
        summary(&search(&cs, "日テ")),
        vec![("日本語テキスト", MatchKind::Subsequence)]
    );
}

#[test]
fn emoji_label_is_matched_by_its_emoji_without_panicking() {
    let cs = cands(&["🎵 Music", "Notes"]);
    assert_eq!(
        summary(&search(&cs, "🎵")),
        vec![("🎵 Music", MatchKind::Prefix)]
    );
    assert_eq!(
        summary(&search(&cs, "🎵m")),
        vec![("🎵 Music", MatchKind::Subsequence)]
    );
}

#[test]
fn multibyte_bytes_in_order_without_matching_chars_do_not_match() {
    // "é" is UTF-8 C3 A9. The label "ã©" is UTF-8 C3 A3 C2 A9: its bytes
    // contain C3 ... A9 in order (positions 0 and 3), so a byte-wise
    // subsequence scan would match. As characters, "é" is not in "ã©", so
    // the char-wise scan must not match.
    let cs = cands(&["ã©"]);
    assert!(search(&cs, "é").is_empty());
}

#[test]
fn query_longer_than_label_returns_nothing() {
    let cs = cands(&["ab"]);
    assert!(search(&cs, "abc").is_empty());
    assert!(search(&cs, "bba").is_empty());
}
