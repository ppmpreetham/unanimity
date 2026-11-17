//! Systems on which a socket must name its device.

use std::io;

use reqwest::ClientBuilder;
use tokio::net::TcpSocket;

/// Ties all connections of the client to `device`.
pub(crate) fn pin_client(builder: ClientBuilder, device: &str) -> ClientBuilder {
    builder.interface(device)
}

/// Ties the socket to `device`.
pub(crate) fn pin_socket(socket: &TcpSocket, device: &str) -> io::Result<()> {
    socket.bind_device(Some(device.as_bytes()))
}
