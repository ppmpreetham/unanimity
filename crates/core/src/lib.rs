//! Download one file over every network at once.
//!
//! A [`Lane`] is one path to the internet: a network of this device, or the
//! network of a peer that runs [`serve`]. [`download`] splits a file across
//! all lanes and stitches the parts into one file.

mod config;
mod discovery;
mod download;
mod error;
mod lane;
mod platform;
mod relay;
mod uplink;

pub use config::DEFAULT_PORT;
pub use download::{Event, download};
pub use error::Error;
pub use lane::{Lane, Peers, gather};
pub use relay::{RelayEvent, fresh_code, serve};
pub use reqwest::Url;
pub use uplink::{Uplink, uplinks};
