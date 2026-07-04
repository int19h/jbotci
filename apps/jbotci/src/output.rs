use super::*;

use owo_colors::OwoColorize;
use std::io::IsTerminal;

#[requires(true)]
#[ensures(true)]
pub(super) fn dark(text: &str, color: bool) -> String {
    if color {
        text.bright_black().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn yellow(text: &str, color: bool) -> String {
    if color {
        text.yellow().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn yellow_underlined(text: &str, color: bool) -> String {
    if color {
        text.yellow().underline().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn blue(text: &str, color: bool) -> String {
    if color {
        text.bright_blue().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn magenta(text: &str, color: bool) -> String {
    if color {
        text.magenta().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn green(text: &str, color: bool) -> String {
    if color {
        text.green().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn red(text: &str, color: bool) -> String {
    if color {
        text.red().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cyan(text: &str, color: bool) -> String {
    if color {
        text.cyan().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn light(text: &str, color: bool) -> String {
    if color {
        text.white().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn stream_supports_ansi_color(stream: concolor::Stream) -> bool {
    concolor::get(stream).ansi_color()
}

#[requires(true)]
#[ensures(ret.is_none_or(|width| width > 0))]
pub(super) fn stdout_terminal_width() -> Option<usize> {
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return None;
    }
    terminal_size::terminal_size_of(stdout)
        .map(|(terminal_size::Width(width), _height)| usize::from(width))
        .filter(|width| *width > 0)
}

#[requires(true)]
#[ensures(ret > 0)]
pub(super) fn stderr_terminal_width() -> usize {
    terminal_size::terminal_size_of(std::io::stderr())
        .map(|(terminal_size::Width(width), _height)| usize::from(width))
        .filter(|width| *width > 0)
        .unwrap_or(DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH)
}
