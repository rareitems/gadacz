use std::io;
use std::time::{Duration,
                Instant};

use crossterm::event::{self,
                       Event,
                       KeyCode,
                       KeyModifiers};
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

fn render<B: Backend>(
    f: &mut tui::Frame<B>,
    app: &mut App,
    mediainfo: &MediaInfo,
    prompt: &'static str,
    input: &String,
    width: u16,
) {
    super::super::render(f, app, mediainfo);
    let block = Block::default()
        .title(prompt)
        .title_alignment(Alignment::Center)
        .borders(tui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let paragraph = Paragraph::new(input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black));

    if let Some(area) = centered_rect_flat(width, 3, f.size()) {
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(paragraph, area);
        f.set_cursor(input.len() as u16 + area.x + 1, area.y + 1);
    };
}

#[allow(clippy::too_many_arguments)]
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mediainfo: &MediaInfo,
    last_tick: &mut Instant,
    tick_rate: Duration,
    prompt: &'static str,
    input: Option<&String>,
    width: u16,
) -> io::Result<Option<String>> {
    let mut input = if let Some(x) = input { x.clone() } else { String::new() };

    // This will show the pushed message for the whole duration for the following loop
    app.msgs.push("Press Escape to cancel".into());
    app.msgs.on_tick();

    loop {
        terminal.draw(|f| render(f, app, mediainfo, prompt, &input, width))?;
        let timeout =
            tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        break Ok(Some(input.drain(..).collect::<String>()));
                    }
                    KeyCode::Char(c) => {
                        if key.modifiers == KeyModifiers::CONTROL && c == 'w' {
                            while let Some(pop) = input.pop() {
                                if pop == ' ' {
                                    break;
                                }
                            }
                        } else {
                            input.push(c);
                        }
                        continue;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        continue;
                    }
                    KeyCode::Esc => {
                        break Ok(None);
                    }

                    _ => continue,
                };
            }
        }
    }
}
