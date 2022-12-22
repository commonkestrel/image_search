//! A crate designed to search Google Images based on provided arguments.
//! Due to the limitations of using only a single request to fetch images, only a max of about 100 images can be found per request.
//! These images may be protected under copyright, and you shouldn't do anything punishable with them, like using them for commercial use.
//!
//! # Examples
//! Using the asynchronous API requires some sort of async runtime, usually [`tokio`](https://crates.io/crates/tokio), which can be added to your `Cargo.toml` like so:
//! ```
//! [dependencies]
//! image_search = "0.3"
//! tokio = { version = "1", features = ["full"] }
//! ```
//! It can be used like this:
//! ```
//! extern crate tokio;
//! extern crate image_search;
//!
//! use std::path::PathBuf;
//! use image_search::{Arguments, urls, search, download};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), image_search::Error> {
//!     let args = Arguments::new("example", 10)
//!         .color(image_search::Color::Gray)
//!         .directory(PathBuf::new("downloads")); // Only affects the download function
//!     
//!     let _image_urls = urls(args.clone()).await?;
//!     let _images = search(args.clone()).await?;
//!     let _paths = download(args).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Blocking
//! There is an optional "blocking" API that can be enabled:
//! ```
//! [dependencies]
//! image_search = { version = "0.3", features = ["blocking"] }
//! ```
//! This is called like so:
//! ```
//! extern crate image_search;
//!
//! use std::path::PathBuf;
//! use image_search::{Arguments, blocking::{urls, search, download}};
//!
//! fn main() -> Result<(), image_search::Error> {
//!     let args = Arguments::new("example", 10)
//!         .color(image_search::Color::Gray)
//!         .directory(PathBuf::from("downloads")); // Only affects the download function
//!     
//!     let _image_urls = urls(args.clone())?;
//!     let _images = search(args.clone())?;
//!     let _paths = download(args)?;
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "blocking")]
pub mod blocking;

extern crate futures;
extern crate glob;
extern crate infer;
extern crate reqwest;
extern crate serde_json;

use std::env;
use std::fmt;
use std::time::Duration;

use std::fs::File;
use std::path::PathBuf;

use futures::future;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Used to construct the arguments for searching and downloading images.
///
/// # Example
/// ```
/// extern crate tokio;
/// extern crate image_search;
///
/// use image_search::{self, Arguments};
///
/// #[tokio::main]
/// async fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("example", 10)
///         .color(image_search::Color::Black)
///         .ratio(image_search::Ratio::Square);
///     let images = image_search::search(args).await?;
/// }
#[derive(Debug, Clone)]
pub struct Arguments {
    query: String,
    limit: usize,
    thumbnails: bool,
    timeout: Option<Duration>,
    directory: Option<PathBuf>,

    color: Color,
    color_type: ColorType,
    license: License,
    image_type: ImageType,
    time: Time,
    ratio: Ratio,
    format: Format,
}

impl Arguments {
    fn params(&self) -> String {
        let split = &String::from("%2C");
        let mut params_str = String::new();

        let color = self.color.param();
        let color_type = self.color_type.param();
        let license = self.license.param();
        let image_type = self.image_type.param();
        let time = self.time.param();
        let ratio = self.ratio.param();
        let format = self.format.param();
        let params = [color, color_type, license, image_type, time, ratio, format];

        for param in params.iter() {
            if param.len() > 1 {
                params_str += split;
                params_str += param;
            }
        }

        params_str
    }

    pub fn new(query: &str, limit: usize) -> Arguments {
        Arguments {
            query: query.to_owned(),
            limit,
            thumbnails: false,
            timeout: Some(Duration::from_secs(20)),

            directory: None,
            color: Color::None,
            color_type: ColorType::None,
            license: License::None,
            image_type: ImageType::None,
            time: Time::None,
            ratio: Ratio::None,
            format: Format::None,
        }
    }

    /// Sets the directory the images will be downloaded to. Only used in the download function.
    pub fn directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.directory = Some(dir.into());
        self
    }

    /// Sets the optional request timeout for the `download` function. Defaults to 20 seconds.
    /// Not recomended to set to `None`, very rarely an image will fail to send data but not throw an error, causing the `download` function to never exit.
    pub fn timeout<D: Into<Option<Duration>>>(mut self, timeout: D) -> Self {
        self.timeout = timeout.into();
        self
    }

    /// Determines whether the image urls are switched out for the thumbnail urls.
    /// For example, the `urls` function will return the thumbnail urls instead of the image urls, and the `download` function will download the thumbnails instead of the full size image.
    /// Only affects the `urls` and `download` functions.
    pub fn thumbnails(mut self, thumb: bool) -> Self {
        self.thumbnails = thumb;
        self
    }

    /// Sets the color that Google will filter by.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the color type that Google will filter by.
    pub fn color_type(mut self, color_type: ColorType) -> Self {
        self.color_type = color_type;
        self
    }

    /// Sets the license that Google will filter by.
    pub fn license(mut self, license: License) -> Self {
        self.license = license;
        self
    }

    /// Sets the image type that Google will filter by.
    pub fn image_type(mut self, image_type: ImageType) -> Self {
        self.image_type = image_type;
        self
    }

    /// Sets how long ago the images can be posted.
    pub fn time(mut self, time: Time) -> Self {
        self.time = time;
        self
    }

    /// Sets the rough aspect ratio the images are filtered by.
    pub fn ratio(mut self, ratio: Ratio) -> Self {
        self.ratio = ratio;
        self
    }

    /// Sets the image format that Google will filter by.
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Color {
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Blue,
    Purple,
    Pink,
    White,
    Gray,
    Black,
    Brown,
}

