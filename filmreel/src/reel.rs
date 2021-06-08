use crate::error::FrError;
use glob::glob;
use std::{
    collections::HashMap,
    convert::TryFrom,
    ffi::OsStr,
    iter::FromIterator,
    ops::Range,
    path::{Path, PathBuf},
    result::Result,
};

/// Represents the sequence of Frames to execute.
///
/// [Reel spec](https://github.com/Bestowinc/filmReel/blob/master/Reel.md#reel)
#[derive(Debug)]
pub struct Reel {
    dir:    PathBuf,
    frames: Vec<MetaFrame>,
}

const SEQUENCE_DUPE_ERR: &str = "Associated frames cannot share the same sequence number";
const METAFRAME_DELIMIT_ERR: &str =
    "Frame filename mast have exactly 3 period delimited sections preceding '.fr.json'";

impl Reel {
    /// A new reel is created from a provided Path or PathBuf
    pub fn new<P>(dir: P, reel_name: &str, range: Option<Range<u32>>) -> Result<Self, FrError>
    where
        P: AsRef<Path>,
    {
        let dir_glob = Self::get_frame_dir_glob(&dir, reel_name);

        let mut frames = Self::get_metaframes(&dir_glob, range)?;

        // sort by string value since sorting by f32 is not idiomatic
        frames.sort_by(|a, b| a.path.cmp(&b.path));

        let reel = Self {
            dir: PathBuf::from(dir.as_ref().to_str().expect("None Reel dir")),
            frames,
        };
        reel.validate()?;
        Ok(reel)
    }

    /// convenience function to get default associated cut file
    pub fn get_default_cut_path(&self) -> PathBuf {
        let reel_name = self.frames[0].reel_name.clone();
        self.dir.join(format!("{}.cut.json", reel_name))
    }

    /// Return only successful frames
    pub fn success_only(self) -> Self {
        Self {
            dir:    self.dir,
            frames: self
                .frames
                .into_iter()
                .filter(MetaFrame::is_success)
                .collect(),
        }
    }

    /// Ensure that the Reel is valid
    pub fn validate(&self) -> Result<(), FrError> {
        let mut sequence_set = HashMap::new();
        // ensure that the Reel has no duplicate sequence number
        for frame in self.frames.iter() {
            if let Some(dupe_ref) = sequence_set.insert(&frame.step, frame.get_filename()) {
                return Err(FrError::ReelParsef(
                    SEQUENCE_DUPE_ERR,
                    format!("{} and {}", dupe_ref, frame.get_filename()),
                ));
            }
        }
        Ok(())
    }

    // get_frame_dir_glob returns a glob pattern corresponding to all the Frame JSON files contained in
    // the path directory provided non-recursively
    pub fn get_frame_dir_glob<P>(dir: P, reel_name: &str) -> PathBuf
    where
        P: AsRef<Path>,
    {
        let dir_ref = dir.as_ref();
        if !dir_ref.is_dir() {
            panic!(
                "dir argument to get_frame_dir_glob is not a directory: {}",
                dir_ref.to_string_lossy().to_string(),
            );
        }

        dir_ref.join(format!("{}.*.*.fr.json", reel_name))
    }

    /// get_metaframes takes a directory glob ref and a possible range, returning a vector of
    /// MetaFrames
    fn get_metaframes<T>(dir_glob: T, range: Option<Range<u32>>) -> Result<Vec<MetaFrame>, FrError>
    where
        T: AsRef<OsStr>,
    {
        // Associate the range with permitted whole sequence values
        // if an Option::None range was passed, all frames are permitted
        let permit_frame: Box<dyn Fn(u32) -> bool> = match range {
            Some(r) => Box::new(move |n| r.contains(&n)),
            None => Box::new(|_| true),
        };

        let mut frames = Vec::new();

        for entry in glob(dir_glob.as_ref().to_str().unwrap())
            .map_err(|e| FrError::ReelParsef("PatternError: {}", e.to_string()))?
            .filter_map(Result::ok)
            .filter(|path| path.is_file())
        {
            let frame = MetaFrame::try_from(&entry)?;
            if permit_frame(frame.step_f32.trunc() as u32) {
                frames.push(frame);
            }
        }
        Ok(frames)
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
    pub reel_name:  String,
    pub frame_type: FrameType,
    pub alt_name:   Option<String>,
    pub name:       String,
    pub path:       PathBuf,
    pub step_f32:   f32,
    step:           String,
}

impl TryFrom<&PathBuf> for MetaFrame {
    type Error = FrError;

