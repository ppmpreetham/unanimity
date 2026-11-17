//! A lane is one path to the internet that can fetch a part of the file.

use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};

use reqwest::{Client, ClientBuilder, Proxy};

use super::config::{CONNECT_TIMEOUT, READ_TIMEOUT, RELAY_USER, USER_AGENT};
use super::discovery::find_peers;
use super::error::Error;
use super::platform::pin_client;
use super::uplink::{Uplink, uplinks};

/// One path to the internet.
#[derive(Debug, Clone)]
pub struct Lane {
    label: String,
    pub(crate) client: Client,
}

/// The peers that lend their internet to a download.
#[derive(Debug, Clone)]
pub struct Peers {
    /// Pairing code that the peers show.
    pub code: u32,
    /// Addresses of peers to use in addition to the discovered ones.
    pub known: Vec<SocketAddr>,
}

impl Lane {
    /// Makes a lane through a network of this device.
    ///
    /// # Errors
    /// Returns [`Error::Http`] when the client cannot start.
    pub fn direct(uplink: &Uplink) -> Result<Self, Error> {
        let builder = timed().no_proxy().local_address(IpAddr::V4(uplink.ip));
        Ok(Self {
            label: uplink.label.clone(),
            client: pin_client(builder, &uplink.device).build()?,
        })
    }

    /// Makes a lane through the internet of a peer.
    ///
    /// # Errors
    /// Returns [`Error::Http`] when the client cannot start.
    pub fn relayed(peer: SocketAddr, code: u32) -> Result<Self, Error> {
        let proxy = Proxy::all(format!("socks5h://{RELAY_USER}:{code}@{peer}"))?;
        Ok(Self {
            label: format!("peer {}", peer.ip()),
            client: timed().proxy(proxy).build()?,
        })
    }

    /// Name to show to the user.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Collects one lane for each network of this device and for each peer.
///
/// # Errors
/// Returns [`Error::Discovery`] when the search for peers cannot start, and
/// [`Error::Http`] when a client cannot start.
pub async fn gather(peers: Option<&Peers>) -> Result<Vec<Lane>, Error> {
    let direct = uplinks().map(|uplink| Lane::direct(&uplink));
    let Some(Peers { code, known }) = peers else {
        return direct.collect();
    };
    let found: BTreeSet<_> = find_peers().await?.chain(known.iter().copied()).collect();
    direct
        .chain(found.into_iter().map(|peer| Lane::relayed(peer, *code)))
        .collect()
}

fn timed() -> ClientBuilder {
    Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
}