impl Color {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Red => "isc:red",
            Self::Orange => "isc:orange",
            Self::Yellow => "isc:yellow",
            Self::Green => "isc:green",
            Self::Teal => "isc:teal",
            Self::Blue => "isc:blue",
            Self::Purple => "isc:purple",
            Self::Pink => "isc:pink",
            Self::White => "isc:white",
            Self::Gray => "isc:gray",
            Self::Black => "isc:black",
            Self::Brown => "isc:brown",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ColorType {
    None,
    Color,
    Grayscale,
    Transparent,
}

impl ColorType {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Color => "ic:full",
            Self::Grayscale => "ic:gray",
            Self::Transparent => "ic:trans",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum License {
    None,
    CreativeCommons,
    Other,
}

impl License {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::CreativeCommons => "il:cl",
            Self::Other => "il:ol",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    None,
    Face,
    Photo,
    Clipart,
    Lineart,
    Animated,
}

impl ImageType {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Face => "itp:face",
            Self::Photo => "itp:photo",
            Self::Clipart => "itp:clipart",
            Self::Lineart => "itp:lineart",
            Self::Animated => "itp:animated",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Time {
    None,
    Day,
    Week,
    Month,
    Year,
}

impl Time {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Day => "qdr:d",
            Self::Week => "qdr:w",
            Self::Month => "qdr:m",
            Self::Year => "qdr:y",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Ratio {
    None,
    Tall,
    Square,
    Wide,
    Panoramic,
}

impl Ratio {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Tall => "iar:t",
            Self::Square => "iar:s",
            Self::Wide => "iar:w",
            Self::Panoramic => "iar:xw",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    None,
    Jpg,
    Gif,
    Png,
    Bmp,
    Svg,
    Webp,
    Ico,
    Raw,
}

impl Format {
    fn param(&self) -> String {
        String::from(match self {
            Self::None => "",
            Self::Jpg => "ift:jpg",
            Self::Gif => "ift:gif",
            Self::Png => "ift:png",
            Self::Bmp => "ift:bmp",
            Self::Svg => "ift:svg",
            Self::Webp => "ift:webp",
            Self::Ico => "ift:ico",
            Self::Raw => "ift:raw",
        })
    }
}

/// Contains info about an image including the original url, the dimensions of the image (x, y), the url of the thumbnail, and the name of the source.
///
/// # Example
/// ```
/// Image {
///     url: "https://www.example.com/static/image.jpg",
///     width: 1920,
///     height: 1080,
///     thumbnail: "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcQQazt0j8bbA34OYbfE9hf7g_bzFGwbSZmwflwVw-rnOSOmdSX03xYzHZkm_TsmbnM3m88&usqp=CAU",
///     source: "https://www.example.com/articles/example.html"
/// }
#[derive(Debug, Clone)]
pub struct Image {
    pub url: String,
    pub width: i64,
    pub height: i64,
    pub thumbnail: String,
    pub source: String,
}

#[derive(Debug)]
pub enum Error {
    Parse,
    Dir(io::Error),
    Network(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse => write!(f, "Unable to parse images from json. Google may have changed the way their data is stored"),
            Self::Dir(err) => write!(f, "Unable to find or create: {}", err),
            Self::Network(err) => write!(f, "Unable to fetch webpage: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Self::Parse => "Unable to parse images from json",
            Self::Dir(_) => "Error when finding or creating directory",
            Self::Network(_) => "Error when making GET request",
        }
    }
}

#[derive(Debug)]
enum DownloadError {
    Overflow,
    Extension,
    Fs(std::io::Error),
    Network(reqwest::Error),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => write!(f, "Ran out of possible images"),
            Self::Extension => write!(f, "Unable to determine file extension"),
            Self::Fs(err) => write!(f, "Problem when creating or writing to file: {}", err),
            Self::Network(err) => write!(f, "Unable to fetch image: {}", err),
        }
    }
}

impl std::error::Error for DownloadError {
    fn description(&self) -> &str {
        match *self {
            Self::Overflow => "Ran out of possible images",
            Self::Extension => "File type not known or not an image",
            Self::Fs(_) => "Error occured creating or writing to file",
            Self::Network(_) => "Error when making GET request to fetch image",
        }
    }
}

macro_rules! debug_display {
    (for $($t:ty),+) => {
        $(impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        })*
    }
}
debug_display!(for Image, Arguments, Color, ColorType, License, ImageType, Time, Ratio, Format);

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
/// extern crate tokio;
/// extern crate image_search;
///
/// use image_search::{Arguments, search};
///
/// #[tokio::main]
/// async fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("example", 10);
///     let images = search(args).await?;
///
///     Ok(())
/// }
pub async fn search(args: Arguments) -> Result<Vec<Image>, Error> {
    let url = build_url(&args);
    let body = match get(url).await {
        Ok(b) => b,
        Err(e) => return Err(Error::Network(e)),
    };

    let imgs = match unpack(body) {
        Some(i) => i,
        None => return Err(Error::Parse),
    };

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
/// extern crate tokio;
/// extern crate image_search;
///
/// use image_search::{Arguments, urls};
///
/// #[tokio::main]
/// async fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("example", 10);
///     let images = image_search::urls(args).await?;
///
///     Ok(())
/// }
pub async fn urls(args: Arguments) -> Result<Vec<String>, Error> {
    let thumbnails = (&args.thumbnails).to_owned();
    let images = search(args).await?;

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
///
/// # Errors
/// This function will return an error if:
/// * The GET request fails
/// * The images are not able to be parsed
/// * The program is unable to create/read/write to files or directories
///
/// # Examples
///
/// ```
/// extern crate tokio;
/// extern crate image_search;
///
/// use image_search::{Arguments, download};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), image_search::Error> {
///     let args = Arguments::new("example", 10).directory(Path::new("downloads"));
///     let paths = image_search::download(args).await?;
///
///     Ok(())
/// }
pub async fn download(args: Arguments) -> Result<Vec<PathBuf>, Error> {
    let images = urls(Arguments {
        query: args.query.clone(),
        limit: 0,
        directory: args.directory.clone(),
        ..args
    })
    .await?;

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

        let all = glob::glob(&(path.display().to_string() + ".*")).unwrap();
        let mut matches = 0;
        for _ in all {
            matches += 1;
        }

        while matches > 0 {
            matches = 0;
            suffix += 1;
            path = dir.join(args.query.to_owned() + &suffix.to_string());
            let all = glob::glob(&(path.display().to_string() + ".*")).unwrap();
            for _ in all {
                matches += 1;
            }
        }

        paths.push(path);
        suffix += 1;
    }

    let with_extensions = download_n(images, paths, args.timeout).await;

    Ok(with_extensions)
}

/// Trys to download
async fn download_n(
    urls: Vec<String>,
    paths: Vec<PathBuf>,
    timeout: Option<Duration>,
) -> Vec<PathBuf> {
    let mut_urls = Arc::new(Mutex::new(urls));

    let mut downloaders = Vec::new();
    let client = reqwest::Client::new();
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
        let mut mut_urls = $urls.lock().unwrap();
        if mut_urls.is_empty() {
            return Err(DownloadError::Overflow);
        }
        let url = mut_urls.remove(0);
        std::mem::drop(mut_urls);

        url
    }};
}

