pub mod cut;
mod error;
pub mod frame;
mod utils;

use std::fs;
use std::io::Result;
use std::path::Path;

// Convenience in converting a Path to a String
pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    // https://github.com/serde-rs/json/issues/160
    let json_string: String = fs::read_to_string(path)?;

    Ok(json_string)
}
