use crate::frame::Frame;
use glob::{glob, Paths};
use lazy_static::lazy_static;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;

// pub fn get_reel_sequence(dir: Path, reel_name: String) -> Result<(), Box<dyn Error>> {
//         let re = Regex::new(&format!(
//             r"(?x)
//                 (?P<head_val>.*)   # value preceding cut var
//                 (?P<esc_char>\\)?  # escape character
//                 (?P<cut_decl>\$\{{
//                 {}
//                 \}})               # Cut Variable Declaration
//                 (?P<tail_val>.*)   # value following cut var
//                 ",
//             var_name
//         ))
//         .expect("write-match regex error");
//     // return all glob matches for frame files with the frame's Reel name enclosed by periods
//     // ex: *.usr.*.fr.json should return all frame files that belong to the "usr" Reel.
//     for entry in glob(dir.join(dir, format!("*.{}.*.fr.json", reel_name)).filter_map(|f| f.is_file()) {
//         entry.trim_end_matches(".fr.json")
//     }
//     if dir.is_file() {
//         for entry in fs::read_dir(dir)? {
//             let entry = entry?;
//         }
//     }

pub fn get_frame_files<P: AsRef<Path>>(
    dir: P,
    reel_name: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let dir_glob = dir.as_ref().join(format!("*.{}.*.fr.json", reel_name));

    println!("{:?}", dir_glob.to_str());
    for entry in glob(&dir_glob.to_str().unwrap())
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
    // .filter(|f| f.is_file())
    {
        files.push(entry);
    }
    println!("{:?}", files);
    Ok(files)
}

// pub struct Reel {
//     frames: Vec<SequenceFrame>
//     }

// pub struct SequenceFrame
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_frame_files() {
        get_frame_files(
            "/Users/mkatychev/Documents/ryerson/stubs/crs/templates",
            "crs",
        )
        .unwrap()
        .iter()
        .for_each(|x| println!("OK: {}", x.display()));
    }
}
