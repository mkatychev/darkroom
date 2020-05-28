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

pub use error::FrError;
pub use reel::Reel;
use serde::Serialize;
use std::{fs, path::Path};

// Convenience in converting a Path to a String
pub fn file_to_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    // https://github.com/serde-rs/json/issues/160
    let json_string: String = fs::read_to_string(path)?;

    Ok(json_string)
}

pub trait ToStringHidden {
    fn to_string_hidden(&self) -> Result<String, FrError>;
}

pub trait ToStringPretty {
    fn to_string_pretty(&self) -> Result<String, FrError>;
}

impl<T> ToStringPretty for T
where
    T: ?Sized + Serialize,
{
    fn to_string_pretty(&self) -> Result<String, FrError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
