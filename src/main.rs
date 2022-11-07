//! TUI Audiobook player

use std::io;
use std::ops::ControlFlow;
use std::time::{Duration,
                Instant};

// use anyhow::Result;
use cache::Cache;
use color_eyre::Help;
use crossterm::event::{self,
                       DisableMouseCapture,
                       EnableMouseCapture,
                       Event,
                       KeyCode,
                       KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode,
                          enable_raw_mode,
                          EnterAlternateScreen,
                          LeaveAlternateScreen};
use data::chapter::Chapter;
use data::mediainfo::MediaInfo;
use gst::prelude::*;
use gst::MessageType;
use gstreamer as gst;
use msgs::Msgs;
use player::Player;
use tui::backend::{Backend,
                   CrosstermBackend};
use tui::Terminal;
use ui::{render,
         Ui};

pub mod cache;
pub mod data; // Handling data
pub mod helpers;
pub mod msgs;
pub mod player; // Handling playing audio
pub mod ui; // Handling rendering UI

/// Assuming that [`ControlFlow`] has unit type inside of it,  matches 'x' on [`ControlFlow`] enum
/// and continues or breaks
macro_rules! match_cflow {
    ($x:expr) => {
        match $x {
            ControlFlow::Continue(()) => continue,
            ControlFlow::Break(()) => break,
        }
    };
}

