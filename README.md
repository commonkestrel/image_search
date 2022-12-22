<div align="center"><img src="https://raw.githubusercontent.com/Jibble330/image_search/master/misc/logo-white.png" width=640px></div>

# Google Image Search
![Crates.io](https://img.shields.io/crates/v/image_search) ![docs.rs](https://img.shields.io/docsrs/image_search) ![Crates.io](https://img.shields.io/crates/l/image_search)

A crate designed to search Google Images based on provided arguments.
Due to the limitations of using only a single request to fetch images, only a max of about 100 images can be found per request.
These images may be protected under copyright, and you shouldn't do anything punishable with them, like using them for commercial use.

# Arguments

There are 2 required parameters, along with a variety of different arguments.
| Argument | Type | Description |
| --- | --- | --- |
| **query** | `&str` | The keyword(s) to search for.  |
| **limit** | `usize` | The maximum amount of images to fetch. Cannot fetch more than 100. |  
| **thumbnails** | `bool` | Causes the `urls` and `downloads` functions to use the urls of the thumbnails instead of the urls of the images. |
| **timeout** | [`Option<Duration>`](https://doc.rust-lang.org/stable/std/time/struct.Duration.html) | Sets the timeout for the `download` function. Setting to `None` is not recommended, since in rare cases images can fail to download but not throw an error, causing the `download` function to never return. |
| **directory** | [`Option<PathBuf>`](https://doc.rust-lang.org/stable/std/path/struct.PathBuf.html) |  |

## Search Arguments

These are optional arguments that Google can use to filter images, useful for narrowing your search.
They are used via the various methods on the `Arguments` struct. Each argument is contained in an `enum` which contains all possible options.

| Argument | Options | Description |
| --- | --- | --- |
| **Color** | `Red`, `Orange`, `Yellow`, `Green`, `Teal`, `Blue`, `Purple`, `Pink`, `White`, `Gray`, `Black`, `Brown` | Filter images by the dominant color. |
| **ColorType** | `Color`, `Grayscale`, `Transparent` | Filter images by the color type. |
| **License** | `CreativeCommons`, `Other` | Filter images by the usage license. |
| **Type** | `Face`, `Photo`, `Clipart`, `Lineart`, `Animated` | Filters by the type of images to search for. |
| **Time** | `Day`, `Week`, `Month`, `Year` | Only finds images posted in the time specified. |
| **AspectRatio** | `Tall`, `Square`, `Wide`, `Panoramic` | Specifies the aspect ratio of the images. |
| **Format** | `Jpg`, `Gif`, `Png`, `Bmp`, `Svg`, `Webp`, `Ico`, `Raw` | Filters out images that are not a specified format. If you would like to download images as a specific format, use the download_format argument instead. |

# Examples
Using the asynchronous API requires some sort of async runtime, usually [`tokio`](https://crates.io/crates/tokio), which can be added to your `Cargo.toml` like so:
```toml
[dependencies]
image_search = "0.3"
tokio = { version = "1", features = ["full"] }
```
It can be used like this:
```rust
extern crate tokio;
extern crate image_search;

use std::path::PathBuf;
use image_search::{Arguments, Color, urls, search, download};
 
#[tokio::main]
async fn main() -> Result<(), image_search::Error> {
    let args = Arguments::new("example", 10)
        .color(Color::Gray)
        .directory(PathBuf::from("downloads")); // Only affects the download function
     
    let _image_urls = urls(args.clone()).await?;
    let _images = search(args.clone()).await?;
    let _paths = download(args).await?;
 
    Ok(())
}
```

# Blocking
There is an optional "blocking" API that can be enabled:
```toml
[dependencies]
image_search = { version = "0.3", features = ["blocking"] }
```
This is called like so:
```rust
extern crate image_search;

use std::path::PathBuf;
use image_search::{Arguments, Time, blocking::{urls, search, download}};

fn main() -> Result<(), image_search::Error> {
    let args = Arguments::new("example", 10)
        .time(Time::Month)
        .directory(PathBuf::from("downloads")); // Only affects the download function
    
    let _image_urls = urls(args.clone())?;
    let _images = search(args.clone())?;
    let _paths = download(args)?;

    Ok(())
}
```