//! Wire protocol and LAN transport for Mote.
//!
//! One transport, two roles:
//! - [`Server`] runs on the PC, answers discovery and forwards key commands.
//! - [`Client`] runs on the phone, discovers the PC and sends commands.
//!
//! Everything is plain TCP/UDP with newline-separated ASCII, so the whole
//! path can be tested with `nc`.

mod client;
mod proto;
mod server;

pub use client::{Client, ClientStatus};
pub use proto::{CONTROL_PORT, Command, Config, DISCOVERY_PORT};
pub use server::{Server, ServerStatus};