    fn try_from(p: &PathBuf) -> Result<Self, Self::Error> {
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
        let sequence_number = reel_parts.remove(0);
        let (seq, fr_type) = parse_sequence(sequence_number)?;
        let name = reel_parts.remove(0);

        // only three indices should be present when split on '.'
        if !reel_parts.is_empty() {
            return Err(FrError::ReelParse(METAFRAME_DELIMIT_ERR));
        }

        Ok(Self {
            path: p.clone(),
            alt_name: None,
            name: name.to_string(),
            reel_name,
            step_f32: seq,
            step: sequence_number.to_string(),
            frame_type: fr_type,
        })
    }
}

impl MetaFrame {
    fn is_success(&self) -> bool {
        self.frame_type == FrameType::Success
    }

    // get_filename returns the str representation of the MetaFrame.path file stem
    pub fn get_filename(&self) -> String {
        return format!("{}.{}.{}.fr.json", self.reel_name, self.step, self.name);
    }

    // get_cut_file retuns the default cut file location
    pub fn get_cut_file<P: AsRef<Path>>(&self, dir: P) -> PathBuf {
        if !dir.as_ref().is_dir() {
            panic!("\"{}\" is not a directory!", dir.as_ref().to_string_lossy());
        }

        dir.as_ref().join(format!("{}.cut.json", self.reel_name))
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
        case("011e_8", (11.8, FrameType::Error)),
        case("01e", (1.0, FrameType::Error)),
        )]
    fn test_parse_sequence(input: &str, expected: (f32, FrameType)) {
        match parse_sequence(input) {
            Ok(mat) => assert_eq!(expected, mat),
            Err(err) => assert_eq!("some_err", err.to_string()),
        }
    }

    #[test]
    fn test_metaframe_try_from() {
        let try_path = MetaFrame::try_from(&PathBuf::from("./reel_name.01s.frame_name.fr.json"))
            .expect("test_metaframe_try_from failed try_from");
        assert_eq!(
            MetaFrame {
                frame_type: FrameType::Success,
                name:       "frame_name".to_string(),
                alt_name:   None,
                path:       PathBuf::from("./reel_name.01s.frame_name.fr.json"),
                reel_name:  "reel_name".to_string(),
                step:       "01s".to_string(),
                step_f32:   1.0,
            },
            try_path
        );
    }

    #[test]
    fn test_validate() {
        let reel = Reel {
            dir:    ".".into(),
            frames: vec![
                MetaFrame::try_from(&PathBuf::from("./reel.01s.frame1.fr.json")).unwrap(),
                MetaFrame::try_from(&PathBuf::from("./reel.01e.frame2.fr.json")).unwrap(),
            ],
        };
        assert!(reel.validate().is_ok());
    }
    #[test]
    fn test_validate_err() {
        let reel = Reel {
            dir:    ".".into(),
            frames: vec![
                MetaFrame::try_from(&PathBuf::from("./reel.01s.frame1.fr.json")).unwrap(),
                MetaFrame::try_from(&PathBuf::from("./reel.01s.frame2.fr.json")).unwrap(),
            ],
        };
        assert_eq!(
            reel.validate().unwrap_err(),
            FrError::ReelParsef(
                SEQUENCE_DUPE_ERR,
                "reel.01s.frame1.fr.json and reel.01s.frame2.fr.json".to_string()
            )
        );
    }
}
