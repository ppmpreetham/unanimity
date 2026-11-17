//! Downloads one file through many lanes.

mod ranged;

use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};

use futures_util::future::select_ok;
use percent_encoding::percent_decode_str;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_RANGE, RANGE};
use reqwest::{Response, StatusCode, Url};
use tokio::fs::{File, rename};
use tokio::io::AsyncWriteExt;

use self::ranged::stitch;
use super::config::{FALLBACK_NAME, PART_EXTENSION};
use super::error::Error;
use super::lane::Lane;

/// A step of a download, for the user interface.
#[derive(Debug)]
pub enum Event<'a> {
    /// The server answered. `size` is absent when the server does not tell it.
    Started {
        /// Where the file goes.
        path: &'a Path,
        /// Number of bytes in the file.
        size: Option<u64>,
    },
    /// A lane wrote bytes to the file.
    Received {
        /// Index of the lane in the slice given to [`download`].
        lane: usize,
        /// Number of bytes written.
        bytes: u64,
    },
    /// A lane failed on a part of the file. Another stream takes the part.
    Failed {
        /// Index of the lane in the slice given to [`download`].
        lane: usize,
        /// Why the lane failed.
        error: &'a Error,
    },
}

/// Downloads `url` through all `lanes` and returns the path of the file.
///
/// `target` is a file path, or a directory in which the file keeps the name
/// that the server gives. The lanes share the work when the server supports
/// range requests. If not, the lane that answers first downloads all of it.
///
/// # Errors
/// Returns [`Error::NoLanes`] when `lanes` is empty, [`Error::Http`] when no
/// lane reaches the server, [`Error::Stalled`] when all lanes fail before the
/// end, and [`Error::Io`] when the file cannot be written.
pub async fn download(
    url: Url,
    target: Option<PathBuf>,
    lanes: &[Lane],
    notify: impl Fn(Event<'_>),
) -> Result<PathBuf, Error> {
    if lanes.is_empty() {
        return Err(Error::NoLanes);
    }
    let (lane, response) = probe(lanes, &url).await?;
    let disposition = response.headers().get(CONTENT_DISPOSITION);
    let name = file_name(disposition.and_then(|v| v.to_str().ok()), response.url());
    let path = match target {
        Some(directory) if directory.is_dir() => directory.join(name),
        Some(file) => file,
        None => name,
    };
    let part = path.with_added_extension(PART_EXTENSION);
    match (response.status(), total_size(&response)) {
        (StatusCode::PARTIAL_CONTENT, Some(size)) => {
            notify(Event::Started {
                path: &path,
                size: Some(size),
            });
            stitch(response.url(), &part, size, lanes, &notify).await
        }
        (StatusCode::OK, _) => {
            notify(Event::Started {
                path: &path,
                size: response.content_length(),
            });
            let mut file = File::create(&part).await?;
            drain(response, &mut file, |bytes| {
                notify(Event::Received { lane, bytes });
            })
            .await?;
            file.flush().await.map_err(Error::Io)
        }
        (status, _) => Err(Error::Unexpected(status)),
    }?;
    rename(&part, &path).await?;
    Ok(path)
}

/// Asks all lanes for the first byte and keeps the answer that comes first.
async fn probe(lanes: &[Lane], url: &Url) -> Result<(usize, Response), Error> {
    let attempts = lanes.iter().enumerate().map(|(index, lane)| {
        Box::pin(async move {
            let request = lane.client.get(url.clone()).header(RANGE, "bytes=0-0");
            request.send().await?.error_for_status().map(|r| (index, r))
        })
    });
    Ok(select_ok(attempts).await?.0)
}

/// Reads the full size from a `Content-Range: bytes 0-0/size` header.
fn total_size(response: &Response) -> Option<u64> {
    let range = response.headers().get(CONTENT_RANGE)?.to_str().ok()?;
    range.rsplit('/').next()?.parse().ok()
}

/// Selects a safe file name from the `Content-Disposition` header or the URL.
fn file_name(disposition: Option<&str>, url: &Url) -> PathBuf {
    disposition
        .and_then(|value| value.split("filename=").nth(1))
        .map(|name| Cow::Borrowed(name.split(';').next().unwrap_or(name).trim_matches('"')))
        .or_else(|| {
            let segment = url.path_segments()?.next_back()?;
            Some(percent_decode_str(segment).decode_utf8_lossy())
        })
        .and_then(|name| Path::new(name.as_ref()).file_name().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(FALLBACK_NAME))
}

/// Writes the body of `response` at the cursor of `file`.
async fn drain(
    mut response: Response,
    file: &mut File,
    mut received: impl FnMut(u64),
) -> Result<(), Error> {
    while let Some(bytes) = response.chunk().await? {
        file.write_all(&bytes).await?;
        received(u64::try_from(bytes.len()).map_err(io::Error::other)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PathBuf, Url, file_name};

    fn name(disposition: Option<&str>, url: &str) -> Option<PathBuf> {
        Url::parse(url).ok().map(|url| file_name(disposition, &url))
    }

    #[test]
    fn header_name_wins_over_url() {
        let header = Some(r#"attachment; filename="a b.zip"; filename*=UTF-8''x"#);
        assert_eq!(name(header, "https://h/x.bin"), Some("a b.zip".into()));
    }

    #[test]
    fn url_name_is_decoded() {
        let url = "https://h/d/my%20file.iso?k=1";
        assert_eq!(name(None, url), Some("my file.iso".into()));
    }

    #[test]
    fn name_cannot_leave_the_directory() {
        let header = Some(r#"attachment; filename="../../etc/passwd""#);
        assert_eq!(name(header, "https://h/"), Some("passwd".into()));
        let header = Some("attachment; filename=..");
        assert_eq!(name(header, "https://h/"), Some("download".into()));
    }
}
