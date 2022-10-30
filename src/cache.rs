use tui::widgets::ListItem;

use crate::data::chapter::{formatted_time,
                           Chapter};

pub struct Cache<'a> {
    // should_recalculate: bool,
    pub info_info: Vec<ListItem<'a>>,

    pub pl_bks_count: Option<Vec<ListItem<'a>>>,
    pub pl_chooses: Option<Vec<ListItem<'a>>>,
    pub pl_lengths: Option<Vec<ListItem<'a>>>,
    pub pl_percentages: Option<Vec<ListItem<'a>>>,
    pub pl_titles: Option<Vec<ListItem<'a>>>,

    pub abs_now: Option<String>,
    pub formatted_abs_now: Option<String>,
    pub formatted_length: Option<String>,
    pub formmated_now: Option<String>,

    pub bk_list0: Option<Vec<ListItem<'a>>>,
    pub bk_list1: Option<Vec<ListItem<'a>>>,

    pub keybidings_list0: Option<Vec<ListItem<'a>>>,
    pub keybidings_list1: Option<Vec<ListItem<'a>>>,
}

impl Cache<'_> {
    pub fn new() -> Self {
        Cache {
            info_info: vec![
                ListItem::new("Chapter: "),
                ListItem::new("Book: "),
                ListItem::new("Author: "),
                ListItem::new("File name: "),
                ListItem::new("Dir: "),
                ListItem::new("Speed: "),
                ListItem::new("StartPos: "),
                ListItem::new("AbsPosForm: "),
                ListItem::new("AbsPos: "),
            ],
            pl_bks_count: None,
            pl_chooses: None,
            pl_lengths: None,
            pl_percentages: None,
            pl_titles: None,

            abs_now: None,
            formatted_abs_now: None,
            formatted_length: None,
            formmated_now: None,

            bk_list0: None,
            bk_list1: None,

            keybidings_list0: None,
            keybidings_list1: None,
        }
    }

    pub fn on_tick(&mut self, chapter: &Chapter, position: u64, abs_position: u64) {
        if self.formatted_length.is_none() {
            self.formatted_length = Some(chapter.formatted_length())
        }

        self.formmated_now = Some(formatted_time(position));
        self.formatted_abs_now = Some(formatted_time(abs_position));
        self.abs_now = Some(abs_position.to_string());
    }

    /// invalidates the cache for things related to bookmarks
    pub fn invalidate_bks(&mut self) {
        self.bk_list0 = None;
        self.bk_list1 = None;
        self.pl_bks_count = None;
    }

    /// invalidates the keybindings list
    pub fn invalidate_kbs(&mut self) {
        self.keybidings_list0 = None;
        self.keybidings_list1 = None;
    }

    /// invalidates the things in the playlist
    pub fn invalidate_pls(&mut self) {
        self.pl_bks_count = None;
        self.pl_chooses = None;
        self.pl_lengths = None;
        self.pl_percentages = None;
        self.pl_titles = None;
    }
}

impl Default for Cache<'_> {
    fn default() -> Self {
        Self::new()
    }
}
