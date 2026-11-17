//! Every tunable value of the crate.

use std::ops::Range;
use std::time::Duration;

/// Port on which a peer listens when the user does not select one.
pub const DEFAULT_PORT: u16 = 7878;

/// Size of one unit of work that a stream takes from the queue.
pub(crate) const CHUNK_SIZE: u64 = 4 << 20;
/// Number of parallel streams on each lane.
pub(crate) const STREAMS_PER_LANE: usize = 4;
/// Consecutive failures after which a stream stops.
pub(crate) const MAX_STRIKES: u32 = 3;
/// Pause before a retry, multiplied by the number of strikes.
pub(crate) const RETRY_DELAY: Duration = Duration::from_millis(500);
/// Time limit to open a connection.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// Time limit between two reads on a connection.
pub(crate) const READ_TIMEOUT: Duration = Duration::from_secs(15);
/// Time to listen for peers on the local network.
pub(crate) const DISCOVERY_WINDOW: Duration = Duration::from_secs(2);
/// DNS-SD service type that peers announce.
pub(crate) const SERVICE_TYPE: &str = "_unanimity._tcp.local.";
/// User name of the relay handshake. The pairing code is the password.
pub(crate) const RELAY_USER: &str = "unanimity";
/// Range of the pairing codes.
pub(crate) const CODE_RANGE: Range<u32> = 100_000..1_000_000;
/// Extension of a file that is not complete.
pub(crate) const PART_EXTENSION: &str = "part";
/// File name when the server gives none.
pub(crate) const FALLBACK_NAME: &str = "download";
/// Name that the clients give to servers. Some servers refuse a client with no name.
pub(crate) const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