async fn download_until(
    urls: Arc<Mutex<Vec<String>>>,
    path: PathBuf,
    client: reqwest::Client,
    timeout: Option<Duration>,
) -> Result<PathBuf, DownloadError> {
    let mut url = next_available!(urls);

    let with_extension = loop {
        let path = download_image(client.clone(), &path, url.to_owned(), timeout).await;
        if path.is_ok() {
            break path;
        }
        url = next_available!(urls);
    };

    with_extension
}

async fn download_image(
    client: reqwest::Client,
    path: &PathBuf,
    url: String,
    timeout: Option<Duration>,
) -> Result<PathBuf, DownloadError> {
    let builder = match timeout {
        Some(t) => client.get(url).timeout(t),
        None => client.get(url),
    };

    let resp = match builder.send().await {
        Ok(r) => r,
        Err(e) => return Err(DownloadError::Network(e)),
    };

    let buf = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return Err(DownloadError::Network(e)),
    };

    let kind = match infer::get(&buf) {
        Some(k) => k,
        None => return Err(DownloadError::Extension),
    };

    if kind.matcher_type() != infer::MatcherType::Image {
        return Err(DownloadError::Extension);
    }

    let with_extension = path.clone().with_extension(kind.extension());

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

async fn get(url: String) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.104 Safari/537.36";

    let resp = client.get(url).header("User-Agent", agent).send().await?;

    Ok(resp.text().await?)
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
            source: uoc!(uoc!(uoc!(inner[22].as_object())["2003"].as_array())[2].as_str())
                .to_string(),
        };

        images.push(image);
    }

    Some(images)
}