fn print_help_menu() {
    println!(
        "TUI Audiobook player

USAGE:
gadacz [OPTIONS] [PATH]

OPTIONS:
-a, --antispoiler   Turn on antispoiler mode (hides the names of the chapters, number of chapters)
-h, --helpr         Pring help information
"
    );
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    gst::init()?;

    let mut args = std::env::args();
    args.next();

    let args: Vec<String> = args.collect();

    let mut path: Option<&str> = None;
    let mut antispoiler_mode: bool = false;

    if args.is_empty() {
        return Err(eyre::eyre!("No argument provided")
            .suggestion("Provide a path to the directory you want to play."));
    }

    for args in &args {
        match args.as_str() {
            "--help" | "-h" => {
                print_help_menu();
                return Ok(());
            }
            "--antispoiler" | "-a" => {
                antispoiler_mode = true;
            }
            p => path = Some(p),
        }
    }

    let path = match path {
        Some(path) => std::path::PathBuf::from(&path).canonicalize()?,
        None => {
            return Err(eyre::eyre!("No path provided")
                .suggestion("Provide a path to the directory you want to play."));
        }
    };

    let mut mediainfo = MediaInfo::from_cache_or_new(&path)?;
    mediainfo.sort_all_bk();

    mediainfo.is_antispoiler = mediainfo.is_antispoiler || antispoiler_mode;

    let player = Player::default();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    enable_raw_mode()?;
    terminal.hide_cursor()?;

    let mut app = App::new(player);
    app.load_chapter(mediainfo.last_chapter, &mediainfo);

    let res = run_app(&mut terminal, &mut app, mediainfo);

    // restore terminal
    let err = disable_raw_mode();
    let err1 = execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture);

    res?.save_to_file()?; // should retry to save the file or some prompt?

    err?;
    err1?;

    app.player.null();

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: MediaInfo,
) -> color_eyre::Result<MediaInfo> {
    let mut mediainfo = mediainfo;

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(33);
    app.current_chapter_index = mediainfo.last_chapter;

    let mut last_time_saved = Instant::now();
    let dur_between_saves = Duration::from_secs(60 * 5);

    let mut last_time_percentage_updated = Instant::now();
    let dur_between_percentage_updates = Duration::from_secs(30);

    loop {
        terminal.draw(|f| render(f, app, &mediainfo))?;
        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('?') => ui::popouts::help_menu::run(
                        terminal,
                        app,
                        &mediainfo,
                        &mut last_tick,
                        tick_rate,
                    )?,

                    KeyCode::Char('p' | ' ') => actions::toggle_play(app),

                    KeyCode::Char('q') => match_cflow!(actions::quit(app, &mut mediainfo)),

                    KeyCode::Char('j') | KeyCode::Down => {
                        match_cflow!(actions::next_chapter(app, &mut mediainfo, true));
                    }

                    KeyCode::Char('d') => actions::add_description(
                        app,
                        &mut mediainfo,
                        terminal,
                        &mut last_tick,
                        tick_rate,
                    )?,

                    KeyCode::Char('D') => actions::delete_description(app, &mut mediainfo),

                    KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                        mediainfo.is_antispoiler = !mediainfo.is_antispoiler;
                        app.cache.invalidate_pls();
                    }

                    KeyCode::Char('a') => match_cflow!(actions::add_bookmark(
                        app,
                        &mut mediainfo,
                        terminal,
                        &mut last_tick,
                        tick_rate
                    )?),

                    KeyCode::Char('m') => actions::add_mark(app),

                    KeyCode::Char('M') => actions::add_bookmark_at_mark(
                        app,
                        &mut mediainfo,
                        terminal,
                        &mut last_tick,
                        tick_rate,
                    )?,

                    KeyCode::Char('k') | KeyCode::Up => actions::prev_chapter(app, &mut mediainfo),

                    KeyCode::Char(',') => {
                        actions::restore_pos_and_chap_before_jump(app, &mut mediainfo)
                    }

                    KeyCode::Char(';') => actions::move_to_arbitrary_position(
                        app,
                        &mut mediainfo,
                        terminal,
                        &mut last_tick,
                        tick_rate,
                    )?,

                    KeyCode::Char(':') => actions::restore_pos_before_jump(app, &mut mediainfo),

                    KeyCode::Char('l') | KeyCode::Right => {
                        actions::move_forward(app, &mut mediainfo);
                    }

                    KeyCode::Char('h') | KeyCode::Left => {
                        actions::move_backward(app, &mut mediainfo);
                    }

                    KeyCode::Char('=' | '+') => {
                        actions::increase_volume(app, &mut mediainfo);
                    }

                    KeyCode::Char('-') => {
                        actions::descrease_volume(app, &mut mediainfo);
                    }

                    KeyCode::Char('F') => {
                        app.player.pause();

                        let chap = app.get_mut_current_chapter(&mut mediainfo);

                        if let Some(start_position) = chap.start_position {
                            // if `start_position` exists the file is a mp4 file so it has to be
                            // handled bit differently
                            chap.update_last_position(start_position + chap.length);
                        } else {
                            chap.update_last_position(chap.length);
                        }

                        if let ControlFlow::Continue(_) =
                            actions::next_chapter(app, &mut mediainfo, false)
                        {
                            continue;
                        }
                    }

                    KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                        if let Some(input) = ui::popouts::input::run(
                            terminal,
                            app,
                            &mediainfo,
                            &mut last_tick,
                            tick_rate,
                            "Input speed. Bigger than 0.0",
                            None,
                            30,
                        )? {
                            if let Ok(speed) = input.parse::<f64>() {
                                mediainfo.speed = speed;
                                if app.player.set_speed(speed).is_err() {
                                    app.msgs.push("Couldn't set the speed".into());
                                }
                            } else {
                                app.msgs.push("Invalid input".into());
                                continue;
                            }
                        } else {
                            app.msgs.push("Cancelled setting speed.".into());
                            continue;
                        };
                    }

                    KeyCode::Char('s') => actions::increase_speed(app, &mut mediainfo),

                    KeyCode::Char('S') => actions::descrease_speed(app, &mut mediainfo),

                    KeyCode::Char('r') => {
                        let res = ui::popouts::yn::run(
                            terminal,
                            app,
                            &mediainfo,
                            &mut last_tick,
                            tick_rate,
                            "Are you sure you want to reset the current chapter? y/n",
                        )?;
                        if res {
                            app.player
                                .seek_seconds(
                                    app.get_current_chapter(&mediainfo).start_position.unwrap_or(0),
                                    mediainfo.speed,
                                )
                                .unwrap();
                        }
                    }

                    KeyCode::Char('b') => {
                        ui::popouts::bookmarks::run(
                            terminal,
                            app,
                            &mut mediainfo,
                            &mut last_tick,
                            tick_rate,
                        )?;
                    }

                    KeyCode::Char('B') => ui::popouts::all_bookmarks::run(
                        terminal,
                        app,
                        &mut mediainfo,
                        &mut last_tick,
                        tick_rate,
                    )?,

                    KeyCode::Char('v') => {
                        if let Some(input) = ui::popouts::input::run(
                            terminal,
                            app,
                            &mediainfo,
                            &mut last_tick,
                            tick_rate,
                            "Input volume. Between 0 and 100",
                            None,
                            31,
                        )? {
                            if let Ok(volume) = input.parse::<u64>() {
                                let v = volume as f64 / 100.0;
                                mediainfo.volume = v;
                                app.player.set_volume(v);
                            } else {
                                app.msgs.push("Invalid input".into());
                                continue;
                            }
                        } else {
                            app.msgs.push("Cancelled adding a bookmark".into());
                            continue;
                        };
                    }

                    // Saves the position
                    KeyCode::Char('z') => {
                        if let Some(pos) = app.player.get_position_sec() {
                            app.get_mut_current_chapter(&mut mediainfo)
                                .update_saved_position(Some(pos));
                        } else {
                            app.msgs.push("Couldn't get the position".into());
                            continue;
                        }
                    }

                    KeyCode::Char('Z') => {
                        if let Some(pos) = app.get_current_chapter(&mediainfo).z_position {
                            if app.player.seek_seconds(pos, mediainfo.speed).is_err() {
                                app.msgs.push(
                                    format!("Couldn't move the saved position at {}", pos).into(),
                                );
                            }
                        } else {
                            app.msgs
                                .push("You do not have a saved position for this chapter".into());
                        }
                    }

                    KeyCode::Char('0') => {}

                    _ => continue,
                },

                Event::Resize(_, _) => {
                    app.cache.invalidate_bks();
                    app.cache.invalidate_kbs();
                    app.cache.invalidate_pls();
                }

                Event::Mouse(mouse) => match mouse.kind {
                    event::MouseEventKind::Down(_) => actions::toggle_play(app),

                    event::MouseEventKind::ScrollUp => {
                        actions::increase_volume(app, &mut mediainfo);
                    }

                    event::MouseEventKind::ScrollDown => {
                        actions::descrease_volume(app, &mut mediainfo);
                    }

                    _ => continue,
                },

                _ => continue,
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let now = Instant::now();
            app.on_tick(&mut mediainfo);
            last_tick = now;

            if last_time_saved.elapsed() >= dur_between_saves {
                last_time_saved = now;
                if let Some(pos) = app.player.get_position_sec() {
                    app.get_mut_current_chapter(&mut mediainfo).update_last_position(pos);
                } else {
                    app.msgs.push("Couldn't get the position".into());
                    continue;
                }

                match mediainfo.save_to_file() {
                    Ok(_) => app.msgs.push("Saved the file".into()),
                    Err(err) => {
                        // the error will be probably too big
                        app.msgs.push(format!("Failed to save the file with err {}", err).into());
                        app.msgs.push(format!("{err}").into());
                    }
                }
            }

            if last_time_percentage_updated.elapsed() >= dur_between_percentage_updates {
                if let Some(pos) = app.player.get_position_sec() {
                    app.get_mut_current_chapter(&mut mediainfo).update_last_position(pos);
                } else {
                    app.msgs.push("Couldn't get the position".into());
                    continue;
                }
                last_time_percentage_updated = now;
                app.cache.pl_percentages = None;
            }
        }
    }

    app.player.null();
    Ok(mediainfo)
}

