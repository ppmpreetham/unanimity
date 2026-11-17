//! Downloads from a local server and compares the file byte for byte.

use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use unanimity::{Lane, Uplink, download};

/// Larger than two chunks, with a remainder, thus the last span is short.
const SIZE: usize = (9 << 20) + 123;

type Outcome = Result<(), Box<dyn Error>>;

fn body() -> Vec<u8> {
    (0..SIZE)
        .map(|index| index.to_le_bytes()[index % 3])
        .collect()
}

/// Answers one request. Honors `Range` only when `ranged` is true.
async fn answer(mut socket: TcpStream, ranged: bool) -> Outcome {
    let mut request = [0; 2048];
    let length = socket.read(&mut request).await?;
    let request = String::from_utf8_lossy(&request[..length]).to_ascii_lowercase();
    let span = request
        .split("range: bytes=")
        .nth(1)
        .filter(|_| ranged)
        .and_then(|range| range.lines().next()?.split_once('-'))
        .and_then(|(start, end)| Some(start.parse().ok()?..=end.parse().ok()?));
    let (body, head) = (body(), "Connection: close\r\nContent-Length:");
    let (head, part) = match span {
        Some(span) => (
            format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{SIZE}\r\n{head} {}\r\n\r\n",
                span.start(),
                span.end(),
                span.clone().count()
            ),
            &body[span],
        ),
        None => (
            format!("HTTP/1.1 200 OK\r\n{head} {SIZE}\r\n\r\n"),
            &body[..],
        ),
    };
    socket.write_all(head.as_bytes()).await?;
    socket.write_all(part).await?;
    Ok(socket.shutdown().await?)
}

async fn origin(ranged: bool) -> Result<SocketAddr, Box<dyn Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(async move { answer(socket, ranged).await.is_ok() });
        }
    });
    Ok(address)
}

async fn fetches_the_exact_bytes(ranged: bool) -> Outcome {
    let loopback = Uplink {
        label: "loopback".into(),
        device: "lo".into(),
        ip: Ipv4Addr::LOCALHOST,
    };
    let lanes = [Lane::direct(&loopback)?, Lane::direct(&loopback)?];
    let directory = tempfile::tempdir()?;
    let url = format!("http://{}/data.bin", origin(ranged).await?).parse()?;
    let path = download(url, Some(directory.path().into()), &lanes, |_| ()).await?;
    assert_eq!(path, directory.path().join("data.bin"));
    assert!(tokio::fs::read(&path).await? == body(), "bytes differ");
    assert!(!path.with_added_extension("part").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn stitches_ranges_from_many_lanes() -> Outcome {
    fetches_the_exact_bytes(true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn falls_back_to_one_stream_without_range_support() -> Outcome {
    fetches_the_exact_bytes(false).await
}
