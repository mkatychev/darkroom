use crate::{cut::Register, error::FrError};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap, convert::TryFrom, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct VirtualReel<'a> {
    pub name: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    // use BTtreeMap en lieu of HashMap to maintain order
    pub frames: VirtualFrames<'a>,
    pub cut: VirtualCut,
}

impl VirtualReel<'_> {
    /// Prepends the "path" key to any PathBuf values in "frames" and "cut"
    pub fn join_path(&mut self) {
        if self.path.is_none() {
            return;
        }

        let reel_path = self.path.clone().unwrap();

        match &mut self.frames {
            VirtualFrames::RenamedList(ref mut map) => {
                for (_, v) in map.iter_mut() {
                    *v = reel_path.join(v.clone());
                }
            }
            VirtualFrames::List(list) => {
                for v in list.iter_mut() {
                    *v = reel_path.join(v.clone());
                }
            }
        }

        match &mut self.cut {
            VirtualCut::MergeCuts(ref mut list) => {
                for v in list.iter_mut() {
                    *v = reel_path.join(v.clone());
                }
            }
            VirtualCut::Cut(ref mut path) => *path = reel_path.join(path.clone()),
            VirtualCut::Register(_) => (),
        }
    }
}

impl TryFrom<PathBuf> for VirtualReel<'_> {
    type Error = FrError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let buf = crate::file_to_reader(path)?;
        let vreel = serde_json::from_reader(buf)?;
        Ok(vreel)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum VirtualCut {
    MergeCuts(Vec<PathBuf>),
    Cut(PathBuf),
    Register(Register),
}

/// VirtualFrames represents the frames variant containing a list of frames that can be renamed
///
/// VirtualFrames::RenamedList variant will replace the frame name with the key value when running the
/// VirtualReel (ordering reel flow by the new key name):
///
///  ```json
///  {"new_frame_name": "usr.01s.createuser.fr.json"}
///  ```
///  The example above will run `"usr.01s.createuser.fr.json"` as `"new_frame_name"`
///
/// VirtualFrames::List variant will retain the frame name and order the reel sequence by the
/// index position of the filepath:
///
///  ```json
///  ["usr.04s.validateuser.fr.json", "usr.01s.createuser.fr.json"]
///  ```
///
///  The example above will run `usr.01s.createuser.fr.json` *after* `usr.04s.validateuser.fr.json`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum VirtualFrames<'a> {
    RenamedList(BTreeMap<Cow<'a, str>, PathBuf>),
    List(Vec<PathBuf>),
}

#[macro_export]
macro_rules! vframes {
    ([$val: expr]) => (
        use ::std::path::PathBuf;
        VirtualFrames::List(vec![PathBuf::from($val)])
    );
    ([$($val: expr),+]) => ({

        let vec = vec![$(std::path::PathBuf::from($val),)+];
        VirtualFrames::List(vec)
    });
    ({$( $key: expr => $val: expr ),*}) => {{
        use ::std::collections::BTreeMap;
        use ::std::path::PathBuf;

        let mut map =  BTreeMap::new();
        $(map.insert($key.into(), $val);)*
            VirtualFrames::RenamedList(map)
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{register, test_ser_de};

    const VREEL_JSON: &str = r#"
{
  "name": "reel_name",
  "frames": [ "frame1.fr.json", "frame2.fr.json"],
  "cut": {"KEY": "value"}
}
    "#;

    test_ser_de!(
        vframe,
        VirtualReel {
            name: "reel_name".into(),
            path: None,
            frames: vframes!(["frame1.fr.json", "frame2.fr.json"]),
            cut: VirtualCut::Register(register!({"KEY" => "value"})),
        },
        VREEL_JSON
    );

    const PATH_VREEL_JSON: &str = r#"
{
  "name": "reel_name",
  "path": "./reel_dir",
  "frames": {
    "1": "other_reel.01s.name.fr.json"
  },
  "cut": ["other_reel.cut.json"]
}
    "#;

    test_ser_de!(
        pathbuf_vframe,
        VirtualReel {
            name: "reel_name".into(),
            path: Some("./reel_dir".into()),
            frames: vframes!({"1" => PathBuf::from("other_reel.01s.name.fr.json")}),
            cut: VirtualCut::MergeCuts(vec!["other_reel.cut.json".into()]),
        },
        PATH_VREEL_JSON
    );
}
