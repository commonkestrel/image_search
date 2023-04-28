//! `image_search::blocking` is an optional feature that contains all the original capabilities of the non-blocking counterpart, but synchronous instead of async.
//! Similar to [`reqwest`](https://crates.io/crates/reqwest)'s blocking feature

extern crate glob;
extern crate surf;
extern crate infer;
extern crate futures;
extern crate async_std;
extern crate serde_json;

use crate::{get, Arguments, DownloadError, Error, Image, SearchResult};
use futures::future;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Search for images based on the provided arguments and return images up to the provided limit.
///
/// # Errors
/// This function will return an error if:
/// * The GET request fails
/// * The images are not able to be parsed
///
/// # Examples
///
/// ```
/// extern crate image_search;
///
/// use image_search::Arguments;
/// use image_search::blocking::search;
///
/// fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("cats", 10);
///     let images = search(args)?;
///
///     Ok(())
/// }
pub fn search(args: Arguments) -> SearchResult<Vec<Image>> {
    let url = build_url(&args);

    let body = async_std::task::block_on(get(url))?;

    let imgs = unpack(body).ok_or(Error::Parse)?;

    if imgs.len() > args.limit && args.limit > 0 {
        Ok(imgs[..args.limit].to_vec())
    } else {
        Ok(imgs)
    }
}

/// Search for images based on the provided arguments and return the urls of the images
///
/// # Errors
/// This function will return an error if:
/// * The GET request fails
/// * The images are not able to be parsed
///
/// # Examples
///
/// ```
/// extern crate image_search;
///
/// use image_search::Arguments;
/// use image_search::blocking::urls;
///
/// fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("cats", 10);
///     let images = urls(args)?;
///
///     Ok(())
/// }
pub fn urls(args: Arguments) -> SearchResult<Vec<String>> {
    let thumbnails = (&args.thumbnails).to_owned();
    let images = search(args)?;

    let mut all: Vec<String> = Vec::new();
    for image in images.iter() {
        if thumbnails {
            all.push(image.thumbnail.to_owned());
        } else {
            all.push(image.url.to_owned());
        }
    }

    Ok(all)
}

/// Search for images based on the provided `Arguments` and downloads them to the path specified in the `directory` field in `Arguments`, or the "images" folder if none is provided.
/// # Errors
/// This function will return an error if:
/// * The GET request fails
/// * The images are not able to be parsed
/// * The program is unable to create/read/write to files or directories
///
/// # Examples
///
/// ```
/// extern crate image_search
///
/// use image_search::Arguments;
/// use image_search::blocking::download;
/// use std::path::Path;
///
/// fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("cats", 10).directory(Path::new("downloads"));
///     let paths = download(args)?;
///
///     Ok(())
/// }
pub fn download(args: Arguments) -> SearchResult<Vec<PathBuf>> {
    let images = urls(Arguments {
        query: args.query.clone(),
        limit: 0,
        directory: args.directory.clone(),
        ..args
    })?;

    let dir = match args.directory {
        Some(dir) => dir.to_owned(),
        None => match env::current_dir() {
            Ok(v) => v,
            Err(e) => return Err(Error::Dir(e)),
        }
        .join("images"),
    };

    match std::fs::create_dir_all(&dir) {
        Ok(_) => (),
        Err(e) => return Err(Error::Dir(e)),
    };

    let mut suffix = 0;
    let mut paths: Vec<PathBuf> = Vec::new();
    for _ in 0..args.limit {
        let mut path = dir.join(args.query.to_owned() + &suffix.to_string());

        let mut matches = match glob::glob(&(path.display().to_string() + ".*")) {
            Ok(paths) => paths.last().is_some(),
            Err(_) => false,
        };

        while matches {
            suffix += 1;
            path = dir.join(args.query.to_owned() + &suffix.to_string());
            matches = match glob::glob(&(path.display().to_string() + ".*")) {
                Ok(paths) => paths.last().is_some(),
                Err(_) => false,
            };
        }

        paths.push(path);
        suffix += 1;
    }

    let with_extensions = async_std::task::block_on(download_n(images, paths, args.timeout));

    Ok(with_extensions)
}

/// Downloads up to n images concurrently
async fn download_n(
    urls: Vec<String>,
    paths: Vec<PathBuf>,
    timeout: Option<Duration>,
) -> Vec<PathBuf> {
    let mut_urls = Arc::new(Mutex::new(urls));

    let mut downloaders = Vec::new();
    let client = surf::Client::new();
    for path in paths {
        downloaders.push(download_until(
            mut_urls.clone(),
            path,
            client.clone(),
            timeout,
        ));
    }

    let with_extensions = future::join_all(downloaders)
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect();

    with_extensions
}

