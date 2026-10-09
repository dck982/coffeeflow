//! Découverte des services legacy/modernes Acaia et de la sonde SCACE.
use super::*;
// Acaia Lunar (ble_scale.cpp). Service et caractéristique legacy, confirmés
// sur ACAIAL-C1EF4C9 ; repli sur le service moderne des Lunar récentes, à
// canaux d'écriture et de notification distincts. UUID 128 bits dans
// l'ordre de l'air.
pub(super) const ACAIA_LEGACY_SERVICE: Uuid = Uuid::new_short(0x1820);
pub(super) const ACAIA_LEGACY_CHAR: Uuid = Uuid::new_short(0x2a80);
pub(super) const ACAIA_SERVICE: Uuid = Uuid::new_long([
    0x55, 0xe4, 0x05, 0xd2, 0xaf, 0x9f, 0xa9, 0x8f, 0xe5, 0x4a, 0x7d, 0xfe, 0x43, 0x53, 0x53, 0x49,
]);
pub(super) const ACAIA_WRITE: Uuid = Uuid::new_long([
    0xb3, 0x9b, 0x72, 0x34, 0xbe, 0xec, 0xd4, 0xa8, 0xf4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49,
]);
pub(super) const ACAIA_NOTIFY: Uuid = Uuid::new_long([
    0x16, 0x96, 0x24, 0x47, 0xc6, 0x23, 0x61, 0xba, 0xd9, 0x4b, 0x4d, 0x1e, 0x43, 0x53, 0x53, 0x49,
]);
/// Poignée d'écriture et caractéristique notifiée de la balance.
pub(super) async fn scale_discover(
    client: &Client<'_>,
) -> Result<(u16, Characteristic<[u8]>), ErrText> {
    let legacy = client
        .services_by_uuid(&ACAIA_LEGACY_SERVICE)
        .await
        .map_err(|e| err_text("service", e))?;
    if let Some(service) = legacy.first() {
        let chr: Characteristic<[u8]> = client
            .characteristic_by_uuid(service, &ACAIA_LEGACY_CHAR)
            .await
            .map_err(|e| err_text("caracteristique", e))?;
        return Ok((chr.handle, chr));
    }
    let modern = client
        .services_by_uuid(&ACAIA_SERVICE)
        .await
        .map_err(|e| err_text("service moderne", e))?;
    let service = modern
        .first()
        .ok_or_else(|| err_text("service", "absent"))?;
    let write: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_WRITE)
        .await
        .map_err(|e| err_text("ecriture", e))?;
    let notify: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_NOTIFY)
        .await
        .map_err(|e| err_text("notification", e))?;
    Ok((write.handle, notify))
}

// Sonde SCACE (common/scace_ble.hpp) : une trame de 12 octets à 10 Hz.
// UUID dans l'ordre de l'air.
pub(super) const SCACE_SERVICE: Uuid = Uuid::new_long([
    0xfe, 0x01, 0x12, 0x4b, 0xb2, 0x61, 0x2e, 0xa2, 0xd8, 0x46, 0xa7, 0x7f, 0x0a, 0x12, 0x17, 0x50,
]);
pub(super) const SCACE_FRAME: Uuid = Uuid::new_long([
    0x38, 0x9a, 0xa7, 0xae, 0x17, 0x6f, 0x21, 0xb2, 0x9a, 0x4c, 0x71, 0x11, 0x74, 0xf5, 0xfe, 0x84,
]);
