use crate::error::FrError;
use glob::glob;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::result::Result;

/// Represents the sequence of Frames to execute.
///
/// [Reel spec](https://github.com/Bestowinc/filmReel/blob/master/Reel.md#reel)
#[derive(Debug)]
pub struct Reel {
    dir: PathBuf,
    frames: Vec<MetaFrame>,
}

impl Reel {
    /// A new reel is created from a provided Path or PathBuf
    pub fn new<P: AsRef<Path>>(dir: P, reel_name: &str) -> Result<Self, FrError> {
        let mut frames = Vec::new();
        let dir_glob = dir.as_ref().join(format!("{}.*.*.fr.json", reel_name));

        for entry in glob(&dir_glob.to_str().unwrap())
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .filter(|path| path.is_file())
        {
            frames.push(MetaFrame::try_from(entry)?);
        }

        // sort by string value since sorting by f32 is not idiomatic
        frames.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Self {
            dir: PathBuf::from(dir.as_ref().to_str().expect("None Reel dir")),
            frames,
        })
    }

    /// convenience function to get default associated cut file
    pub fn get_default_cut_path(&self) -> PathBuf {
        let reel_name = self.frames[0].reel_name.clone();
        self.dir.join(format!("{}.cut.json", reel_name))
    }

    /// Return only successful frames
    pub fn success_only(self) -> Self {
        Self {
            dir: self.dir,
            frames: self
                .frames
                .clone()
                .into_iter()
                .filter(|x| x.is_success())
                .collect(),
        }
    }
}

impl IntoIterator for Reel {
    type Item = MetaFrame;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.frames.into_iter()
    }
}

// TODO add subsequence number
/// MetaFrame holds the metadata needed for sequential Frame execution and Take generation.
///
/// Frame filename anatomy:
///
/// ```text
/// ┌─────────── Reel name              // usr
/// │   ┌─────── Sequence number        // 01
/// │   │ ┌───── Return type            // se
/// │   │ │  ┌── Command name           // createuser
/// ▼   ▼ ▼  ▼
/// usr.01se.createuser.fr.json
///                     ▲
///                     └─ Frame suffix // .fr.json
/// ```
///
#[derive(Clone, PartialEq, Debug)]
pub struct MetaFrame {
    pub path: PathBuf,
    pub name: String,
    pub reel_name: String,
    pub step: f32,
    pub frame_type: FrameType,
}

impl TryFrom<PathBuf> for MetaFrame {
    type Error = FrError;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        let mut reel_parts: Vec<&str> = match p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.trim_end_matches(".fr.json"))
            .map(|s| s.split('.').collect())
        {
            Some(s) => s,
            None => return Err(FrError::ReelParse("failed parsing PathBuf")),
        };

        let reel_name = String::from(reel_parts.remove(0));
        let (seq, fr_type) = parse_sequence(reel_parts.remove(0))?;
        let name = reel_parts.remove(0);

        // only three indices should be present when split on '.'
        assert!(
            reel_parts.is_empty(),
            "frame name should only have 3 period delimited sections ending with '.fr.json'"
        );

        Ok(Self {
            path: p.clone(),
            name: name.to_string(),
            reel_name,
            step: seq,
            frame_type: fr_type,
        })
    }
}

impl MetaFrame {
    fn is_success(&self) -> bool {
        self.frame_type == FrameType::Success
    }
}

fn parse_sequence(seq: &str) -> Result<(f32, FrameType), FrError> {
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
                return Err(FrError::ReelParsef(
                    "{} is an invalidsequence char!",
                    ch.to_string(),
                ))
            }
        }
    }

    let seq_f32 = match String::from_iter(&seq_chars).parse::<f32>() {
        Ok(v) => v,
        Err(e) => {
            return Err(FrError::ReelParsef(
                "Could not parse value into f32",
                e.to_string(),
            ))
        }
    };
    let frame_type = FrameType::from(type_str);

    if let FrameType::Invalid = frame_type {
        return Err(FrError::ReelParse(
            "Unrecognized frame type in frame sequence",
        ));
    }

    Ok((seq_f32, frame_type))
}

/// [Frame Types](https://github.com/Bestowinc/filmReel/blob/master/Reel.md#frame-type)
#[derive(Clone, PartialEq, Debug)]
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

    #[test]
    fn test_metaframe_try_from() {
        let try_path = MetaFrame::try_from(PathBuf::from("./reel_name.01s.frame_name.fr.json"))
            .expect("test_metaframe_try_from failed try_from");
        assert_eq!(
            MetaFrame {
                frame_type: FrameType::Success,
                name: "frame_name".to_string(),
                path: PathBuf::from("./reel_name.01s.frame_name.fr.json"),
                reel_name: "reel_name".to_string(),
                step: 1.0,
            },
            try_path
        );
    }
}
