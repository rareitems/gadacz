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
use crate::{ui,
            App};

fn render<'a, B: Backend>(
    f: &mut tui::Frame<B>,
    app: &mut App,
    mediainfo: &'a MediaInfo,
    items: &Vec<ListItem>,
    index: usize,
) {
    super::super::render(f, app, mediainfo);
    let popout = centered_rec_perc(75, 75, f.size());
    let block = Block::default()
        .title("Choose a bookmark")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    f.render_widget(Clear, popout); // this clears out the background
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

/// TEST
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: &mut MediaInfo,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> std::io::Result<()> {
    if app.get_current_chapter(mediainfo).bookmarks.is_empty() {
        app.msgs.push("This chapter doesn't have any bookmarks".into());
        return Ok(());
    }

    let was_playing = app.player.is_playing_and_pause();

    if let Some(pos) = app.player.get_position_sec() {
        app.get_mut_current_chapter(mediainfo).before_jump_position = Some(pos);
    }

    let current_chapter = app.get_current_chapter(mediainfo);

    let items: Vec<_> =
        current_chapter.bookmarks.iter().map(|x| ListItem::new(&*x.formatted_position)).collect();

    let len = current_chapter.bookmarks.len();

    let mut i = if let Some(index) = app.index_bookmark {
        if index >= len { len.saturating_sub(1) } else { index }
    } else {
        0
    };

    app.msgs.push(
        "Press Enter to chose a bookmark. Press j and k to move up and down. Press d to delete
a bookmark. Press e to change a name of a bookmark. Press Escape to cancel."
            .into(),
    ); // this message will not disappear
    app.msgs.on_tick();

    loop {
        terminal.draw(|f| render(f, app, mediainfo, &items, i))?;

        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.msgs.push("Canceled choosing a bookmark".into());
                        break;
                    }

                    KeyCode::Char('k') | KeyCode::Up => {
                        i = i.saturating_sub(1);
                    }

                    KeyCode::Char('j') | KeyCode::Down => {
                        i = std::cmp::min(i.saturating_add(1), len - 1);
                    }

                    KeyCode::Enter => {
                        app.bookmark_select(None, i, mediainfo);
                        break;
                    }

                    KeyCode::Char('e') => {
                        let res = ui::popouts::input::run(
                            terminal,
                            app,
                            mediainfo,
                            last_tick,
                            tick_rate,
                            "Change the name of the bookmark",
                            Some(&current_chapter.bookmarks[i].name),
                            100,
                        )?;

                        if let Some(new_name) = res {
                            app.msgs.push(
                                format!(
                                    "Changed name from {} to {}",
                                    app.get_current_chapter(mediainfo).bookmarks[i].name,
                                    new_name
                                )
                                .into(),
                            );

                            app.get_mut_current_chapter(mediainfo).bookmarks[i]
                                .change_name(new_name);
                        }

                        app.cache.invalidate_bks();
                        break;
                    }

                    KeyCode::Char('d') => {
                        let res = ui::popouts::yn::run(
                            terminal,
                            app,
                            mediainfo,
                            last_tick,
                            tick_rate,
                            "Are you sure you want to delete the bookmark? y/n",
                        )?;
                        if res {
                            let delete = app.get_mut_current_chapter(mediainfo).delete_bookmark(i);
                            app.msgs.push(
                                format!("Deleted bookmark: {}", delete.formatted_position).into(),
                            );

                            app.cache.invalidate_bks();
                            app.cache.pl_bks_count = None;
                            break;
                        }
                    }

                    _ => continue,
                }
            }
        }
    }

    if was_playing {
        app.player.play();
    }

    app.index_bookmark = Some(i);

    Ok(())
}
