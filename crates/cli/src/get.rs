//! The `get` command: download a file and show the progress of each lane.

use std::path::PathBuf;

use indicatif::{HumanBytes, HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use unanimity::{Event, Peers, Url, download, gather};

use super::config::{BAR_CHARS, LANE_TEMPLATE, TICK, TOTAL_TEMPLATE};
use super::root_cause;

pub(crate) async fn get(
    url: Url,
    output: Option<PathBuf>,
    peers: Option<Peers>,
) -> anyhow::Result<()> {
    let lanes = gather(peers.as_ref()).await?;
    let board = MultiProgress::new();
    let total_style = ProgressStyle::with_template(TOTAL_TEMPLATE)?.progress_chars(BAR_CHARS);
    let total = board.add(ProgressBar::no_length().with_style(total_style));
    let lane_style = ProgressStyle::with_template(LANE_TEMPLATE)?;
    let bars: Vec<_> = lanes
        .iter()
        .map(|lane| ProgressBar::hidden().with_prefix(lane.label().to_owned()))
        .map(|bar| bar.with_style(lane_style.clone()))
        .collect();
    total.enable_steady_tick(TICK);
    let outcome = download(url, output, &lanes, |event| match event {
        Event::Started { path, size } => {
            total.set_prefix(path.display().to_string());
            total.set_length(size.unwrap_or_default());
            total.reset_elapsed();
        }
        Event::Received { lane, bytes } => {
            total.inc(bytes);
            bars.get(lane).into_iter().for_each(|bar| {
                // a lane gets a line with its first bytes, thus a dead network stays out of view
                if bar.position() == 0 {
                    board.add(bar.clone()).enable_steady_tick(TICK);
                }
                bar.inc(bytes);
            });
        }
        Event::Failed { lane, error } => bars
            .get(lane)
            .into_iter()
            .for_each(|bar| bar.set_message(root_cause(error).to_string())),
    })
    .await;
    bars.iter()
        .chain([&total])
        .for_each(ProgressBar::finish_and_clear);
    // a lane that had no work and no failure has nothing to tell
    for bar in bars
        .iter()
        .filter(|bar| bar.position() > 0 || !bar.message().is_empty())
    {
        let bytes = HumanBytes(bar.position()).to_string();
        println!("  {:<24} {bytes:>11}  {}", bar.prefix(), bar.message());
    }
    println!(
        "saved {} ({} in {})",
        outcome?.display(),
        HumanBytes(total.position()),
        HumanDuration(total.elapsed())
    );
    Ok(())
}
