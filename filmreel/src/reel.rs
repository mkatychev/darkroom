use glob::glob;
use std::convert::TryFrom;
use std::error::Error;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::result::Result;

/// Represents the sequence of Frames to execute.
///
/// [Reel spec](https://github.com/Bestowinc/filmReel/blob/supra_dump/Reel.md#reel)
pub struct Reel {
    frames: Vec<MetaFrame>,
}

impl Reel {
    /// A new reel is created from a provided Path or PathBuf
    pub fn new<P: AsRef<Path>>(dir: P, reel_name: &str) -> Result<Self, Box<dyn Error>> {
        let mut frames = Vec::new();
        let dir_glob = dir.as_ref().join(format!("*.{}.*.fr.json", reel_name));

        for entry in glob(&dir_glob.to_str().unwrap())
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .filter(|path| path.is_file())
        {
            frames.push(MetaFrame::try_from(entry)?);
        }
        Ok(Self { frames })
    }
}

// TODO add subsequence number
/// MetaFrame holds the metadata needed that allows a Reel to generate Frames and Takes in a particular
/// sequence.
///
/// Frame filename anatomy:
///
/// ```text
/// ┌─────────── Sequence number        // 01
/// │ ┌───────── Frame type             // se
/// │ │  ┌────── Reel name              // usr
/// │ │  │   ┌─  Method name            // createuser
/// ▼ ▼  ▼   ▼
/// 01se.usr.createuser.fr.json
///                     ▲
///                     └─ Frame suffix // .fr.json
/// ```
///
#[derive(PartialEq, Debug)]
pub struct MetaFrame {
    path: PathBuf,
    name: String,
    reel_name: String,
    step: f32,
    frame_type: FrameType,
}

impl TryFrom<PathBuf> for MetaFrame {
    type Error = Box<dyn Error>;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        let mut reel_parts: Vec<&str> = p
            .file_name()
            .map(|s| s.to_str())
            .expect("file_name.to_str() error")
            .map(|s| s.trim_end_matches(".fr.json"))
            .map(|s| s.split('.').collect())
            .expect("str split error");

        let (seq, fr_type) = parse_sequence(reel_parts.remove(0))?;
        let reel_name = String::from(reel_parts.remove(0));
        let name = reel_parts.remove(0);

        // only three indices should be present when split on '.'
        assert!(reel_parts.is_empty());

        Ok(Self {
            path: p.clone(),
            name: name.to_string(),
            reel_name,
            step: seq,
            frame_type: fr_type,
        })
    }
}

fn parse_sequence(seq: &str) -> Result<(f32, FrameType), Box<dyn Error>> {
    let mut seq_chars: Vec<char> = Vec::new();
    let mut type_str: String = String::new();
    for ch in seq.chars() {
        match ch {
            // [0-9]
            ch if ch.is_ascii_digit() => {
                seq_chars.push(ch);
            }
            // [A-Za-z]
            ch if ch.is_ascii_alphabetic() => {
                type_str.push(ch);
            }
            '_' => {
                seq_chars.push('.');
            }
            _ => {
                unimplemented!("{} is an invalid sequence char!", ch);
            }
        }
    }

    let seq_f32 = String::from_iter(seq_chars).parse::<f32>()?;

    Ok((seq_f32, FrameType::from(type_str)))
}

/// [Frame Types](https://github.com/Bestowinc/filmReel/blob/supra_dump/Reel.md#frame-type)
#[derive(PartialEq, Debug)]
pub enum FrameType {
    Error,
    Success,
    PsError, // P.S. error
    Invalid,
}

impl<T: AsRef<str>> From<T> for FrameType {
    fn from(fr: T) -> Self {
        match fr.as_ref() {
            "e" => Self::Error,
            "s" => Self::Success,
            "se" => Self::PsError,
            _ => Self::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest(input, expected,
        case("02se", (2.0, FrameType::PsError)),
        case("10s_1", (10.1, FrameType::Success)),
        case("011e_8", (11.8, FrameType::Error))
        )]
    fn test_parse_sequence(input: &str, expected: (f32, FrameType)) {
        match parse_sequence(input) {
            Ok(mat) => assert_eq!(expected, mat),
            Err(err) => assert_eq!("some_err", err.to_string()),
        }
    }
}
