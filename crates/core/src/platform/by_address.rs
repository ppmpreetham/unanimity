//! Systems on which the source address is sufficient to select the interface.

use std::io;

use reqwest::ClientBuilder;
use tokio::net::TcpSocket;

/// Does nothing: the source address selects the interface.
pub(crate) fn pin_client(builder: ClientBuilder, _device: &str) -> ClientBuilder {
    builder
}

/// Does nothing: the source address selects the interface.
#[expect(clippy::unnecessary_wraps, reason = "same contract as by_device")]
pub(crate) fn pin_socket(_socket: &TcpSocket, _device: &str) -> io::Result<()> {
    Ok(())
}
