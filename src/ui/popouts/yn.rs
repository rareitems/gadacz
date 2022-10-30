use std::time::{Duration,
                Instant};

use crossterm::event::{self,
                       Event,
                       KeyCode};
use tui::backend::Backend;
use tui::layout::Alignment;
use tui::style::{Color,
                 Style};
use tui::widgets::{Block,
                   Clear,
                   Paragraph};
use tui::Terminal;

use crate::data::mediainfo::MediaInfo;
use crate::ui::centered_rect_flat;
use crate::App;

fn render<'a, B: Backend>(
    f: &mut tui::Frame<B>,
    app: &mut App,
    mediainfo: &'a MediaInfo,
    prompt: &'static str,
) {
    super::super::render(f, app, mediainfo);

    let block = Block::default()
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(prompt)
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Center);

    if let Some(area) = centered_rect_flat(prompt.len() as u16 + 2, 3, f.size()) {
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    } else {
        app.msgs.push("Couldn't create a popout".into());
    }
}

pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: &MediaInfo,
    last_tick: &mut Instant,
    tick_rate: Duration,
    prompt: &'static str,
) -> std::io::Result<bool> {
    loop {
        terminal.draw(|f| render(f, app, mediainfo, prompt))?;
        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') => {
                        break Ok(true);
                    }
                    KeyCode::Char('n' | 'q') | KeyCode::Esc => {
                        break Ok(false);
                    }
                    _ => continue,
                }
            }
        }
    }
}
