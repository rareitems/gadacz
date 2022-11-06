use std::io;
use std::time::{Duration,
                Instant};

use crossterm::event::{self,
                       Event,
                       KeyCode};
use tui::backend::Backend;
use tui::layout::{Alignment,
                  Constraint,
                  Direction,
                  Layout};
use tui::style::{Color,
                 Style};
use tui::widgets::{Block,
                   Clear,
                   List,
                   ListItem};
use tui::Terminal;

use super::super::centered_rec_perc;
use crate::data::mediainfo::MediaInfo;
use crate::App;

pub fn run<'a, B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: &'a MediaInfo,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> io::Result<()> {
    app.player.pause();

    app.msgs.push("Press Escape to cancel".into());
    app.msgs.on_tick();

    loop {
        terminal.draw(|f| render(f, app, mediainfo))?;
        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }

                    _ => continue,
                }
            }
        }
    }

    Ok(())
}

fn render<B: Backend>(f: &mut tui::Frame<B>, app: &mut App, mediainfo: &MediaInfo) {
    super::super::render(f, app, mediainfo);
    let area = centered_rec_perc(75, 75, f.size()); // could be done better with rect function that
    // takes integer contraist instead of percentages

    let block = Block::default()
        .title("Help Menu")
        .title_alignment(Alignment::Center)
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(block, area);

    let count = app.ui.keybindings_list.len() / 2;
    let list0: Vec<_> =
        app.ui.keybindings_list.iter().take(count).map(|it| ListItem::new(*it)).collect();
    let list1: Vec<_> =
        app.ui.keybindings_list.iter().skip(count).map(|it| ListItem::new(*it)).collect();
    f.render_widget(List::new(list0).style(Style::default().fg(Color::White)), chunks[0]);
    f.render_widget(List::new(list1).style(Style::default().fg(Color::White)), chunks[1]);
}
