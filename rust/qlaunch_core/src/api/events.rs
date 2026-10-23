use flutter_rust_bridge::frb;

use crate::model::Id;

/// What the core tells Dart happened. An event only says that something
/// changed: a subscriber that misses one re-reads `snapshot()`.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreEvent {
    ModelChanged,
    AppsChanged,
    ShortcutCreated {
        item_id: Id,
    },
    ShortcutCancelled,
    /// Reserved. The window shows the system wallpaper itself, so nothing needs
    /// to re-read it today; the variant stays so the event list still matches
    /// the spec and a future reader (a live wallpaper, say) can emit it.
    WallpaperChanged,
    HomePressed,
    /// Home pressed while a system overlay (the notification shade, say) had
    /// focus. The press is consumed by dismissing that overlay, so it may
    /// return to the launcher but must not advance the usual Home sequence.
    HomePressedFromOverlay,
    BackPressed,
    Error(String),
}

type Deliver = Box<dyn Fn(&CoreEvent) -> bool + Send + Sync>;

/// Delivers events to every registered subscriber, in the order they
/// subscribed.
///
/// A subscriber's closure returns `false` once it is gone (a closed Dart
/// stream), and `emit` then drops it, so a subscriber that went away does not
/// stay in the list for good. Delivery runs on the caller's thread while the
/// global state lock is held, so a closure must not call back into the API.
#[frb(ignore)]
pub struct EventBus {
    next_token: u64,
    subscribers: Vec<(u64, Deliver)>,
}

impl EventBus {
    pub const fn new() -> Self {
        Self {
            next_token: 0,
            subscribers: Vec::new(),
        }
    }

    /// Registers a subscriber. Its closure returns `false` once it is gone,
    /// and is dropped then. Returns a token for [`EventBus::unsubscribe`].
    pub fn subscribe(&mut self, deliver: Deliver) -> u64 {
        let token = self.next_token;
        self.next_token += 1;
        self.subscribers.push((token, deliver));
        token
    }

    /// Removes the subscriber `token` named. `false` if it was not there,
    /// because it had already gone or never existed.
    pub fn unsubscribe(&mut self, token: u64) -> bool {
        let before = self.subscribers.len();
        self.subscribers.retain(|(t, _)| *t != token);
        self.subscribers.len() != before
    }

    pub fn emit(&mut self, event: &CoreEvent) {
        self.subscribers.retain(|(_, deliver)| deliver(event));
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
