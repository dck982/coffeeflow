//! Politique de scan BLE partagée par la balance et la sonde SCACE.
//!
//! Le Wi-Fi et le BLE tournent ensemble : le scan ne prend jamais plus de la
//! moitié du temps radio (fenêtre = moitié de l'intervalle), et n'est jamais
//! continu au sens d'une écoute à 100 %.

/// La sonde reste des mois au tiroir : tant que la balance est connectée, la
/// chercher ne doit pas occuper la radio en continu. Un scan d'une seconde
/// toutes les dix la trouve en moins de dix secondes après son allumage.
pub const PROBE_SCAN_WINDOW_US: i64 = 1000 * 1000;
pub const PROBE_SCAN_PERIOD_US: i64 = 10 * 1000 * 1000;

/// Intervalle de scan, en ms, quand la balance manque.
pub const SCAN_INTERVAL_MS: u16 = 60;
/// Fenêtre d'écoute, en ms : la moitié de l'intervalle.
pub const SCAN_WINDOW_MS: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scan {
    Off,
    /// Recherche de la balance, tant qu'elle manque (50 % du temps radio).
    Continuous,
    /// Fenêtre d'une seconde pour la sonde.
    ProbeWindow,
}

/// Paramètres de scan à transmettre au contrôleur BLE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanParams {
    pub interval_ms: u16,
    pub window_ms: u16,
    /// Durée de la fenêtre, `None` pour « jusqu'à annulation ».
    pub duration_us: Option<i64>,
    /// Scan actif (demande de scan response) : la Lunar peut annoncer son
    /// nom dans la scan response seulement, et le contrôleur du S3 déduplique
    /// sinon par adresse.
    pub active: bool,
    pub filter_duplicates: bool,
}

impl Scan {
    pub const fn params(self) -> Option<ScanParams> {
        match self {
            Self::Off => None,
            Self::Continuous => Some(ScanParams {
                interval_ms: SCAN_INTERVAL_MS,
                window_ms: SCAN_WINDOW_MS,
                duration_us: None,
                active: true,
                filter_duplicates: false,
            }),
            // La sonde annonce son service dans le paquet d'advertising : un
            // scan passif, dédupliqué, suffit et n'émet rien.
            Self::ProbeWindow => Some(ScanParams {
                interval_ms: SCAN_INTERVAL_MS,
                window_ms: SCAN_WINDOW_MS,
                duration_us: Some(PROBE_SCAN_WINDOW_US),
                active: false,
                filter_duplicates: true,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Inputs {
    /// Balance ni connectée ni en cours de connexion.
    pub scale_wanted: bool,
    /// `scace.enabled` et sonde ni connectée ni en cours de connexion.
    pub probe_wanted: bool,
    /// Une connexion est demandée : NimBLE n'en accepte qu'une à la fois.
    pub connecting: bool,
    /// Infusion ou purge en cours.
    pub cycle_active: bool,
}

/// La balance garde le scan continu d'avant la sonde : elle conditionne
/// l'arrêt au poids. Sa recherche trouve aussi la sonde, si elle est voulue.
/// Pendant un cycle, la sonde seule ne justifie aucun scan : elle doit être
/// connectée avant l'infusion.
pub const fn decide(input: &Inputs) -> Scan {
    if input.connecting {
        return Scan::Off;
    }
    if input.scale_wanted {
        return Scan::Continuous;
    }
    if input.probe_wanted && !input.cycle_active {
        return Scan::ProbeWindow;
    }
    Scan::Off
}

/// Une fenêtre de sonde part au plus tôt une période après le début de la
/// précédente. 0 signifie qu'aucune fenêtre n'a encore été ouverte.
pub const fn probe_window_due(now_us: i64, last_window_start_us: i64) -> bool {
    last_window_start_us == 0 || now_us - last_window_start_us >= PROBE_SCAN_PERIOD_US
}
