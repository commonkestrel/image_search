# Image Search
A crate designed to search Google Images based on provided arguments.
Due to the limitations of using only a single request to fetch images, only a max of about 100 images can be found per request.
These images may be protected under copyright, and you shouldn't do anything punishable with them, like using them for commercial use.

# Examples
Using the asynchronous API requires some sort of async runtime, usually [`tokio`], which can be added to your `Cargo.toml` like so:
```toml
[dependencies]
image_search = "0.2"
tokio = { version = "1", features = ["full"] }
```
It is called like so
```rust
extern crate tokio;
extern crate image_search;

use image_search::{Arguments, urls, search, download};

#[tokio::main]
async fn main() -> Resutl<(), image_search::Error> {
    let args = Arguments::new("example", 10)
        .color(image_search::Color::Gray)
        .directory(Path::new("downloads")); // Only affects the download function
    
    let image_urls = urls(args.clone()).await?;
    let images = search(args.clones()).await?;
    let paths = download(args).await?;

    Ok(())
}
```

# Blocking
There is an optional "blocking" API that can be enabled:
```toml
[dependencies]
image_search = { version = "0.2", features = ["blocking"] }
```
This is called like so:
```rust
extern crate image_search;

use image_search{Arguments, blocking::{urls, search, download}};

fn main() -> Result<(), image_search::Error> {
    let args = Arguments::new("example", 10)
        .color(image_search::Color::Gray)
        .directory(Path::new("downloads")); // Only affects the download function
    
    let image_urls = urls(args.clone())?;
    let images = search(args.clones())?;
    let paths = download(args)?;

    Ok(())
}
```
[`tokio`]: https://docs.rs/tokio/latest/tokio/