pub struct App<'a> {
    current_chapter_index: usize,  // index of the currently chosen chapter
    index_bookmark: Option<usize>, // index of the chosen bookmark
    index_all_bookmark: Option<usize>, // index of the chosen bookmark
    player: Player,                // things related to actually playing the playback
    msgs: Msgs,
    ui: Ui,
    cache: Cache<'a>,
    marked_position: Option<u64>, // position marked by the user with 'm' keybind

    /// position and chapter marked before making a jump form 'B' menu
    pos_and_chap_before_jump: Option<(u64, usize)>,
}

impl<'app> App<'app> {
    fn new(player: Player) -> Self {
        Self {
            player,
            current_chapter_index: 0, // index of the current chapter
            msgs: Msgs::default(),
            index_bookmark: None,
            cache: Cache::new(),
            ui: ui::Ui::new(),
            marked_position: None,
            index_all_bookmark: None,
            pos_and_chap_before_jump: None,
        }
    }

    fn load_chapter(&mut self, chapter_index: usize, mediainfo: &MediaInfo) {
        self.cache.invalide_all();

        self.player.null();
        self.player = Player::default();
        self.current_chapter_index = chapter_index;
        let current_chapter = self.get_current_chapter(mediainfo);
        self.player.load_chapter(
            current_chapter,
            &mediainfo.path,
            mediainfo.speed,
            mediainfo.volume,
        );
    }

