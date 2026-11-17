//! How a peer and a host find each other on the local network.

use std::collections::HashMap;
use std::net::SocketAddr;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::time::sleep;

use super::config::{DISCOVERY_WINDOW, SERVICE_TYPE};
use super::error::Error;

/// Announces a relay on `port` until the returned daemon shuts down.
pub(crate) fn announce(port: u16) -> Result<ServiceDaemon, Error> {
    let name = format!("unanimity-{:08x}", rand::random::<u32>());
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &name,
        &format!("{name}.local."),
        (),
        port,
        None,
    )?;
    let daemon = ServiceDaemon::new()?;
    daemon.register(service.enable_addr_auto())?;
    Ok(daemon)
}

/// Listens for announcements and returns one address for each relay.
///
/// A relay that this device can reach on many interfaces announces itself
/// on each of them. The name of the relay makes these one peer.
pub(crate) async fn find_peers() -> Result<impl Iterator<Item = SocketAddr>, Error> {
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(SERVICE_TYPE)?;
    sleep(DISCOVERY_WINDOW).await;
    daemon.shutdown()?;
    let peers: HashMap<String, SocketAddr> = events
        .try_iter()
        .filter_map(|event| match event {
            ServiceEvent::ServiceResolved(service) => service
                .get_addresses_v4()
                .into_iter()
                .next()
                .map(|ip| (service.fullname, SocketAddr::from((ip, service.port)))),
            _ => None,
        })
        .collect();
    Ok(peers.into_values())
}
