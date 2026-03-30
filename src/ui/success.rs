use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::{get_orange_accent, get_orange_color};

pub struct SuccessView<'a> {
    pub logs: &'a [String],
    pub phase: &'a str,
    pub keycloak_url: &'a str,
    pub portal_url: &'a str,
}

pub fn render_success(frame: &mut Frame, view: &SuccessView<'_>) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    let title_text = if view.phase == "Keycloak" {
        "✅  Keycloak Installed!"
    } else {
        "✅  Portal Installed!"
    };

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(get_orange_accent())),
        )
        .centered();
    frame.render_widget(title, chunks[0]);

    let message = if view.phase == "Keycloak" {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Keycloak is running! Next steps:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Keycloak Admin: "),
                Span::styled(
                    view.keycloak_url,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  1. Log in to Keycloak Admin",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "  2. Create realm and client (nqrust-portal)",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "  3. Copy the Client Secret from Credentials tab",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "  4. Press I to install the Portal",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Identity Portal is fully running!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Portal:   "),
                Span::styled(
                    view.portal_url,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Keycloak: "),
                Span::styled(
                    view.keycloak_url,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]),
            Line::from(""),
        ]
    };

    let message_widget = Paragraph::new(message).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(get_orange_accent()))
            .title(if view.phase == "Keycloak" {
                "Next Steps"
            } else {
                "Access URLs"
            })
            .title_style(
                Style::default()
                    .fg(get_orange_color())
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(message_widget, chunks[1]);

    let log_lines: Vec<Line> = view
        .logs
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|log| Line::from(Span::styled(log.clone(), Style::default().fg(Color::White))))
        .collect();

    let logs_widget = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(get_orange_accent()))
            .title("Installation Summary")
            .title_style(
                Style::default()
                    .fg(get_orange_color())
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(logs_widget, chunks[2]);

    let help_text = if view.phase == "Keycloak" {
        "I Install Portal   Q Quit   Ctrl+C Quit"
    } else {
        "Press Ctrl+C to exit"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    frame.render_widget(help, chunks[3]);
}
