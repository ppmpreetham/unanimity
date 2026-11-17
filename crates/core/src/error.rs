//! The error type of the crate.

use std::io;

use fast_socks5::server::SocksServerError;
use reqwest::StatusCode;

/// A failure of a download, a relay, or the discovery of peers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A request to the server failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// A file or socket operation failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// The local network did not permit discovery.
    #[error("peer discovery failed")]
    Discovery(#[from] mdns_sd::Error),
    /// The host did not complete the relay handshake.
    #[error("relay handshake failed")]
    Handshake(#[from] SocksServerError),
    /// There is no network to download through.
    #[error("no usable network found")]
    NoLanes,
    /// All streams stopped before the file was complete.
    #[error("every network failed before the download completed")]
    Stalled,
    /// The server gave a status that does not fit the request.
    #[error("server answered {0} to a range request")]
    Unexpected(StatusCode),
    /// A peer has no network apart from the one it shares with the host.
    #[error("no independent uplink could reach the target")]
    NoUplink,
}
