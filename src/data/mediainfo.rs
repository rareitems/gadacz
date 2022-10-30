use std::collections::HashSet;
use std::path::{Path,
                PathBuf};
use std::time::Duration;

use color_eyre::{Help,
                 Report};
use gstreamer as gst;
use serde::{Deserialize,
            Serialize};

use super::chapter::Chapter;
use crate::data::chapter::formatted_time;
use crate::data::make_uri;

const M4_EXTENSIONS: [&str; 2] = ["m4a", "m4b"];
const VALID_EXTENSIONS: [&str; 8] = ["flac", "m4a", "m4b", "mp3", "mp4", "ogg", "opus", "wav"];

type EyreResult<T> = color_eyre::Result<T>;

#[derive(Debug)]
enum Error {
    NoExtentsion(PathBuf),
    FromIo(std::io::Error),
    MissingFiles(Vec<PathBuf>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoExtentsion(s) => write!(f, "{s:#?} has no extension"),
            Error::FromIo(io) => io.fmt(f),
            Error::MissingFiles(v) => write!(
                f,
                "{v:?} those filename exist in the cached json 'gadacz_data.json' file but are \
                 not present in the directory"
            ),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(io: std::io::Error) -> Self {
        Error::FromIo(io)
    }
}

impl std::error::Error for Error {}

/// Information about book
#[derive(Debug, Deserialize, Serialize)]
pub struct MediaInfo {
    #[serde(skip)]
    pub chaptercount: usize,

    #[serde(skip)]
    pub path: PathBuf, // path for the currently playing book

    pub speed: f64,
    pub volume: f64,
    pub last_chapter: usize,    // index of the last played chapter
    pub chapters: Vec<Chapter>, // list of chapters for the given book
}

impl MediaInfo {
    /// Read cached `MediaInfo` from ``gadacz_data.json`` under the given path, if it doesn't exist
    /// scan the `path` and create new ``MediaInfo``
    pub fn from_cache_or_new(path: &Path) -> EyreResult<Self> {
        if let Some(file) =
            path.read_dir()?.find(|x| x.as_ref().unwrap().file_name() == "gadacz_data.json")
        {
            let mut mi = MediaInfo::from_json(path, file?)?;
            mi.path = path.to_owned(); // path is not being cached
            Ok(mi)
        } else {
            MediaInfo::new(path)
        }
    }

    /// Check if there is a `gadacz_data.json` under the `path`
    /// If true, read the data, add new file under the path, rescan all the all `Chapters`
    /// If false, create a  new `Mediainfo` from the files under the `path`
    pub fn from_json(path: &Path, file: std::fs::DirEntry) -> EyreResult<Self> {
        let data = std::fs::read_to_string(file.path())?;
        let mut mediainfo: MediaInfo = serde_json::from_str(&data)?;

        let content = scan_dir(path)?;

        // Check if all files in the cached json are actually present in the directory
        let c = mediainfo
            .chapters
            .iter()
            .map(|it| {
                let mut p = std::path::PathBuf::new();
                p.push(path);
                p.push(std::path::Path::new(&it.filename));
                p
            })
            .filter(|it| !content.contains(it))
            .collect::<Vec<_>>();

        if !c.is_empty() {
            return EyreResult::Err(Report::new(Error::MissingFiles(c)).suggestion(
                "Restore those files or remove/ change files names manully in the \
                 gadacz_data.json file",
            ));
        }

        // filter out files that are already inside the cached mediainfo and add the rest to the
        // cached mediainfo chapters
        let names: Vec<_> = mediainfo.chapters.iter().map(|it| it.filename.as_str()).collect();
        let c: Vec<_> = content
            .iter()
            .filter(|it| !names.contains(&it.file_name().unwrap().to_str().unwrap()))
            .collect();

        // handle new m4a / m4b files
        {
            let mut new_chapters = Vec::new();
            for it in &c {
                // unwrapping extension() without any special notifaction for the user since it was
                // already done in `scan_dir`
                let ext = it.extension().unwrap().to_str().unwrap();
                if M4_EXTENSIONS.contains(&ext) {
                    let mut h = handle_m4(it, path)?;
                    new_chapters.append(&mut h);
                }
            }

            mediainfo.chapters.append(&mut new_chapters);
        }

        // handle new non-mp4 files
        {
            let mut new_chapters: Vec<_> = c
                .iter()
                .filter_map(|it| {
                    // unwrapping extension() without any special notifaction for the user since it
                    // was already done in `scan_dir`
                    let ext = it.extension().unwrap().to_str().unwrap();
                    if !M4_EXTENSIONS.contains(&ext) {
                        Some(Chapter::new(it, path, None, None))
                    } else {
                        None
                    }
                })
                .collect();

            mediainfo.chapters.append(&mut new_chapters);
        }

        mediainfo.chaptercount = mediainfo.chapters.len();
        mediainfo.scan_chapters(path);
        mediainfo.sort_chapters();

        Ok(mediainfo)
    }

