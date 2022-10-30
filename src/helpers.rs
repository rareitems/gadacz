/// Tries to turn a string format `"xhymzs"` where `x`, `y`, `z` are ammount of hours, minutes or
/// into seconds
pub fn try_into_seconds(input: &str) -> Option<u64> {
    let mut total = 0;
    let mut number = String::new();
    for it in input.chars() {
        if it.is_numeric() {
            number.push(it);
            continue;
        }

        match it {
            'h' => {
                total += number.parse::<u64>().unwrap() * 3600;
                number = String::new();
            }
            'm' => {
                total += number.parse::<u64>().unwrap() * 60;
                number = String::new();
            }
            's' => {
                total += number.parse::<u64>().unwrap();
                number = String::new();
            }
            _ => return None,
        }
    }

    match total {
        0 => None,
        _ => Some(total),
    }
}

/// Produces a string in format `"xhymzs"` where `x`, `y`, `z` are ammount of hours, minutes or
/// seconds respectively according to given `position` and `start_position`
pub fn format_position(position: u64, start_position: Option<u64>) -> String {
    if let Some(start_position) = start_position {
        let off_position = position - start_position;

        let abs_minutes = position / 60;
        let off_minutes = off_position / 60;

        if abs_minutes == 0 {
            return format!("{}s({}s)", off_position, position);
        }

        let abs_hours = abs_minutes / 60;
        let off_hours = off_minutes / 60;

        if abs_hours == 0 {
            let abs_seconds = position - abs_minutes * 60;
            let off_seconds = off_position - off_minutes * 60;
            return format!("{}m{}s({}m{}s)", off_minutes, off_seconds, abs_minutes, abs_seconds);
        }

        let abs_minutes = abs_minutes - abs_hours * 60;
        let abs_seconds = position - abs_hours * 3600 - abs_minutes * 60;

        let off_minutes = off_minutes - off_hours * 60;
        let off_seconds = off_position - off_hours * 3600 - off_minutes * 60;

        match off_hours {
            0 => match off_minutes {
                0 => format!("{}s({}h{}m{}s)", off_seconds, abs_hours, abs_minutes, abs_seconds),
                _ => format!(
                    "{}m{}s({}h{}m{}s)",
                    off_minutes, off_seconds, abs_hours, abs_minutes, abs_seconds
                ),
            },
            _ => format!(
                "{}h{}m{}s({}h{}m{}s)",
                off_hours, off_minutes, off_seconds, abs_hours, abs_minutes, abs_seconds
            ),
        }
    } else {
        let minutes = position / 60;

        if minutes == 0 {
            return format!("{}s", position);
        }

        let hours = minutes / 60;

        if hours == 0 {
            let seconds = position - minutes * 60;
            return format!("{}m{}s", minutes, seconds);
        }

        let minutes = minutes - hours * 60;
        let seconds = position - hours * 3600 - minutes * 60;

        format!("{}h{}m{}s", hours, minutes, seconds)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_into_seconds() {
        let data = "11h11m11s".to_string();
        let actual = try_into_seconds(&data);
        let expected = Some(11 * 3600 + 11 * 60 + 11);
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display0() {
        let data = 1;
        let actual = format_position(data, None);
        let expected = "1s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display1() {
        let data = 61;
        let actual = format_position(data, None);
        let expected = "1m1s";

        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display2() {
        let data = 3600;
        let actual = format_position(data, None);
        let expected = "1h0m0s";

        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display3() {
        let data = 3661;
        let actual = format_position(data, None);
        let expected = "1h1m1s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display4() {
        let data = 8217;
        let actual = format_position(data, None);
        let expected = "2h16m57s";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display5() {
        let data = 8217;
        let actual = format_position(data, Some(0));
        let expected = "2h16m57s(2h16m57s)";
        assert_eq!(actual, expected);
    }

    #[test]
    fn formatted_display6() {
        let data = 0;
        let actual = format_position(data, None);
        let expected = "0s";
        assert_eq!(actual, expected);
    }
}