macro_rules! next_available {
    ($urls:expr) => {{
        let mut mut_urls = $urls.lock().unwrap(); // Safe: no thread should panic while holding, since this is the only unwrap/expect
        if mut_urls.is_empty() {
            return Err(DownloadError::Overflow);
        }
        let url = mut_urls.remove(0);
        std::mem::drop(mut_urls);

        url
    }};
}

/// Trys to download an image to a given path until one is successful or it runs out of possible urls
async fn download_until(
    urls: Arc<Mutex<Vec<String>>>,
    path: PathBuf,
    client: surf::Client,
    timeout: Option<Duration>,
) -> Result<PathBuf, DownloadError> {
    let mut url = next_available!(urls);

    let with_extension = loop {
        let with_extension = download_image(client.clone(), &path, url.to_owned(), timeout).await;
        if with_extension.is_ok() {
            break with_extension;
        }
        url = next_available!(urls);
    };

    with_extension
}

async fn download_image(
    client: surf::Client,
    path: &PathBuf,
    url: String,
    timeout: Option<Duration>,
) -> Result<PathBuf, DownloadError> {
    let buf = match timeout {
        Some(duration) => {
            async_std::future::timeout(duration, client.recv_bytes(surf::get(url))).await?
        }
        None => client.recv_bytes(surf::get(url)).await,
    }?;

    let first_128 = buf.iter().take(1024).map(|x| *x).collect::<Vec<u8>>();
    let svg = match std::str::from_utf8(&first_128) {
        Ok(s) => s.contains("<svg"),
        Err(_) => false,
    };

    let extension = if svg {
        "svg".to_owned()
    } else {
        let kind = match infer::get(&buf) {
            Some(k) => k,
            None => return Err(DownloadError::Extension),
        };

        if kind.matcher_type() != infer::MatcherType::Image {
            return Err(DownloadError::Extension);
        }

        kind.extension().to_owned()
    };

    let with_extension = path.clone().with_extension(extension);

    let mut f = match File::create(&with_extension) {
        Ok(f) => f,
        Err(e) => return Err(DownloadError::Fs(e)),
    };

    match f.write_all(&buf) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::Fs(e)),
    };

    Ok(with_extension)
}

fn build_url(args: &Arguments) -> String {
    let mut url = "https://www.google.com/search?tbm=isch&q=".to_string() + &args.query;

    let params = args.params();
    if params.len() > 0 {
        url += &"&tbs=ic:specific".to_string();
        url += &params;
    }

    url
}

/// shorthand for unwrap_or_continue
macro_rules! uoc {
    ($opt: expr) => {
        match $opt {
            Some(v) => v,
            None => {
                continue;
            }
        }
    };
}

fn unpack(mut body: String) -> Option<Vec<Image>> {
    let script = body.rfind("AF_initDataCallback")?;
    body = body[script..].to_string();

    let start = body.find("[")?;
    body = body[start..].to_string();

    let script_end = body.find("</script>")?;
    body = body[..script_end].to_string();

    let end = body.rfind(",")?;
    body = body[..end].to_string();

    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(j) => j,
        Err(_) => return None,
    };

    let image_objects = json.as_array()?[56].as_array()?[1].as_array()?[0]
        .as_array()?
        .last()?
        .as_array()?[1]
        .as_array()?[0]
        .as_array()?;

    let mut images: Vec<Image> = Vec::new();
    for obj in image_objects.iter() {
        let inner = uoc!(uoc!(
            uoc!(uoc!(uoc!(obj.as_array())[0].as_array())[0].as_object())["444383007"].as_array()
        )[1]
        .as_array());

        let (url, width, height) = match inner[3].as_array() {
            Some(i) => (
                uoc!(i[0].as_str()).to_string(),
                uoc!(i[2].as_i64()),
                uoc!(i[1].as_i64()),
            ),
            None => continue,
        };

        let image = Image {
            url,
            width,
            height,
            thumbnail: uoc!(uoc!(inner[2].as_array())[0].as_str()).to_string(),
            source: uoc!(uoc!(uoc!(inner[25].as_object())["2003"].as_array())[2].as_str()).to_string(),
        };

        images.push(image);
    }

    Some(images)
}
