//! SHT40 du boîtier de l'écran : conversion des codes bruts de la mesure à
//! haute précision en °C et en % d'humidité relative.

/// Température en °C : -45 + 175 × code / 65535.
pub fn temperature_c(raw: u16) -> f32 {
    -45.0 + 175.0 * f32::from(raw) / 65535.0
}

/// Humidité relative en % : -6 + 125 × code / 65535, bornée à 0..100 (le
/// capteur peut sortir un peu de la plage physique).
pub fn humidity_pct(raw: u16) -> f32 {
    (-6.0 + 125.0 * f32::from(raw) / 65535.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_formula() {
        assert_eq!(temperature_c(0), -45.0);
        assert!((temperature_c(65535) - 130.0).abs() < 1e-3);
        // Mesure du banc : 24,56 °C.
        assert!((temperature_c(26_051) - 24.56).abs() < 0.01);
    }

    #[test]
    fn humidity_formula_and_bounds() {
        // Sous -6 % + 6 % la formule donne 0 : le code 0 est borné.
        assert_eq!(humidity_pct(0), 0.0);
        assert_eq!(humidity_pct(65535), 100.0);
        // Mesure du banc : 42,9 %.
        assert!((humidity_pct(25_638) - 42.9).abs() < 0.1);
        assert!((humidity_pct(32_768) - 56.5).abs() < 0.1);
    }
}
