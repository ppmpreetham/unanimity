//! Shares the byte ranges of a file between the streams of all lanes.

use std::io::SeekFrom;
use std::iter::repeat_n;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use async_channel::{Receiver, Sender, unbounded};
use futures_util::future::try_join_all;
use reqwest::header::RANGE;
use reqwest::{StatusCode, Url};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::time::sleep;

use super::super::config::{CHUNK_SIZE, MAX_STRIKES, RETRY_DELAY, STREAMS_PER_LANE};
use super::super::error::Error;
use super::super::lane::Lane;
use super::{Event, drain};

/// The state that all streams of one download share.
struct Job<'a, N> {
    url: &'a Url,
    part: &'a Path,
    queue: Receiver<Range<u64>>,
    requeue: Sender<Range<u64>>,
    /// Bytes not yet in the file. The queue closes when this is zero.
    left: AtomicU64,
    notify: &'a N,
}

/// Downloads `size` bytes of `url` into `part` through all `lanes`.
///
/// Each stream takes the next span from one queue, thus a fast lane does
/// more of the work. A span that fails goes back to the queue.
pub(super) async fn stitch(
    url: &Url,
    part: &Path,
    size: u64,
    lanes: &[Lane],
    notify: &impl Fn(Event<'_>),
) -> Result<(), Error> {
    debug_assert!(size > 0, "a satisfied range request has a size");
    File::create(part).await?.set_len(size).await?;
    let (requeue, queue) = unbounded();
    let queued = (0..size.div_ceil(CHUNK_SIZE))
        .map(|index| index * CHUNK_SIZE)
        .all(|start| {
            requeue
                .try_send(start..size.min(start + CHUNK_SIZE))
                .is_ok()
        });
    debug_assert!(queued, "the queue is open and has no limit");
    let job = Job {
        url,
        part,
        queue,
        requeue,
        left: AtomicU64::new(size),
        notify,
    };
    let streams = lanes
        .iter()
        .enumerate()
        .flat_map(|lane| repeat_n(lane, STREAMS_PER_LANE))
        .map(|(index, lane)| stream(&job, index, lane));
    try_join_all(streams).await?;
    match job.left.load(Ordering::Relaxed) {
        0 => Ok(()),
        _ => Err(Error::Stalled),
    }
}

/// Takes spans from the queue until the file is complete or the lane fails
/// too many times in sequence. Only a file error stops the other streams.
async fn stream<N: Fn(Event<'_>)>(
    job: &Job<'_, N>,
    index: usize,
    lane: &Lane,
) -> Result<(), Error> {
    let mut file = OpenOptions::new().write(true).open(job.part).await?;
    let mut strikes = 0;
    while strikes < MAX_STRIKES
        && let Ok(mut span) = job.queue.recv().await
    {
        let outcome = fetch(job, index, lane, &mut file, &mut span).await;
        file.flush().await?;
        match outcome {
            Ok(()) => strikes = 0,
            Err(Error::Io(error)) => return Err(error.into()),
            Err(error) => {
                strikes += 1;
                (job.notify)(Event::Failed {
                    lane: index,
                    error: &error,
                });
                // the bytes that did arrive stay, only the remainder goes back
                let requeued = job.requeue.try_send(span).is_ok();
                debug_assert!(requeued, "the queue is open while bytes are left");
                sleep(RETRY_DELAY * strikes).await;
            }
        }
        if job.left.load(Ordering::Relaxed) == 0 {
            job.queue.close();
        }
    }
    Ok(())
}

/// Downloads `span` into `file` and moves the start of `span` forward by
/// each byte that it writes.
async fn fetch<N: Fn(Event<'_>)>(
    job: &Job<'_, N>,
    index: usize,
    lane: &Lane,
    file: &mut File,
    span: &mut Range<u64>,
) -> Result<(), Error> {
    let range = format!("bytes={}-{}", span.start, span.end - 1);
    let request = lane.client.get(job.url.clone()).header(RANGE, range);
    let response = request.send().await?;
    if response.status() != StatusCode::PARTIAL_CONTENT
        || response.content_length() != Some(span.end - span.start)
    {
        return Err(Error::Unexpected(response.status()));
    }
    file.seek(SeekFrom::Start(span.start)).await?;
    drain(response, file, |bytes| {
        span.start += bytes;
        job.left.fetch_sub(bytes, Ordering::Relaxed);
        (job.notify)(Event::Received { lane: index, bytes });
    })
    .await
}
