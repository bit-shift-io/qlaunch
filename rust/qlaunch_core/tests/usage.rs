use qlaunch_core::model::AppRef;
use qlaunch_core::usage::{Usage, UsageHit};

fn app(package: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: "Main".into(),
    }
}

/// Every app resolves to a label equal to its package, uppercased, except
/// `gone`, which resolves to nothing.
fn label_of(a: &AppRef) -> Option<String> {
    (a.package != "gone").then(|| a.package.to_uppercase())
}

fn labels(hits: &[UsageHit]) -> Vec<&str> {
    hits.iter().map(|h| h.label.as_str()).collect()
}

#[test]
fn a_first_launch_inserts_with_count_one() {
    let mut u = Usage::default();
    u.record(&app("a"), 100);
    assert_eq!(u.apps.len(), 1);
    assert_eq!(u.apps[0].count, 1);
    assert_eq!(u.apps[0].last_used_ms, 100);
}

#[test]
fn a_repeat_launch_increments_and_moves_the_time_forward() {
    let mut u = Usage::default();
    u.record(&app("a"), 100);
    u.record(&app("a"), 200);
    assert_eq!(u.apps.len(), 1);
    assert_eq!(u.apps[0].count, 2);
    assert_eq!(u.apps[0].last_used_ms, 200);
}

#[test]
fn frequent_orders_by_count_then_recency_then_label() {
    let mut u = Usage::default();
    for _ in 0..3 {
        u.record(&app("often"), 1);
    }
    u.record(&app("rare"), 9);
    u.record(&app("tie_a"), 5);
    u.record(&app("tie_a"), 5); // count 2, older
    u.record(&app("tie_b"), 7);
    u.record(&app("tie_b"), 7); // count 2, more recent
    assert_eq!(
        labels(&u.frequent(10, label_of)),
        ["OFTEN", "TIE_B", "TIE_A", "RARE"]
    );
}

#[test]
fn recent_orders_by_last_used_then_count_then_label() {
    let mut u = Usage::default();
    u.record(&app("older"), 10);
    u.record(&app("newer"), 30);
    assert_eq!(labels(&u.recent(10, label_of)), ["NEWER", "OLDER"]);
}

#[test]
fn entries_that_do_not_resolve_are_skipped() {
    let mut u = Usage::default();
    u.record(&app("gone"), 50);
    u.record(&app("here"), 10);
    assert_eq!(labels(&u.frequent(10, label_of)), ["HERE"]);
    assert_eq!(labels(&u.recent(10, label_of)), ["HERE"]);
}

#[test]
fn zero_and_oversized_limits_do_not_panic() {
    let mut u = Usage::default();
    u.record(&app("a"), 1);
    assert!(u.frequent(0, label_of).is_empty());
    assert_eq!(u.recent(99, label_of).len(), 1);
}

#[test]
fn usage_round_trips_through_json() {
    let mut u = Usage::default();
    u.record(&app("a"), 42);
    let json = serde_json::to_string(&u).unwrap();
    assert_eq!(serde_json::from_str::<Usage>(&json).unwrap(), u);
    assert_eq!(
        serde_json::from_str::<Usage>("{}").unwrap(),
        Usage::default()
    );
}
