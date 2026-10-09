//! Protocole de coffeeflow entre l'écran et les capteurs : identifiant CAN,
//! charges utiles, cadrage série, CRC, version, trame BLE de la sonde SCACE
//! et table des codes LOG.
//!
//! Le crate est `no_std` et sans allocation : le firmware l'utilise tel
//! quel, les outils Mac ajoutent la feature `std` (et `serde` si besoin).
//! Le comportement reproduit au bit près celui du code C++ archivé
//! (`archive/firmware/common`) : octets sur le fil, longueurs (DLC) émises,
//! formats anciens encore acceptés en réception.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

pub mod crc;
pub mod frame;
pub mod framing;
pub mod id;
pub mod log_codes;
pub mod messages;
pub mod scace;
pub mod version;

pub use crc::{Crc32Incremental, crc16_ccitt, crc32_ieee};
pub use frame::Frame;
pub use framing::{
    FramingError, MAX_COBS_SIZE, MAX_FRAMED_SIZE, MAX_PDU_SIZE, StreamDecoder, cobs_decode,
    cobs_encode, decode_pdu, encode_framed, encode_pdu,
};
pub use id::{
    CanId, CanIdError, Dest, MessageType, Node, decode_can_id, encode_can_id,
    is_known_message_type, split_can_id,
};
pub use log_codes::{LOG_CODES, LogCode, LogCodeInfo, LogSeverity, log_code_name};
pub use messages::{DecodeError, Message};
pub use version::{FIRMWARE_VERSION, Version, heating_protocol_supported};
