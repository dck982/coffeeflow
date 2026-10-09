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
