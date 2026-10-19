//! How often each app has been launched, and when it was last launched.
//!
//! Kept in the stored document so Frequent and Recent survive a restart. The
//! rules are deliberately simple: one count and one timestamp per app, ordered
//! for the two questions the search screen asks.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::model::AppRef;

/// Every app the user has launched, in no particular order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub apps: Vec<AppUsage>,
}

/// One app's launch count and the time of its most recent launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUsage {
    pub package: String,
    pub activity: String,
    pub count: u32,
    /// Unix milliseconds of the last launch. Only ever compared with another
    /// value from the same clock, so its epoch does not matter.
    pub last_used_ms: i64,
}

/// An app to show under Frequent or Recent: the app plus the label to draw for
/// it. The label is resolved from the current model rather than stored, so a
/// renamed app shows its new name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageHit {
    pub app: AppRef,
    pub label: String,
}

impl Usage {
    /// Records one successful launch at `now_ms`.
    pub fn record(&mut self, app: &AppRef, now_ms: i64) {
        match self
            .apps
            .iter_mut()
            .find(|u| u.package == app.package && u.activity == app.activity)
        {
            Some(entry) => {
                entry.count = entry.count.saturating_add(1);
                entry.last_used_ms = now_ms;
            }
            None => self.apps.push(AppUsage {
                package: app.package.clone(),
                activity: app.activity.clone(),
                count: 1,
                last_used_ms: now_ms,
            }),
        }
    }

    /// The most-launched apps, most launches first, up to `limit`.
    ///
    /// `label_of` resolves an app to the label to show; an app it does not know
    /// (one removed from the launcher, or uninstalled) is left out.
    pub fn frequent(
        &self,
        limit: usize,
        label_of: impl Fn(&AppRef) -> Option<String>,
    ) -> Vec<UsageHit> {
        self.ranked(limit, label_of, |a, b| {
            b.count
                .cmp(&a.count)
                .then(b.last_used_ms.cmp(&a.last_used_ms))
        })
    }

    /// The most-recently-launched apps, newest first, up to `limit`.
    pub fn recent(
        &self,
        limit: usize,
        label_of: impl Fn(&AppRef) -> Option<String>,
    ) -> Vec<UsageHit> {
        self.ranked(limit, label_of, |a, b| {
            b.last_used_ms
                .cmp(&a.last_used_ms)
                .then(b.count.cmp(&a.count))
        })
    }

    /// Resolves, orders (by `rank`, then label so the last tie is broken) and
    /// truncates.
    fn ranked(
        &self,
        limit: usize,
        label_of: impl Fn(&AppRef) -> Option<String>,
        rank: impl Fn(&AppUsage, &AppUsage) -> Ordering,
    ) -> Vec<UsageHit> {
        let mut hits: Vec<(AppUsage, String)> = self
            .apps
            .iter()
            .filter_map(|u| {
                let app = AppRef {
                    package: u.package.clone(),
                    activity: u.activity.clone(),
                };
                label_of(&app).map(|label| (u.clone(), label))
            })
            .collect();
        hits.sort_by(|(a, la), (b, lb)| rank(a, b).then(la.cmp(lb)));
        hits.into_iter()
            .take(limit)
            .map(|(u, label)| UsageHit {
                app: AppRef {
                    package: u.package,
                    activity: u.activity,
                },
                label,
            })
            .collect()
    }
}
