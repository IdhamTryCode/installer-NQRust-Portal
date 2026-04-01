use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::app::state::HomeMenuItem;
use crate::ui::{ASCII_HEADER, get_orange_accent, get_orange_color};

pub fn render_home(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    // ASCII header
    let header = Paragraph::new(ASCII_HEADER)
        .style(Style::default().fg(get_orange_color()))
        .centered();
    frame.render_widget(header, chunks[0]);

    // Status panel
    let identity_indicator = if app.identity_running { "●" } else { "○" };
    let identity_status = if app.identity_running {
        "Running"
    } else {
        "Not installed"
    };
    let identity_status_color = if app.identity_running {
        Color::Green
    } else {
        Color::DarkGray
    };

    let portal_indicator = if app.portal_running { "●" } else { "○" };
    let portal_status = if app.portal_running {
        "Running"
    } else {
        "Not installed"
    };
    let portal_color = if app.portal_running {
        Color::Green
    } else {
        Color::DarkGray
    };

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                identity_indicator,
                Style::default()
                    .fg(identity_status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Identity   ", Style::default().fg(Color::White)),
            Span::styled(
                identity_status,
                Style::default()
                    .fg(identity_status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                portal_indicator,
                Style::default()
                    .fg(portal_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Portal     ", Style::default().fg(Color::White)),
            Span::styled(
                portal_status,
                Style::default()
                    .fg(portal_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let status_widget = Paragraph::new(status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(get_orange_accent()))
            .title("Service Status")
            .title_style(
                Style::default()
                    .fg(get_orange_color())
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(status_widget, chunks[1]);

    // Menu
    let menu_items = app.get_menu_items();
    let menu_lines: Vec<Line> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let label = match item {
                HomeMenuItem::InstallIdentity => "  Install Identity",
                HomeMenuItem::InstallPortal => "  Install Portal",
                HomeMenuItem::UpdateToken => "  Update GHCR Token",
                HomeMenuItem::Exit => "  Exit",
            };
            if i == app.menu_selection {
                Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Black)
                        .bg(get_orange_color())
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(label, Style::default().fg(Color::White)))
            }
        })
        .collect();

    let menu_widget = Paragraph::new(menu_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(get_orange_accent()))
            .title("Actions")
            .title_style(
                Style::default()
                    .fg(get_orange_color())
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(menu_widget, chunks[2]);

    // Help
    let help = Paragraph::new("↑↓ Navigate   Enter Select   Ctrl+C Quit")
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    frame.render_widget(help, chunks[3]);
}
