use std::path::Path;

use gstreamer as gst;
use serde::{Deserialize,
            Serialize};

use super::bookmarks::Bookmark;
use super::make_uri;

macro_rules! get {
    ($tag:expr, $ty:ty) => {
        $tag.get::<$ty>().map(|k| k.get().to_owned())
    };
}

/// Information about a single file / chapter
#[derive(Debug, Deserialize, Serialize)]
pub struct Chapter {
    // pub path: PathBuf,
    pub filename: String,

    pub start_position: Option<u64>, /* start position for m4a/b chapter markings in seconds
                                      * from the start */
    pub length: u64, // in seconds

    pub length_display: String,

    pub bookmarks: Vec<Bookmark>,
    pub last_position: u64, // absolute position of when the chapter wast last played

    pub m4_title: Option<String>,
    pub m4_tracknumber: Option<u32>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(skip)]
    pub title: Option<String>,
    #[serde(skip)]
    pub album: Option<String>,
    #[serde(skip)]
    pub artist: Option<String>,
    #[serde(skip)]
    pub tracknumber: Option<u32>,

    #[serde(skip)]
    pub trackcount: Option<u32>,

    #[serde(skip)]
    pub desc_from_file: Option<String>, // description from the description tag

    #[serde(skip)]
    pub z_position: Option<u64>, // position saved via 'z' keymap

    #[serde(skip)]
    pub before_jump_position: Option<u64>, // position saved before jump
}

impl core::fmt::Display for Chapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#?}\r", self.title)
    }
}

impl Chapter {
    pub fn new(
        file_path: &Path,
        path: &Path,
        start_position: Option<u64>,
        length: Option<u64>,
    ) -> Self {
        let filename = file_path.file_name().unwrap().to_str().unwrap().to_owned();
        let mut path = path.to_path_buf();
        path.push(&filename);

        let length = if let Some(length) = length {
            length
        } else {
            let disc = gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(1)).unwrap();
            let disc = disc.discover_uri(&make_uri(&path)).unwrap();
            disc.duration().unwrap().seconds()
        };

        let length_display = formatted_time(length);

        Self {
            filename,
            last_position: 0,
            bookmarks: Vec::new(),
            start_position,
            length,
            length_display,

            description: None,

            m4_title: None,
            m4_tracknumber: None,

            title: None,
            album: None,
            artist: None,
            tracknumber: None,
            trackcount: None,
            desc_from_file: None,
            z_position: None,
            before_jump_position: None,
        }
    }

    /// Make a [Chapter] from m4a/m4b chapter
    pub fn from_m4(
        filename: String,
        title: Option<String>,
        start_position: Option<u64>,
        subtracknumber: Option<u32>,
        length: Option<u64>,
    ) -> Self {
        let (length, length_display) = if let Some(length) = length {
            (length, formatted_time(length))
        } else {
            (0, String::new())
        };

        Self {
            filename,
            last_position: 0,
            bookmarks: Vec::new(),
            start_position,
            length,
            length_display,

            description: None,

            m4_title: title,
            m4_tracknumber: subtracknumber,

            title: None,
            album: None,
            artist: None,
            tracknumber: None,
            trackcount: None,
            desc_from_file: None,
            before_jump_position: None,
            z_position: None,
        }
    }

    /// Gets information about the chapter from tags
    // path is the path to the dir from the user
    pub fn get_info_from_tags(&mut self, path: &Path, disc: &gstreamer_pbutils::Discoverer) {
        let mut path = path.to_path_buf();
        path.push(&self.filename);
        let disc = disc.discover_uri(&make_uri(&path)).unwrap();
        let tags = disc.tags().unwrap();

        self.title = get!(tags, gst::tags::Title);
        self.album = get!(tags, gst::tags::Album);
        self.artist = get!(tags, gst::tags::Artist);
        self.desc_from_file = get!(tags, gst::tags::Description);
        self.trackcount = get!(tags, gst::tags::TrackCount);
        self.tracknumber = get!(tags, gst::tags::TrackNumber);
    }

    pub fn formatted_length(&self) -> String {
        let minutes = self.length / 60;

        if minutes == 0 {
            return format!("{}s", self.length);
        }

        let hours = minutes / 60;

        if hours == 0 {
            let seconds = self.length - minutes * 60;
            return format!("{}m{}s", minutes, seconds);
        }

        let minutes = minutes - hours * 60;
        let seconds = self.length - hours * 3600 - minutes * 60;

        format!("{}h{}m{}s", hours, minutes, seconds)
    }

    pub fn add_bookmark(&mut self, name: String, position: u64) {
        self.bookmarks.push(Bookmark::new(position, self.start_position, name));
    }

    pub fn get_title_or_filename(&self) -> &String {
        if let Some(m4_title) = self.m4_title.as_ref() {
            return m4_title;
        }
        self.title.as_ref().unwrap_or(&self.filename)
    }

    pub fn get_name(&self) -> &str {
        if let Some(m4_title) = self.m4_title.as_deref() {
            return m4_title;
        }
        self.title.as_deref().unwrap_or(&self.filename)
    }

    pub fn update_last_position(&mut self, position: u64) {
        self.last_position = position;
    }

    pub fn update_saved_position(&mut self, position: Option<u64>) {
        self.z_position = position;
    }

    pub fn get_start_position(&self) -> u64 {
        self.start_position.unwrap_or(0)
    }

    pub fn delete_bookmark(&mut self, index: usize) -> Bookmark {
        self.bookmarks.swap_remove(index)
    }

    pub fn get_track_number(&self) -> u32 {
        self.m4_tracknumber.unwrap_or_else(|| self.tracknumber.unwrap())
    }
}

pub fn formatted_time(length: u64) -> String {
    let minutes = length / 60;

    if minutes == 0 {
        return format!("{}s", length);
    }

    let hours = minutes / 60;

    if hours == 0 {
        let seconds = length - minutes * 60;
        return format!("{}m{}s", minutes, seconds);
    }

    let minutes = minutes - hours * 60;
    let seconds = length - hours * 3600 - minutes * 60;

    format!("{}h{}m{}s", hours, minutes, seconds)
}
