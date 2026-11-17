//! The relay through which a peer lends its internet to a host.
//!
//! The relay speaks SOCKS5, thus the host sees it as a normal proxy and TLS
//! stays end to end between the host and the server.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use fast_socks5::server::Socks5ServerProtocol;
use fast_socks5::util::target_addr::TargetAddr;
use fast_socks5::{ReplyError, Socks5Command};
use futures_util::future::ready;
use futures_util::stream::{FuturesUnordered, StreamExt, TryStreamExt, poll_fn};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpSocket, TcpStream, lookup_host};
use tokio::select;
use tokio::time::timeout;

use super::config::{CODE_RANGE, CONNECT_TIMEOUT, RELAY_USER};
use super::discovery::announce;
use super::error::Error;
use super::platform::pin_socket;
use super::uplink::{Uplink, uplinks};

/// The end of one connection of a host, for the user interface.
#[derive(Debug)]
pub enum RelayEvent<'a> {
    /// A connection closed after it carried `bytes` to the host.
    Relayed {
        /// Address of the host.
        host: SocketAddr,
        /// Number of bytes downloaded for the host.
        bytes: u64,
    },
    /// A connection failed or was refused.
    Failed {
        /// Address of the host.
        host: SocketAddr,
        /// Why the connection failed.
        error: &'a Error,
    },
}

/// Makes a random pairing code.
#[must_use]
pub fn fresh_code() -> u32 {
    rand::random_range(CODE_RANGE)
}

/// Lends the internet of this device to each host that knows `code`, until
/// `shutdown` completes.
///
/// # Errors
/// Returns [`Error::Io`] when the port is not available or the listener
/// fails, and [`Error::Discovery`] when the relay cannot be announced.
pub async fn serve(
    port: u16,
    code: u32,
    shutdown: impl Future<Output = ()>,
    notify: impl Fn(RelayEvent<'_>),
) -> Result<(), Error> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await?;
    let announcement = announce(listener.local_addr()?.port())?;
    let notify = &notify;
    let relays = poll_fn(|context| listener.poll_accept(context).map(Some))
        .try_for_each_concurrent(None, |(socket, host)| async move {
            match relay(socket, code).await {
                Ok(bytes) => notify(RelayEvent::Relayed { host, bytes }),
                Err(error) => notify(RelayEvent::Failed {
                    host,
                    error: &error,
                }),
            }
            Ok(())
        });
    let outcome = select! {
        outcome = relays => outcome,
        () = shutdown => Ok(()),
    };
    announcement.shutdown()?;
    outcome.map_err(Error::Io)
}

/// Serves one connection and returns the number of bytes sent to the host.
async fn relay(socket: TcpStream, code: u32) -> Result<u64, Error> {
    let facing = socket.local_addr()?.ip();
    let is_paired = |user: String, pass: String| user == RELAY_USER && pass.parse() == Ok(code);
    let (protocol, command, target) = Socks5ServerProtocol::accept_password_auth(socket, is_paired)
        .await?
        .0
        .read_command()
        .await?;
    match open(facing, &command, target).await {
        Ok(mut outbound) => {
            let mut inbound = protocol.reply_success(outbound.local_addr()?).await?;
            Ok(copy_bidirectional(&mut inbound, &mut outbound).await?.1)
        }
        Err(error) => {
            protocol.reply_error(&ReplyError::HostUnreachable).await?;
            Err(error)
        }
    }
}

/// Connects to `target` through the uplink that reaches it first. The uplink
/// that faces the host is not used, because it is the internet of the host.
async fn open(
    facing: IpAddr,
    command: &Socks5Command,
    target: TargetAddr,
) -> Result<TcpStream, Error> {
    if *command != Socks5Command::TCPConnect {
        return Err(io::Error::from(io::ErrorKind::Unsupported).into());
    }
    let address = lookup_host(target.into_string_and_port())
        .await?
        .find(is_public)
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    uplinks()
        .filter(|uplink| IpAddr::V4(uplink.ip) != facing)
        .map(|uplink| dial(uplink, address))
        .collect::<FuturesUnordered<_>>()
        .filter_map(|dialed| ready(dialed.ok()))
        .next()
        .await
        .ok_or(Error::NoUplink)
}

/// Connects to `address` from a socket that is tied to `uplink`.
async fn dial(uplink: Uplink, address: SocketAddr) -> io::Result<TcpStream> {
    let socket = TcpSocket::new_v4()?;
    socket.bind((uplink.ip, 0).into())?;
    pin_socket(&socket, &uplink.device)?;
    timeout(CONNECT_TIMEOUT, socket.connect(address)).await?
}

/// Tells if `address` is on the internet and not in a network of the peer.
fn is_public(address: &SocketAddr) -> bool {
    matches!(address.ip(), IpAddr::V4(ip) if !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()))
}
