//! Annulation d'une session de scan déjà ouverte : la politique peut changer
//! pendant l'attente des annonces. Le scanner doit alors libérer le contrôleur.
use screen_core::ble_policy::{self, Inputs, Scan};

pub fn scan_continues(
    policy: Scan,
    inputs: &Inputs,
    enabled: bool,
    now_us: i64,
    start_us: i64,
) -> bool {
    enabled
        && ble_policy::decide(inputs) == policy
        && (policy != Scan::ProbeWindow || now_us - start_us < ble_policy::PROBE_SCAN_WINDOW_US)
}

/// Revérifier juste avant les commandes HCI, sans attente entre test et drapeau.
pub fn scan_can_start(inputs: &Inputs, enabled: bool) -> bool {
    enabled && ble_policy::decide(inputs) != Scan::Off
}
/// Une échéance fixe reste due même si toutes les notifications sont prêtes.
pub fn heartbeat_poll_due(now_us: u64, deadline_us: u64) -> bool {
    now_us >= deadline_us
}