    fn bookmark_select(
        &mut self,
        track: Option<usize>,
        bookmark_index: usize,
        mediainfo: &MediaInfo,
    ) {
        if let Some(track) = track {
            self.load_chapter(track, mediainfo);
            let current_chapter = self.get_current_chapter(mediainfo);
            let bookmark = current_chapter.bookmarks.get(bookmark_index).unwrap();
            self.player.seek_seconds(bookmark.position, mediainfo.speed).unwrap();
            if let Some(tracknumber) = current_chapter.tracknumber {
                self.msgs.push(
                    format!(
                        "Selected bookmark: {} from Chapter {}, track number: {}",
                        bookmark.formatted_position,
                        current_chapter.get_title_or_filename(),
                        tracknumber
                    )
                    .into(),
                );
            } else {
                self.msgs.push(
                    format!(
                        "Selected bookmark: {} from Chapter {}, track number: None",
                        bookmark.formatted_position,
                        current_chapter.get_title_or_filename(),
                    )
                    .into(),
                );
            }
        } else {
            let current_chapter = self.get_current_chapter(mediainfo);
            let bookmark = current_chapter.bookmarks.get(bookmark_index).unwrap();

            self.player.seek_seconds(bookmark.position, mediainfo.speed).unwrap();
            self.msgs.push(format!("Selected bookmark: {}", bookmark.formatted_position,).into());
        }
    }

    fn on_tick(&mut self, mediainfo: &mut MediaInfo) {
        let current_chapter = self.get_current_chapter(mediainfo);

        let (abs_position, position) = if let Some(abs_pos) = self.player.get_position_sec() {
            if let Some(sub) = abs_pos.checked_sub(current_chapter.start_position.unwrap_or(0)) {
                (abs_pos, sub)
            } else {
                (abs_pos, 0)
            }
        } else {
            (0, 0)
        };

        self.ui.on_tick(mediainfo.volume, position, current_chapter.length);
        self.cache.on_tick(current_chapter, position, abs_position);
        self.msgs.on_tick();

        // handle gstreamer messages
        if let Some(msg) = self.player.bus.pop_filtered(&[MessageType::Eos, MessageType::Error]) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(_) => {
                    self.get_mut_current_chapter(mediainfo).update_last_position(abs_position);

                    if self.current_chapter_index + 1 < mediainfo.chaptercount {
                        self.msgs.push("End of stream. Starting next chapter".into());

                        self.msgs.push(self.current_chapter_index.to_string().into());
                        self.load_chapter(self.current_chapter_index + 1, mediainfo);
                        self.player.play();
                    } else {
                        self.msgs.push("End of the book".into());
                        self.player.pause();
                    }
                }
                MessageView::Error(err) => {
                    self.msgs.push(
                        format!(
                            "Error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        )
                        .into(),
                    );
                }

                _ => (),
            }
        } else if self.player.is_playing() && position >= current_chapter.length {
            self.get_mut_current_chapter(mediainfo).update_last_position(abs_position);
            if self.current_chapter_index + 1 < mediainfo.chaptercount {
                self.msgs.push("End of the chapter. Starting next chapter".into());
                self.load_chapter(self.current_chapter_index + 1, mediainfo);
                self.player.play();
            } else {
                self.msgs.push("End of the book".into());
                self.player.pause();
            }
        }
    }

    fn get_current_chapter<'a, 'b>(&'a self, mediainfo: &'b MediaInfo) -> &'b Chapter {
        mediainfo.chapters.get(self.current_chapter_index).unwrap()
    }

    fn get_mut_current_chapter<'a, 'b>(&'a self, mediainfo: &'b mut MediaInfo) -> &'b mut Chapter {
        mediainfo.chapters.get_mut(self.current_chapter_index).unwrap()
    }
}

mod actions {
    use std::io;
    use std::ops::ControlFlow;
    use std::time::{Duration,
                    Instant};

    use tui::backend::Backend;
    use tui::Terminal;

    use crate::data::mediainfo::MediaInfo;
    use crate::{ui,
                App};

