use image_search::{
    blocking::{download, search, urls},
    Arguments,
};
use std::path::PathBuf;

fn main() -> Result<(), image_search::Error> {
    let args = Arguments::new("example", 10)
        .color(image_search::Color::Gray)
        .directory(PathBuf::from("downloads")); // Only affects the download function

    let _image_urls = urls(args.clone())?;
    let _images = search(args.clone())?;
    let _paths = download(args)?;

    Ok(())
}
