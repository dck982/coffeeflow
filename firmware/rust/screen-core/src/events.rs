//! Journal court des événements de l'écran (page de service).

use alloc::vec::Vec;

/// Nature d'un événement publié dans le journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Boot,
    CanPresenceLost,
    CanPresenceRestored,
    WifiApStarted,
    WifiConnected,
    WifiDisconnected,
    NetworkForgotten,
    TimeKnown,
}

impl EventKind {
    /// Texte court (majuscules, pas de style) pour l'écran de service.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Boot => "BOOT",
            Self::CanPresenceLost => "CAN PERDU",
            Self::CanPresenceRestored => "CAN OK",
            Self::WifiApStarted => "AP DEMARRE",
            Self::WifiConnected => "WIFI CONNECTE",
            Self::WifiDisconnected => "WIFI PERDU",
            Self::NetworkForgotten => "RESEAU OUBLIE",
            Self::TimeKnown => "HEURE CONNUE",
        }
    }
}

/// Événement horodaté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    /// Horloge monotone en ms depuis le boot, jamais l'heure murale.
    pub uptime_ms: u32,
}

/// Capacité du journal.
pub const EVENT_LOG_CAPACITY: usize = 8;

/// Anneau de [`EVENT_LOG_CAPACITY`] événements.
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    ring: [Option<Event>; EVENT_LOG_CAPACITY],
    count: usize,
    next: usize,
}

impl EventLog {
    pub const fn new() -> Self {
        Self {
            ring: [None; EVENT_LOG_CAPACITY],
            count: 0,
            next: 0,
        }
    }

    pub fn push(&mut self, kind: EventKind, uptime_ms: u32) {
        self.ring[self.next] = Some(Event { kind, uptime_ms });
        self.next = (self.next + 1) % EVENT_LOG_CAPACITY;
        if self.count < EVENT_LOG_CAPACITY {
            self.count += 1;
        }
    }

    /// Au plus `max_count` événements, le plus récent en premier.
    pub fn recent(&self, max_count: usize) -> Vec<Event> {
        let n = self.count.min(max_count);
        (0..n)
            .filter_map(|i| {
                self.ring[(self.next + EVENT_LOG_CAPACITY - 1 - i) % EVENT_LOG_CAPACITY]
            })
            .collect()
    }
}
