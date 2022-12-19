//! Image Search is designed to search Google Images based on provided arguments.
//! Due to the limitations of using only a single request to fetch images, only a max of about 100 images can be found per request.
//! These images may be protected under copyright, and you shouldn't do anything punishable with them, like using them for commercial use.

extern crate glob;
extern crate infer;
extern crate reqwest;
extern crate serde_json;

use crate::{Arguments, DownloadError, Error, Image};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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
pub fn search(args: Arguments) -> Result<Vec<Image>, Error> {
    let url = build_url(&args);
    let body = match get(url) {
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
pub fn urls(args: Arguments) -> Result<Vec<String>, Error> {
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
pub fn download(args: Arguments) -> Result<Vec<PathBuf>, Error> {
    let query = &args.query.to_owned();
    let directory = &args.directory.to_owned();
    let images = urls(args)?;

    let client = reqwest::blocking::Client::new();

    let dir = match directory {
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
    for url in images.iter() {
        let mut path = dir.join(query.to_owned() + &suffix.to_string());

        let all = glob::glob(&(path.display().to_string() + ".*")).unwrap();
        let mut matches = 0;
        for _ in all {
            matches += 1;
        }

        while matches > 0 {
            matches = 0;
            suffix += 1;
            path = dir.join(query.to_owned() + &suffix.to_string());
            let all = glob::glob(&(path.display().to_string() + ".*")).unwrap();
            for _ in all {
                matches += 1;
            }
        }
        suffix += 1;

        let with_extension = match download_image(&client, path, url.to_owned()) {
            Ok(e) => e,
            Err(_) => continue,
        };

        paths.push(with_extension);
    }

    Ok(paths)
}

fn download_image(
    client: &reqwest::blocking::Client,
    mut path: PathBuf,
    url: String,
) -> Result<PathBuf, DownloadError> {
    let resp = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return Err(DownloadError::Network(e)),
    };

    let buf = match resp.bytes() {
        Ok(b) => b,
        Err(e) => return Err(DownloadError::Network(e)),
    };

    let kind = match infer::get(&buf) {
        Some(k) => k,
        None => return Err(DownloadError::Extension),
    };

    path.set_extension(kind.extension());

    let mut f = match File::create(&path) {
        Ok(f) => f,
        Err(e) => return Err(DownloadError::Fs(e)),
    };

    match f.write_all(&buf) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::Fs(e)),
    };

    Ok(path)
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

fn get(url: String) -> Result<String, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.104 Safari/537.36";

    let resp = client.get(url).header("User-Agent", agent).send()?;

    Ok(resp.text()?)
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
