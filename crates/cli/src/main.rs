//! Command line interface for unanimity.

mod config;
mod get;
mod share;

use std::error::Error;
use std::iter::successors;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;
use unanimity::{DEFAULT_PORT, Peers, Url, uplinks};

use self::get::get;
use self::share::share;

/// Download one file over every network you and your friends have.
#[derive(Parser)]
#[command(version)]
enum Command {
    /// Download a file through all networks of this device and all peers
    Get {
        /// Address of the file
        url: Url,
        /// File or directory to write to
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Pairing code of the peers, as `unanimity share` shows it
        #[arg(short, long, env = "UNANIMITY_CODE")]
        code: Option<u32>,
        /// Address of a peer that discovery does not find, as IP:PORT
        #[arg(short, long, requires = "code")]
        peer: Vec<SocketAddr>,
    },
    /// Lend the internet of this device to a download on another device
    Share {
        /// Pairing code to use, random if absent
        #[arg(short, long, env = "UNANIMITY_CODE")]
        code: Option<u32>,
        /// Port to listen on
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
    /// List the networks that this device can download through
    Networks,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    match Command::parse() {
        Command::Get {
            url,
            output,
            code,
            peer,
        } => get(url, output, code.map(|code| Peers { code, known: peer })).await,
        Command::Share { code, port } => share(code, port).await,
        Command::Networks => {
            uplinks().for_each(|uplink| println!("{:<24} {}", uplink.label, uplink.ip));
            Ok(())
        }
    }
}

/// Finds the first cause of `error`, which is the part a user can act on.
fn root_cause<'a>(error: &'a (dyn Error + 'static)) -> &'a (dyn Error + 'static) {
    successors(Some(error), |&error| error.source())
        .last()
        .unwrap_or(error)
}
