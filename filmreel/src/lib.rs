/*!

# filmReel <img src="https://raw.githubusercontent.com/Bestowinc/filmReel/master/images/filmreel.svg" width="65"/>

A [VCR specification](https://github.com/Bestowinc/filmReel) implemented in Rust.

The `filmreel` crate is a pure Rust implementation of the declarative contract testing spec enjoying the memory safety
property and other benefits from the Rust language.

## Quick Start

Add the following to the Cargo.toml of your project:

```toml
[dependencies]
filmreel = "0.6"
```

*/

pub mod cut;
mod error;
pub mod frame;
pub mod reel;
pub mod response;
pub mod utils;
pub mod vreel;

#[cfg(test)]
mod serde_tests;

pub use cut::Register;
pub use error::{FrError, WithPath};
pub use frame::Frame;
pub use reel::{MetaFrame, Reel};
pub use response::Response;
use serde::Serialize;
use std::{fs, io, path::Path};
pub use vreel::VirtualReel;

// Convenience in converting a Path to a String
pub fn file_to_string<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    // https://github.com/serde-rs/json/issues/160
    let json_string: String = fs::read_to_string(path)?;

    Ok(json_string)
}

// Convenience in converting a Path to a BufReader
pub fn file_to_reader<P>(path: P) -> Result<io::BufReader<fs::File>, FrError>
where
    P: AsRef<Path>,
{
    // https://github.com/serde-rs/json/issues/160
    let file = fs::File::open(&path).with_path(&path)?;
    Ok(io::BufReader::new(file))
}

pub trait ToStringHidden: ToStringPretty {
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

impl<T> ToStringHidden for T
where
    T: ?Sized + Serialize,
{
    /// Pretty formatting for Register serialization, any cut variable names starting with an underscore are
    /// presented as `${_HIDDEN}` in stdout
    fn to_string_hidden(&self) -> Result<String, FrError> {
        let val = match serde_json::to_value(self)? {
            serde_json::Value::Object(mut map) => {
                for (k, v) in map.iter_mut() {
                    if k.starts_with('_') {
                        *v = serde_json::Value::String("${_HIDDEN}".to_string());
                    }
                }
                serde_json::Value::Object(map)
            }
            i => i,
        };
        let str_val = serde_json::to_string_pretty(&val)?;
        Ok(str_val)
    }
}

// https://stackoverflow.com/questions/26368288/how-do-i-stop-iteration-and-return-an-error-when-iteratormap-returns-a-result
pub fn until_err<T, E>(err: &mut &mut Result<(), E>, item: Result<T, E>) -> Option<T> {
    match item {
        Ok(item) => Some(item),
        Err(e) => {
            **err = Err(e);
            None
        }
    }
}