    pub fn new(path: &Path) -> EyreResult<Self> {
        let content = scan_dir(path)?;

        if content.is_empty() {
            eyre::bail!("Given directory is empty or it has no files with valid extensions.")
        }

        // handle non m4a/m4b files
        let mut chapters: Vec<_> = content
            .iter()
            .filter_map(|it| {
                // unwrapping extension() without any special notifaction for the user since it was
                // already done in `scan_dir`
                let ext = it.extension().unwrap().to_str().unwrap();
                if M4_EXTENSIONS.contains(&ext) {
                    None
                } else {
                    Some(Chapter::new(it, path, None, None))
                }
            })
            .collect();

        // handle m4a/m4b files
        for it in &content {
            // unwrapping extension() without any special notifaction for the user since it was
            // already done in `scan_dir`
            let ext = it.extension().unwrap().to_str().unwrap();
            if M4_EXTENSIONS.contains(&ext) {
                let mut h = handle_m4(it, path)?;
                chapters.append(&mut h);
            }
        }

        let mut mediainfo = Self {
            last_chapter: 0,
            speed: 1.0,
            volume: 0.5,
            path: path.to_owned(),
            chaptercount: chapters.len(),
            chapters,
        };

        mediainfo.scan_chapters(path);
        mediainfo.sort_chapters();

        Ok(mediainfo)
    }

    /// Iterate over all the 'chapters' in 'self' scanning each 'Chapter' for gstreamer tags
    fn scan_chapters(&mut self, path: &Path) {
        let disc = gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(1)).unwrap();
        for it in &mut self.chapters {
            it.get_info_from_tags(path, &disc)
        }
    }

    /// Sort by track number if tracknumber is not avaiable sort by title or filename
    fn sort_chapters(&mut self) {
        self.chapters.sort_by(|a, b| {
            match (a.tracknumber.is_some(), b.tracknumber.is_some()) {
                (true, true) => match a.tracknumber.cmp(&b.tracknumber) {
                    std::cmp::Ordering::Less => std::cmp::Ordering::Less,
                    std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
                    std::cmp::Ordering::Equal => {
                        match (a.m4_tracknumber.is_some(), b.m4_tracknumber.is_some()) {
                            (true, true) => a.m4_tracknumber.cmp(&b.m4_tracknumber),
                            _ => std::cmp::Ordering::Equal,
                        }
                    }
                },
                // if any of the track numbers is not avaiable sort by title
                // if title is not avaible sort by filename
                _ => match (a.title.is_some(), b.title.is_some()) {
                    (true, true) => a.title.cmp(&b.title),
                    (true, false) => a.title.as_ref().unwrap().cmp(&b.filename),
                    (false, true) => a.filename.cmp(b.title.as_ref().unwrap()),
                    (false, false) => a.filename.cmp(&b.filename),
                },
            }
        });
    }

    pub fn save_to_file(&self) -> EyreResult<()> {
        let json_as_string = serde_json::to_string(&self)?;
        let mut path = self.path.to_path_buf();
        path.push("gadacz_data.json");
        let mut file = std::fs::File::create(path)?;
        std::io::Write::write_all(&mut file, json_as_string.as_bytes())?;
        Ok(())
    }
}

fn handle_m4(file_path: &PathBuf, path: &Path) -> EyreResult<Vec<Chapter>> {
    let mp4_tag = mp4ameta::Tag::read_from_path(file_path)?;
    let mut chapters: Vec<_> = mp4_tag.chapters().collect();
    let mut m4_chapters: Vec<Chapter> = Vec::new();

    if chapters.is_empty() {
        // it is a m4a/b file but doesn't have chapters so add the whole file
        m4_chapters.push(Chapter::new(file_path, path, None, None));
    } else {
        // [mp4ameta] can sometimes produce duplicates
        {
            let mut hash_set: HashSet<(&String, Duration)> =
                HashSet::with_capacity(chapters.len() / 2);
            chapters.retain(|it| {
                if hash_set.contains(&(&it.title, it.start)) {
                    false
                } else {
                    hash_set.insert((&it.title, it.start));
                    true
                }
            });
        }

        for (index, it) in chapters.iter().enumerate() {
            m4_chapters.push(Chapter::from_m4(
                file_path.file_name().unwrap().to_str().unwrap().to_string(),
                Some(it.title.clone()),
                Some(it.start.as_secs_f64().ceil() as u64),
                Some(index as u32),
                None,
            ))
        }

        // assigning length of each m4 chapter
        let mut iter = m4_chapters.iter_mut().peekable();
        while let Some(it) = iter.next() {
            if let Some(peek) = iter.peek() {
                if it.filename == peek.filename {
                    assert!(it.length == 0);
                    it.length = (peek.start_position).unwrap() - (it.start_position).unwrap();
                    it.length_display = formatted_time(it.length);
                }
            } else {
                // case at the end of the file
                // grabbing the length of the whole file
                let disc =
                    gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(1)).unwrap();
                let disc = disc.discover_uri(&make_uri(file_path)).unwrap();
                let length = disc.duration().unwrap().seconds();

                // figuring out the length of that chapter from the length of the whole
                // file and the m4 chapter starter position
                it.length = length - (it.start_position).unwrap();
                it.length_display = formatted_time(it.length);
            }
        }
    }

    Ok(m4_chapters)
}

/// scan the dir under the path for files with valid extensions
/// Returns a Vec with
fn scan_dir(path: &Path) -> Result<Vec<PathBuf>, Error> {
    path.read_dir()?
        .filter_map(|it| {
            let it = match it {
                std::result::Result::Ok(ok) => ok,
                std::result::Result::Err(err) => return Some(Err(Error::FromIo(err))),
            };

            let file = it.path();

            let ext = if let Some(ext) = file.extension() {
                ext.to_str().unwrap()
            } else {
                return Some(Err(Error::NoExtentsion(file)));
            };

            if VALID_EXTENSIONS.contains(&ext) {
                return Some(Ok(file));
            }
            None
        })
        .collect::<Result<Vec<PathBuf>, Error>>()
}
