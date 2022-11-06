use tui::backend::Backend;
use tui::layout::{Alignment,
                  Constraint,
                  Direction,
                  Layout,
                  Rect};
use tui::style::{Color,
                 Modifier,
                 Style};
use tui::widgets::{Block,
                   BorderType,
                   Borders,
                   Gauge,
                   List,
                   ListItem,
                   Paragraph,
                   Wrap};

use crate::data::mediainfo::MediaInfo;
use crate::App;

pub mod popouts;

pub struct Ui {
    pub chapter_bar: u16,
    pub volume_bar: u16,

    pub yn_prompt: &'static str, // text for yes/no prompt

    // pub keybindings_list: Vec<ListItem<'static>>,
    pub keybindings_list: Vec<&'static str>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            chapter_bar: 0,
            volume_bar: 50,
            yn_prompt: "NONE",
            keybindings_list: vec![
                "? : List all shortcuts",
                "= : Increase volume by 5%",
                "- : Decrease volume by 5%",
                "v : Set arbitrary volume",
                "; : Jump to arbitrary position",
                "a : Add new bookmark",
                "b : Bookmark menu (only this chapter)",
                "B : Bookmark menu (all chapters)",
                "h : Move 5 seconds backwards",
                "j : Move 1 chapter forwards",
                "k : Move 1 chapter backwards",
                "l : Move 5 seconds forwards",
                "p : Toggle pause and play",
                "q : Quit",
                "r : Reset progress of the chapter",
                "s : Increase speed by 0.25",
                "S : Decrease speed by 0.25",
                "C-s : Set arbitrary speed",
                "m : Mark position for a bookmark",
                "M : Create bookmark at the marked position",
                "d : Set description for the current chapter",
                "D : Delete description for the current chapter",
                "z : Save position",
                "Z : Restore saved position",
                "F : Set 100% completion and move to next chapter",
                ": : Go to the position before the jump or bookmark(for current chapter) change",
                ", : Go to position and chapter before the bookmark(for all chapters) change",
            ],
        }
    }

    pub fn on_tick(&mut self, volume: f64, position: u64, length: u64) {
        self.volume_bar = (volume * 100.0) as u16;
        self.chapter_bar = ((position as f64 / length as f64) * 100.0) as u16;
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

pub fn render<'a, B: Backend>(f: &mut tui::Frame<B>, app: &mut App, mediainfo: &'a MediaInfo) {
    let current_chapter = app.get_current_chapter(mediainfo);

    // Splitting the space into 3 parts
    let main_chunk = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(65), // info + playlist
            Constraint::Percentage(30), // bookmarks + keybinds
            Constraint::Percentage(5),  // showing messages
        ])
        .split(f.size());

    let block = Block::default()
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    f.render_widget(block, main_chunk[0]);

    // Top Block Split
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Info
            Constraint::Percentage(40), // Playlist
        ])
        .split(main_chunk[0]);

    // Info Block
    let block = Block::default()
        .title_alignment(Alignment::Center)
        .title(tui::text::Span::styled("Info", Style::default().fg(Color::White)));
    f.render_widget(block, top_chunks[0]);

    let info_split = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(2),
            Constraint::Percentage(15),
            Constraint::Percentage(2),
            Constraint::Percentage(15),
            Constraint::Percentage(2),
            Constraint::Percentage(10),
        ])
        .split(top_chunks[0]);

    let info_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Max(12), Constraint::Percentage(40)])
        .split(info_split[0]);

    let info_info = List::new(&*app.cache.info_info).style(Style::default().fg(Color::White));
    f.render_widget(info_info, info_chunks[0]);

    let items = vec![
        ListItem::new(current_chapter.get_name()),
        ListItem::new(current_chapter.album.as_deref().unwrap_or("None")),
        ListItem::new(current_chapter.artist.as_deref().unwrap_or("None")),
        ListItem::new(&*current_chapter.filename),
        ListItem::new(mediainfo.path.display().to_string()),
        ListItem::new(mediainfo.speed.to_string()),
        ListItem::new(current_chapter.start_position.unwrap_or(0).to_string()),
        ListItem::new(app.cache.formatted_abs_now.as_deref().unwrap_or("None")),
        ListItem::new(app.cache.abs_now.as_deref().unwrap_or("None")),
    ];
    let list = List::new(items).style(Style::default().fg(Color::White));
    f.render_widget(list, info_chunks[1]);

    // progress bar
    let chapter_bar = Gauge::default()
        .block(Block::default().borders(Borders::NONE).title("Chapter Progress"))
        .gauge_style(
            Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC),
        )
        .label(format!(
            "{} / {}",
            app.cache.formmated_now.as_ref().unwrap_or(&"".to_string()),
            current_chapter.length_display
        ))
        .percent(app.ui.chapter_bar);
    f.render_widget(chapter_bar, info_split[2]);

    let volume_bar = Gauge::default()
        .block(Block::default().borders(Borders::NONE).title("Volume"))
        .gauge_style(
            Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC),
        )
        .percent(app.ui.volume_bar);
    f.render_widget(volume_bar, info_split[4]);

    // extra information
    {
        if let Some(pos) = app.marked_position {
            let info = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(13), Constraint::Percentage(50)])
                .split(info_split[6]);

            let left_items = vec![ListItem::new("Marked Position: ")];
            let left_list = List::new(left_items).style(Style::default().fg(Color::White));

            let right_item = vec![ListItem::new(pos.to_string())];
            let right_list = List::new(right_item).style(Style::default().fg(Color::White));

            f.render_widget(left_list, info[0]);
            f.render_widget(right_list, info[1]);
        }
    }

    // Playlist space
    let block = Block::default()
        .title_alignment(Alignment::Center)
        .title(tui::text::Span::styled("Playlist", Style::default().fg(Color::White)));
    f.render_widget(block, top_chunks[1]);

    let playlist_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Percentage(6),  // percetage of "watched"
            Constraint::Percentage(4),  // if chosen
            Constraint::Percentage(75), // name of the song
            Constraint::Percentage(1),  // empty space
            Constraint::Percentage(10), // length
            Constraint::Percentage(1),  // empty space
            Constraint::Percentage(2),  // number of bookmarks
        ])
        .split(top_chunks[1]);

    let playlist_height: usize = playlist_chunk[0].height.into();
    let number_of_rest_tracks = mediainfo.chaptercount - (app.current_chapter_index + 1);

    // calculate how many chapters to skip for rendering inside the playlist chunk
    let skip = if (app.current_chapter_index + 1) >= playlist_height {
        let s = (app.current_chapter_index + 1) - playlist_height;

        // add different ammount of padding (so it always shows two tracks at the bottom and
        // fills ups the playlist chunk) according to how many tracks are there left
        if number_of_rest_tracks == 0 {
            s
        } else if number_of_rest_tracks == 1 {
            1 + s
        } else {
            2 + s
        }
    } else { usize::from(playlist_height - (app.current_chapter_index + 1) == 1) };

    // NumberType::from(playlist_height - (app.current_chapter_index + 1) == 1)

    if let Some(pl_percentages) = app.cache.pl_percentages.as_ref() {
        let list = List::new(&**pl_percentages);
        f.render_widget(list, playlist_chunk[0]);
    } else {
        let percentages: Vec<_> = mediainfo
            .chapters
            .iter()
            .skip(skip)
            .take(playlist_height)
            .map(|x| {
                let perc = {
                    let p = (((x.last_position as f64 - x.start_position.unwrap_or(0) as f64)
                        / x.length as f64)
                        * 100.0)
                        .ceil() as u16;

                    if p >= 100 { 100 } else { p }
                };
                let string = format!("{perc}%");

                if perc >= 75 {
                    ListItem::new(string).style(Style::default().fg(Color::Green))
                } else if perc >= 50 {
                    ListItem::new(string).style(Style::default().fg(Color::LightGreen))
                } else if perc >= 25 {
                    ListItem::new(string).style(Style::default().fg(Color::Gray))
                } else {
                    ListItem::new(string).style(Style::default().fg(Color::DarkGray))
                }
            })
            .collect();
        app.cache.pl_percentages = Some(percentages);
        let list = List::new(&**app.cache.pl_percentages.as_ref().unwrap());
        f.render_widget(list, playlist_chunk[0]);
    }

    if let Some(chooses) = app.cache.pl_chooses.as_ref() {
        let choses_list = List::new(&**chooses);
        f.render_widget(choses_list, playlist_chunk[1]);
    } else {
        let chooses: Vec<_> = mediainfo
            .chapters
            .iter()
            .skip(skip)
            .take(playlist_height)
            .enumerate()
            .map(|(i, _)| {
                if skip + i == app.current_chapter_index {
                    ListItem::new(">>> ").style(Style::default().fg(Color::Red))
                } else {
                    ListItem::new("    ")
                }
            })
            .collect();
        app.cache.pl_chooses = Some(chooses);
        let choses_list = List::new(&**app.cache.pl_chooses.as_ref().unwrap());
        f.render_widget(choses_list, playlist_chunk[1]);
    }

    if let Some(titles) = app.cache.pl_titles.as_ref() {
        let list = List::new(&**titles).style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[2]);
    } else {
        let titles: Vec<_> = mediainfo
            .chapters
            .iter()
            .skip(skip)
            .take(playlist_height)
            .map(|x| {
                ListItem::new({
                    if let Some(desc) = &x.description {
                        format!("{} [{}]", x.get_title_or_filename(), desc)
                    } else {
                        x.get_title_or_filename().clone()
                    }
                })
            })
            .collect();
        app.cache.pl_titles = Some(titles);
        let list = List::new(&**app.cache.pl_titles.as_ref().unwrap())
            .style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[2]);
    }

    if let Some(lengths) = app.cache.pl_lengths.as_ref() {
        let list = List::new(&**lengths).style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[4]);
    } else {
        let lengths: Vec<_> = mediainfo
            .chapters
            .iter()
            .skip(skip)
            .take(playlist_height)
            .map(|x| ListItem::new(x.length_display.clone()))
            .collect();
        app.cache.pl_lengths = Some(lengths);
        let list = List::new(&**app.cache.pl_lengths.as_ref().unwrap())
            .style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[4]);
    }

    if let Some(pl_bks_count) = app.cache.pl_bks_count.as_ref() {
        let list = List::new(&**pl_bks_count).style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[6]);
    } else {
        let bks_count: Vec<_> = mediainfo
            .chapters
            .iter()
            .skip(skip)
            .take(playlist_height)
            .map(|x| ListItem::new(x.bookmarks.len().to_string()))
            .collect();
        app.cache.pl_bks_count = Some(bks_count);
        let list = List::new(&**app.cache.pl_bks_count.as_ref().unwrap())
            .style(Style::default().fg(Color::White));
        f.render_widget(list, playlist_chunk[6]);
    }

    let bk_help_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunk[1]);

    // bookmarks
    {
        let bk_block = Block::default()
            .borders(tui::widgets::Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .border_type(BorderType::Thick)
            .title_alignment(Alignment::Left)
            .title(tui::text::Span::styled("Bookmarks", Style::default().fg(Color::White)));

        let bk_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(bk_help_chunk[0]);

        f.render_widget(bk_block, bk_help_chunk[0]);

        let bk_lists_block = Block::default()
            .borders(tui::widgets::Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray));

        let bk_lists_block_no_border = Block::default();

        let bk_height = bk_chunks[0].height as usize;

        if app.cache.bk_list0.is_none() {
            let bk0: Vec<_> = current_chapter
                .bookmarks
                .iter()
                .take(bk_height)
                .map(|it| ListItem::new(it.formatted_position.clone()))
                .collect();
            app.cache.bk_list0 = Some(bk0);
        }

        if app.cache.bk_list1.is_none() {
            let bk1: Vec<_> = current_chapter
                .bookmarks
                .iter()
                .skip(bk_height)
                .take(bk_height)
                .map(|it| ListItem::new(it.formatted_position.clone()))
                .collect();
            app.cache.bk_list1 = Some(bk1);
        }

        let bookmarks_list = List::new(&**app.cache.bk_list0.as_ref().unwrap())
            .block(bk_lists_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(bookmarks_list, bk_chunks[0]);

        let bookmarks_list = List::new(&**app.cache.bk_list1.as_ref().unwrap())
            .block(bk_lists_block_no_border)
            .style(Style::default().fg(Color::White));
        f.render_widget(bookmarks_list, bk_chunks[1]);
    }

    let help_block = Block::default()
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .border_type(BorderType::Thick)
        .title_alignment(Alignment::Right)
        .title(tui::text::Span::styled("Keybindings", Style::default().fg(Color::White)));
    f.render_widget(help_block, bk_help_chunk[1]);

    let help_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            // Constraint::Percentage(33),
        ])
        .split(bk_help_chunk[1]);

    let height = help_chunks[0].height as usize;

    let help_blocks = Block::default()
        .borders(tui::widgets::Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray))
        .border_type(BorderType::Rounded);

    if let Some(list0) = app.cache.keybidings_list0.as_ref() {
        f.render_widget(
            List::new(&**list0).block(help_blocks).style(Style::default().fg(Color::White)),
            help_chunks[0],
        );
    } else {
        let list: Vec<_> =
            app.ui.keybindings_list.iter().take(height).map(|it| ListItem::new(*it)).collect();
        app.cache.keybidings_list0 = Some(list);
    }

    let help_blocks_no_border_right = Block::default();

    if let Some(list1) = app.cache.keybidings_list1.as_ref() {
        f.render_widget(
            List::new(&**list1)
                .block(help_blocks_no_border_right)
                .style(Style::default().fg(Color::White)),
            help_chunks[1],
        );
    } else {
        let list: Vec<_> = app
            .ui
            .keybindings_list
            .iter()
            .skip(height)
            .take(height)
            .map(|it| ListItem::new(*it))
            .collect();
        app.cache.keybidings_list1 = Some(list);
    }

    // Bottom block - Help
    let help_block = Block::default()
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    // .title("Help");

    let paragraph = Paragraph::new(app.msgs.current.as_deref().unwrap_or(""))
        .block(help_block)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, main_chunk[2]);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rec_perc(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect_flat(flat_x: u16, flat_y: u16, r: Rect) -> Option<Rect> {
    if r.width < flat_x || r.height < flat_y {
        return None;
    }

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height - flat_y) / 2),
            Constraint::Length(flat_y),
            Constraint::Length((r.height - flat_y) / 2),
        ])
        .split(r);

    Some(
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((r.width - flat_x) / 2 - 2),
                Constraint::Length(flat_x + 2), // 2 for fitting correctly
                Constraint::Length((r.width - flat_x) / 2 - 2),
            ])
            .split(popup_layout[1])[1],
    )
}