    /// toggles playback
    pub fn toggle_play(app: &mut App) {
        if app.player.is_paused() {
            app.player.play();
            app.msgs.push("Starting Playback".into());
        } else if app.player.is_playing() {
            app.player.pause();
            app.msgs.push("Stopping Playback".into());
        }
    }

    /// Moves playlist and playback to the next chapter
    ///
    /// * `should_update`: If `true` it will update last position of the chapter before moving to
    ///   the next
    pub fn next_chapter(
        app: &mut App,
        mediainfo: &mut MediaInfo,
        should_update: bool,
    ) -> ControlFlow<()> {
        if app.current_chapter_index + 1 >= mediainfo.chaptercount {
            app.msgs.push("You are the end of the playlist. Can't move any further.".into());
            return ControlFlow::Continue(());
        }
        if should_update {
            if let Some(pos) = app.player.get_position_sec() {
                app.get_mut_current_chapter(mediainfo).update_last_position(pos);
            } else {
                app.msgs.push("Couldn't get the position and update the position".into());
                return ControlFlow::Continue(());
            }
        }
        let was_playing = app.player.is_playing_and_pause();
        app.load_chapter(app.current_chapter_index + 1, &*mediainfo);
        if was_playing {
            app.player.play();
        }
        app.marked_position = None;
        app.msgs.push("Moved to the next chapter".into());
        ControlFlow::Continue(())
    }

    /// Moves playlist and playback to the previous chapter
    pub fn prev_chapter(app: &mut App, mediainfo: &mut MediaInfo) {
        if app.current_chapter_index < 1 {
            app.msgs.push("You are the start of the playlist. Can't move any backwards.".into());
            return;
        }
        if let Some(pos) = app.player.get_position_sec() {
            app.get_mut_current_chapter(mediainfo).update_last_position(pos);
        } else {
            app.msgs.push("Couldn't get the position".into());
            return;
        }
        let was_playing = app.player.is_playing_and_pause();
        app.load_chapter(app.current_chapter_index - 1, &*mediainfo);
        if was_playing {
            app.player.play();
        }
        app.marked_position = None;
        app.msgs.push("Moved to the previous chapter".into());
    }

    pub fn increase_volume(app: &mut App, mediainfo: &mut MediaInfo) {
        if mediainfo.volume + 0.05 > 1.0 {
            mediainfo.volume = 1.0;
            app.player.set_volume(1.0);
            app.msgs.push("Can't increase volume beyond 100%".into());
        } else {
            mediainfo.volume += 0.05;
            app.player.set_volume(mediainfo.volume);
            app.msgs.push("Increased volume by 5%".into());
        }
    }

    pub fn descrease_volume(app: &mut App, mediainfo: &mut MediaInfo) {
        if mediainfo.volume - 0.05 < 0.0 {
            mediainfo.volume = 0.0;
            app.player.set_volume(0.0);
            app.msgs.push("Can't descrease volume below 0%".into());
        } else {
            mediainfo.volume -= 0.05;
            app.player.set_volume(mediainfo.volume);
            app.msgs.push("Decreased volume by 5%".into());
        }
    }

    /// Saves the current position of the chapter and index of currently played chapter then breaks
    /// the loop
    pub fn quit(app: &mut App, mediainfo: &mut MediaInfo) -> ControlFlow<()> {
        if let Some(pos) = app.player.get_position_sec() {
            app.get_mut_current_chapter(mediainfo).update_last_position(pos);
        } else {
            app.msgs.push("Couldn't get the position".into());
            return ControlFlow::Continue(());
        }
        mediainfo.last_chapter = app.current_chapter_index;
        ControlFlow::Break(())
    }

    /// Add a description for the currently chosen item in the playlist
    pub fn add_description<B: Backend>(
        app: &mut App,
        mediainfo: &mut MediaInfo,
        terminal: &mut Terminal<B>,
        last_tick: &mut Instant,
        tick_rate: Duration,
    ) -> io::Result<()> {
        let was_playing = app.player.is_playing_and_pause();

        if let Some(description) = ui::popouts::input::run(
            terminal,
            app,
            mediainfo,
            last_tick,
            tick_rate,
            "Input a description for the chapter",
            app.get_current_chapter(mediainfo).description.as_ref(),
            35,
        )? {
            let current_chapter = app.get_mut_current_chapter(mediainfo);

            if description.is_empty() {
                current_chapter.description = None;
            } else {
                current_chapter.description = Some(description);
            }
        } else {
            app.msgs.push("Cancelled adding a description".into());
        };

        if was_playing {
            app.player.play();
        }
        app.cache.pl_titles = None;

        Ok(())
    }

