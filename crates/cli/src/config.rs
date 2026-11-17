//! Every tunable value of the command line interface.

use std::time::Duration;

/// Progress line of the full file.
pub(crate) const TOTAL_TEMPLATE: &str =
    "{prefix:.bold} {wide_bar:.cyan/dim} {bytes}/{total_bytes}  {bytes_per_sec:.green}  eta {eta}";
/// Progress line of one lane.
pub(crate) const LANE_TEMPLATE: &str =
    "  {spinner:.cyan} {prefix:<24} {bytes:>11}  {bytes_per_sec:>12.green}  {msg:.red}";
/// Status line of a peer that shares its internet.
pub(crate) const SHARE_TEMPLATE: &str = "  {spinner:.cyan} lent {bytes}  {msg:.dim}";
/// Characters of the progress bar: done, head, to do.
pub(crate) const BAR_CHARS: &str = "━╸─";
/// Interval at which the progress lines refresh.
pub(crate) const TICK: Duration = Duration::from_millis(120);
