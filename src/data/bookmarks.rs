use serde::{Deserialize,
            Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Bookmark {
    /// position inside the Chapter in seconds
    pub position: u64,
    /// user given name for the bookmark
    pub name: String,
    /// user given name for the bookmark
    pub formatted_position: String,
}

impl Bookmark {
    pub fn new(position: u64, start_position: Option<u64>, name: String) -> Self {
        let formatted_position = format_position(&name, position, start_position);
        Self { position, name, formatted_position }
    }

    pub fn change_name(&mut self, new_name: String) {
        let name = self.formatted_position.split_once(' ').unwrap();
        self.formatted_position = format!("{} {}", new_name, name.1);
        self.name = new_name;
    }
}

impl core::fmt::Display for Bookmark {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#?} at {:#?}\r", self.name, self.position)
    }
}

fn format_position(name: &String, position: u64, start_position: Option<u64>) -> String {
    if let Some(start_position) = start_position {
        let off_position = position - start_position;

        let abs_minutes = position / 60;
        let off_minutes = off_position / 60;

        if abs_minutes == 0 {
            return format!("\"{}\" at {}s({}s)", name, off_position, position);
        }

        let abs_hours = abs_minutes / 60;
        let off_hours = off_minutes / 60;

        if abs_hours == 0 {
            let abs_seconds = position - abs_minutes * 60;
            let off_seconds = off_position - off_minutes * 60;
            return format!(
                "\"{}\" at {}m{}s({}m{}s)",
                name, off_minutes, off_seconds, abs_minutes, abs_seconds
            );
        }

        let abs_minutes = abs_minutes - abs_hours * 60;
        let abs_seconds = position - abs_hours * 3600 - abs_minutes * 60;

        let off_minutes = off_minutes - off_hours * 60;
        let off_seconds = off_position - off_hours * 3600 - off_minutes * 60;

        match off_hours {
            0 => match off_minutes {
                0 => format!(
                    "\"{}\" at {}s({}h{}m{}s)",
                    name, off_seconds, abs_hours, abs_minutes, abs_seconds
                ),
                _ => format!(
                    "\"{}\" at {}m{}s({}h{}m{}s)",
                    name, off_minutes, off_seconds, abs_hours, abs_minutes, abs_seconds
                ),
            },
            _ => format!(
                "\"{}\" at {}h{}m{}s({}h{}m{}s)",
                name, off_hours, off_minutes, off_seconds, abs_hours, abs_minutes, abs_seconds
            ),
        }
    } else {
        let minutes = position / 60;

        if minutes == 0 {
            return format!("\"{}\" at {}s", name, position);
        }

        let hours = minutes / 60;

        if hours == 0 {
            let seconds = position - minutes * 60;
            return format!("\"{}\" at {}m{}s", name, minutes, seconds);
        }

        let minutes = minutes - hours * 60;
        let seconds = position - hours * 3600 - minutes * 60;

        format!("\"{}\" at {}h{}m{}s", name, hours, minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_display0() {
        let data = Bookmark::new(1, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 1s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display1() {
        let data = Bookmark::new(61, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 1m1s";

        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display2() {
        let data = Bookmark::new(3600, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 1h0m0s";

        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display3() {
        let data = Bookmark::new(3661, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 1h1m1s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display4() {
        let data = Bookmark::new(8217, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 2h16m57s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display5() {
        let data = Bookmark::new(8217, Some(0), "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 2h16m57s(2h16m57s)";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display6() {
        let data = Bookmark::new(0, None, "bk".to_string());
        let actual = data.formatted_position;
        let expected = "\"bk\" at 0s";
        assert_eq!(actual, expected);
    }
}
