//! Compteurs écrits par le cœur 1 et lus par le rapport du cœur 0. Ils
//! existent dans tous les builds, que la radio correspondante soit là ou non.

use core::sync::atomic::AtomicU32;

/// Associations, déconnexions et échecs de la station Wi-Fi.
pub static ASSOCIATED: AtomicU32 = AtomicU32::new(0);
pub static DISCONNECTED: AtomicU32 = AtomicU32::new(0);
pub static FAILED: AtomicU32 = AtomicU32::new(0);
/// Adresse IPv4 obtenue, 0 tant qu'il n'y en a pas.
pub static ADDRESS: AtomicU32 = AtomicU32::new(0);
/// Rapports d'annonces BLE reçus.
pub static ADV_REPORTS: AtomicU32 = AtomicU32::new(0);
/// Sessions de scan démarrées puis arrêtées.
pub static SCAN_CYCLES: AtomicU32 = AtomicU32::new(0);