    pub fn delete_description(app: &mut App, mediainfo: &mut MediaInfo) {
        let current_chapter = app.get_mut_current_chapter(mediainfo);
        current_chapter.description = None;
        app.cache.pl_titles = None;
    }

    pub fn add_bookmark<B: Backend>(
        app: &mut App,
        mediainfo: &mut MediaInfo,
        terminal: &mut Terminal<B>,
        last_tick: &mut Instant,
        tick_rate: Duration,
    ) -> std::io::Result<ControlFlow<()>> {
        let was_playing = app.player.is_playing_and_pause();
        let position = app.player.get_position_sec().unwrap();

        let name = if let Some(name) = ui::popouts::input::run(
            terminal,
            app,
            mediainfo,
            last_tick,
            tick_rate,
            "Input the name for the bookmark. Confirm with Enter. Cancel with Escape",
            None,
            100,
        )? {
            name
        } else {
            app.msgs.push("Cancelled adding a bookmark".into());
            return Ok(ControlFlow::Continue(()));
        };

        app.get_mut_current_chapter(mediainfo).add_bookmark(name, position);
        app.msgs.push("Added a bookmark".into());

        app.cache.invalidate_bks();
        if was_playing {
            app.player.play();
        }

        Ok(ControlFlow::Continue(()))
    }

    pub fn add_mark(app: &mut App) {
        if let Some(pos) = app.player.get_position_sec() {
            app.marked_position = Some(pos);
            app.msgs.push(format!("Marked position at {pos}").into());
        } else {
            app.msgs.push("Couldnt get the current position".into());
        }
    }

    pub fn add_bookmark_at_mark<B: Backend>(
        app: &mut App,
        mediainfo: &mut MediaInfo,
        terminal: &mut Terminal<B>,
        last_tick: &mut Instant,
        tick_rate: Duration,
    ) -> std::io::Result<()> {
        if let Some(pos) = app.marked_position {
            let was_playing = app.player.is_playing_and_pause();
            let name = if let Some(name) = ui::popouts::input::run(
                terminal,
                app,
                mediainfo,
                last_tick,
                tick_rate,
                "Input the name for the bookmark. Confirm with Enter. Cancel with Escape",
                None,
                75,
            )? {
                name
            } else {
                app.msgs.push("Cancelled adding a bookmark".into());
                return Ok(());
            };

            app.get_mut_current_chapter(mediainfo).add_bookmark(name, pos);
            app.msgs.push("Added a bookmark".into());
            app.cache.invalidate_bks();
            app.marked_position = None;

            if was_playing {
                app.player.play();
            }
        }

        Ok(())
    }

    pub fn move_to_arbitrary_position<B: Backend>(
        app: &mut App,
        mediainfo: &mut MediaInfo,
        terminal: &mut Terminal<B>,
        last_tick: &mut Instant,
        tick_rate: Duration,
    ) -> std::io::Result<()> {
        let was_playing = app.player.is_playing_and_pause();

        let input = if let Some(name) = ui::popouts::input::run(
            terminal,
            app,
            mediainfo,
            last_tick,
            tick_rate,
            "Input the position. Number followed by a 'h' - hours, 'm' - minutes, 's' - seconds",
            None,
            82,
        )? {
            name
        } else {
            app.msgs.push("Canceled".into());
            return Ok(());
        };

        let secs = if let Some(secs) = crate::helpers::try_into_seconds(&input) {
            secs
        } else {
            app.msgs.push(
                "Detected an illegal character. 'h'/'m'/'s' and numbers are the only legal".into(),
            );
            return Ok(());
        };

        if secs > app.get_current_chapter(mediainfo).length {
            app.msgs.push("Given position is bigger than the length of the chapter".into());
            return Ok(());
        }

        if let Some(pos) = app.player.get_position_sec() {
            app.get_mut_current_chapter(mediainfo).before_jump_position = Some(pos);
        }

        app.player
            .seek_seconds(
                app.get_current_chapter(mediainfo).get_start_position() + secs,
                mediainfo.speed,
            )
            .unwrap();

        app.msgs.push(format!("Moved to {}", input).into());

        if was_playing {
            app.player.play();
        }

        Ok(())
    }

