//! Shared visual cues; the words still carry meaning without colour.
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};

pub(crate) const MINT: Color = Color::Rgb(128, 225, 178);
pub(crate) const MUTED: Color = Color::Rgb(159, 181, 176);
pub(crate) const AMBER: Color = Color::Rgb(242, 186, 113);
pub(crate) const SELECTED: Color = Color::Rgb(32, 65, 54);
pub(crate) fn base() -> Style {
    Style::default()
        .fg(Color::Rgb(226, 234, 230))
        .bg(Color::Rgb(12, 17, 17))
}
pub(crate) fn report_line(text: String) -> Line<'static> {
    let style = if text.starts_with("> ") {
        Style::default()
            .fg(MINT)
            .bg(SELECTED)
            .add_modifier(Modifier::BOLD)
    } else if text.contains("Different status") || text.contains("Stored values differ") {
        Style::default().fg(AMBER)
    } else if !text.is_empty()
        && text.chars().any(char::is_alphabetic)
        && text
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
    {
        Style::default().fg(MINT).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::styled(text, style)
}

// This is activity, never a completion percentage or a network stage.
pub(crate) fn activity(elapsed: std::time::Duration, reduced: bool) -> &'static str {
    if reduced {
        "●"
    } else {
        ["◐", "◓", "◑", "◒"][(elapsed.as_millis() / 180 % 4) as usize]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn motion_is_explicit_repeatable_and_can_be_stopped() {
        use std::time::Duration;
        assert_eq!(super::activity(Duration::ZERO, false), "◐");
        assert_eq!(super::activity(Duration::from_millis(180), false), "◓");
        assert_eq!(super::activity(Duration::from_millis(720), false), "◐");
        for ms in [0, 180, 720, u64::MAX] {
            assert_eq!(super::activity(Duration::from_millis(ms), true), "●");
        }
    }
}
