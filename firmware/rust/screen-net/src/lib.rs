//! Formats de transport réseau testables sans radio : requêtes HTTP bornées,
//! credentials NVS compatibles IDF, handshake WebSocket et réponse SNTP.
#![no_std]
extern crate alloc;

pub mod credentials;
pub mod http;
pub mod sntp;
pub mod websocket;