    pub fn move_forward(app: &mut App, mediainfo: &mut MediaInfo) {
        let abs_pos = if let Some(pos) = app.player.get_position_sec() {
            pos
        } else {
            app.msgs.push("Couldn't get the position".into());
            return;
        };
        let current_chapter = app.get_current_chapter(mediainfo);
        let start_pos = current_chapter.start_position.unwrap_or(0);
        let cur_pos = abs_pos.saturating_sub(start_pos);
        match (cur_pos + 5).cmp(&current_chapter.length) {
            std::cmp::Ordering::Equal | std::cmp::Ordering::Less => {
                app.player.seek_seconds(abs_pos + 5, mediainfo.speed).unwrap();
                app.msgs.push("Move forwards by 5 seconds".into());
            }
            std::cmp::Ordering::Greater => {
                app.player
                    .seek_seconds(start_pos + current_chapter.length, mediainfo.speed)
                    .unwrap();
                app.msgs.push("Moved to the end".into());
            }
        }
    }

    pub fn move_backward(app: &mut App, mediainfo: &mut MediaInfo) {
        let abs_pos = if let Some(pos) = app.player.get_position_sec() {
            pos
        } else {
            app.msgs.push("Couldn't get the position".into());
            return;
        };
        let current_chapter = app.get_current_chapter(mediainfo);
        let start_pos = current_chapter.get_start_position();
        if let Some(sub) = abs_pos.checked_sub(5) {
            match sub.cmp(&start_pos) {
                std::cmp::Ordering::Greater => {
                    app.player.seek_seconds(sub, mediainfo.speed).unwrap();
                    app.msgs.push("Move backwards by 5 seconds".into());
                }
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                    app.player.seek_seconds(start_pos, mediainfo.speed).unwrap();
                    app.msgs.push("Moved to the start".into());
                }
            }
        } else {
            app.player.seek_seconds(start_pos, mediainfo.speed).unwrap();
            app.msgs.push("Moved to the start".into());
        }
    }

    pub fn descrease_speed(app: &mut App, mediainfo: &mut MediaInfo) {
        let speed = ((mediainfo.speed - 0.25) * 100.0).round() / 100.0;
        if speed <= 0.0 {
            app.msgs.push("Can't descrease the speed any further".into());
            return;
        }

        if app.player.set_speed(speed).is_ok() {
            mediainfo.speed = speed;
        } else {
            app.msgs.push("Couldn't descrease the speed".into());
        }
    }

    pub fn increase_speed(app: &mut App, mediainfo: &mut MediaInfo) {
        let speed = ((mediainfo.speed + 0.25) * 100.0).round() / 100.0;
        if app.player.set_speed(speed).is_ok() {
            mediainfo.speed = speed;
        } else {
            app.msgs.push("Couldn't increase the speed".into());
        }
    }

    pub fn restore_pos_before_jump(app: &mut App, mediainfo: &mut MediaInfo) {
        app.player.if_playing_pause();
        if let Some(pos) = app.get_current_chapter(mediainfo).before_jump_position {
            if app.player.seek_seconds(pos, mediainfo.speed).is_err() {
                app.msgs.push("Couldn't restore the position".into());
            } else {
                app.msgs.push("Retored the position before a jump".into());
            }
        } else {
            app.msgs.push("There isn't a jump saved for this chapter.".into());
        }
    }

    pub fn restore_pos_and_chap_before_jump(app: &mut App, mediainfo: &mut MediaInfo) {
        app.player.if_playing_pause();
        if let Some((pos, chapter)) = app.pos_and_chap_before_jump {
            if chapter == app.current_chapter_index {
                if app.player.seek_seconds(pos, mediainfo.speed).is_err() {
                    app.msgs.push("Couldn't restore the position".into());
                } else {
                    app.msgs.push("0".into());
                }
            } else {
                app.load_chapter(chapter, mediainfo);
                if app.player.seek_seconds(pos, mediainfo.speed).is_err() {
                    app.msgs.push("Couldn't restore the position".into());
                } else {
                    app.msgs.push("1".into());
                }
            }
        } else {
            app.msgs.push("There is no saved position before the jump".into());
        }
    }
}
