use std::time::{Duration,
                Instant};

use crossterm::event::{self,
                       Event,
                       KeyCode};
use tui::backend::Backend;
use tui::layout::Alignment;
use tui::style::{Color,
                 Modifier,
                 Style};
use tui::widgets::{Block,
                   Borders,
                   Clear,
                   List,
                   ListItem,
                   ListState};
use tui::Terminal;

use super::super::centered_rec_perc;
use crate::data::mediainfo::MediaInfo;
use crate::App;

fn render<B: Backend>(
    f: &mut tui::Frame<B>,
    app: &mut App,
    mediainfo: &MediaInfo,
    items: &Vec<ListItem>,
    index: usize,
) {
    super::super::render(f, app, mediainfo);

    let popout = centered_rec_perc(75, 75, f.size());
    let block = Block::default()
        .title("Choose a bookmark")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    f.render_widget(Clear, popout);
    f.render_widget(block, popout);

    let list = List::new(&**items)
        .block(Block::default().title("List").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC).fg(Color::Green))
        .highlight_symbol(">>");
    let mut state = ListState::default();
    state.select(Some(index));
    f.render_stateful_widget(list, popout, &mut state);
}

pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: &mut MediaInfo,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> std::io::Result<()> {
    let len = mediainfo.chapters.iter().fold(0, |acc, it| it.bookmarks.len() + acc);

    if len == 0 {
        app.msgs.push("None of the chapter have any bookmarks".into());
        return Ok(());
    }

    let was_playing = app.player.is_playing_and_pause();

    let mut i = if let Some(index) = app.index_all_bookmark {
        if index >= len { len.saturating_sub(1) } else { index }
    } else {
        0
    };

    let mut indexes = Vec::new();

    app.msgs.push(
        "Press Enter to chose a bookmark. Press jk to move up and down. Press Escape to cancel."
            .into(),
    ); // this message will not disappear
    app.msgs.on_tick();

    let items: Vec<ListItem> = mediainfo
        .chapters
        .iter()
        .enumerate()
        .filter_map(|(chapter_index, chapter)| {
            if chapter.bookmarks.is_empty() {
                None
            } else {
                Some(
                    chapter
                        .bookmarks
                        .iter()
                        .enumerate()
                        .map(|(bk_index, bk)| {
                            indexes.push((chapter_index, bk_index));
                            ListItem::new(format!(
                                "{} | chapter name: {} | chapter number: {}",
                                &*bk.formatted_position,
                                chapter.get_title_or_filename(),
                                chapter.get_track_number()
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
            }
        })
        .flatten()
        .collect();

    assert!(indexes.len() == items.len());

    let index: Option<(Option<usize>, usize)> = loop {
        terminal.draw(|f| render(f, app, mediainfo, &items, i))?;
        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.msgs.push("Canceled choosing a bookmark".into());
                        break None;
                    }

                    KeyCode::Char('k') | KeyCode::Up => {
                        i = i.saturating_sub(1);
                    }

                    KeyCode::Char('j') | KeyCode::Down => {
                        i = std::cmp::min(i.saturating_add(1), len - 1);
                    }

                    KeyCode::Enter => {
                        let (chapter_index, bk_index) = indexes[i];
                        break Some((Some(chapter_index), bk_index));
                    }

                    _ => continue,
                }
            }
        }
    };

    app.index_all_bookmark = Some(i);

    if let Some((Some(chapter_index), bk_index)) = index {
        let curent_pos = app.player.get_position_sec().unwrap();
        if chapter_index == app.current_chapter_index {
            app.pos_and_chap_before_jump = Some((curent_pos, chapter_index));
            app.get_mut_current_chapter(mediainfo).before_jump_position = Some(curent_pos);
        } else {
            app.pos_and_chap_before_jump = Some((curent_pos, app.current_chapter_index));
        }
        app.bookmark_select(Some(chapter_index), bk_index, mediainfo);
    }

    if was_playing {
        app.player.play();
    }

    Ok(())
}
