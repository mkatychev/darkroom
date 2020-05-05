/*!

A [filmReel](https://github.com/Bestowinc/filmReel) implementation for Rust.

The `filmreel` crate is a pure Rust implementation of the declarative contract testing spec enjoying the memory safety
property and other benefits from the Rust language.

## Quick Start

Add the following to the Cargo.toml of your project:

```toml
[dependencies]
filmreel = "0.2"
```

*/

pub mod cut;
mod error;
pub mod frame;
pub mod reel;
pub mod utils;

pub use reel::Reel;
use std::fs;
use std::io::Result;
use std::path::Path;

// Convenience in converting a Path to a String
pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    // https://github.com/serde-rs/json/issues/160
    let json_string: String = fs::read_to_string(path)?;

    Ok(json_string)
}
