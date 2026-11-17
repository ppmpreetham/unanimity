//! The `share` command: lend the internet of this device to a host.

use indicatif::{ProgressBar, ProgressStyle};
use tokio::signal::ctrl_c;
use unanimity::{RelayEvent, fresh_code, serve, uplinks};

use super::config::{SHARE_TEMPLATE, TICK};
use super::root_cause;

pub(crate) async fn share(code: Option<u32>, port: u16) -> anyhow::Result<()> {
    let code = code.unwrap_or_else(fresh_code);
    uplinks().for_each(|uplink| println!("sharing {} ({})", uplink.label, uplink.ip));
    println!("pairing code {code}, port {port}");
    println!("on the main device: unanimity get <URL> --code {code}");
    let status = ProgressBar::no_length().with_style(ProgressStyle::with_template(SHARE_TEMPLATE)?);
    status.enable_steady_tick(TICK);
    status.set_message("ctrl-c to stop");
    // an error of the signal handler also stops the relay, which is the safe result
    let stop = async { drop(ctrl_c().await) };
    serve(port, code, stop, |event| match event {
        RelayEvent::Relayed { host, bytes } => {
            status.inc(bytes);
            status.set_message(format!("last for {}", host.ip()));
        }
        RelayEvent::Failed { host, error } => {
            status.println(format!("{}: {}", host.ip(), root_cause(error)));
        }
    })
    .await?;
    status.finish();
    Ok(())
}
