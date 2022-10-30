use std::path::Path;

pub mod bookmarks;
pub mod chapter;
pub mod mediainfo;

/// Given a ```path``` creates a string in a format needed by gstreamer
pub fn make_uri(path: &Path) -> String {
    format!("file://{}", path.to_str().unwrap())
}
