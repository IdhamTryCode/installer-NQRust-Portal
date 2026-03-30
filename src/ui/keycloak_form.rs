use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::app::form_data::{FocusState, KC_ADMIN_PASSWORD, KC_ADMIN_USERNAME, KC_DB_PASSWORD, KC_DB_USER};
use crate::ui::{get_orange_accent, get_orange_color};

pub fn render_keycloak_form(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("⚙  Install Keycloak — Phase 1 of 2")
        .style(Style::default().fg(get_orange_color()).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(get_orange_accent())))
        .centered();
    frame.render_widget(title, chunks[0]);

    // Form fields
    let total = app.keycloak_form.get_total_fields();
    let mut form_lines = vec![Line::from("")];

    for i in 0..total {
        let label = app.keycloak_form.get_field_label(i);
        let value = app.keycloak_form.get_field_value(i);
        let is_focused = matches!(&app.keycloak_form.focus_state, FocusState::Field(f) if *f == i);
        let is_editing = is_focused && app.keycloak_form.editing;

        let display = if value.is_empty() && !is_editing {
            "<enter value>".to_string()
        } else {
            value.to_string()
        };

        let field_style = if is_focused {
            Style::default().fg(Color::Black).bg(get_orange_color()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cursor = if is_focused { "▶" } else { " " };

        form_lines.push(Line::from(vec![
            Span::styled(format!(" {} ", cursor), field_style),
            Span::styled(format!("{:<20}", label), field_style),
            Span::styled(display, field_style),
        ]));
        form_lines.push(Line::from(""));
    }

    if !app.keycloak_form.error_message.is_empty() {
        form_lines.push(Line::from(Span::styled(
            format!("  ⚠  {}", app.keycloak_form.error_message),
            Style::default().fg(Color::Red),
        )));
    }

    let form_widget = Paragraph::new(form_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(get_orange_accent()))
            .title("Configuration")
            .title_style(Style::default().fg(get_orange_color()).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(form_widget, chunks[1]);

    // Hardcoded defaults info
    let info_lines = vec![
        Line::from(Span::styled("  Hardcoded defaults (not editable):", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("  Admin: {} / {}   DB user: {} / {}", KC_ADMIN_USERNAME, KC_ADMIN_PASSWORD, KC_DB_USER, KC_DB_PASSWORD),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];
    let info_widget = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title("Defaults")
            .title_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(info_widget, chunks[2]);

    // Buttons
    let save_focused = matches!(&app.keycloak_form.focus_state, FocusState::SaveButton);
    let cancel_focused = matches!(&app.keycloak_form.focus_state, FocusState::CancelButton);

    let save_style = if save_focused {
        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    };
    let cancel_style = if cancel_focused {
        Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let button_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(" Install Keycloak ", save_style),
        Span::raw("   "),
        Span::styled(" Cancel ", cancel_style),
    ]);
    let buttons = Paragraph::new(button_line);
    frame.render_widget(buttons, chunks[3]);

    // Help
    let help = Paragraph::new("Tab/↑↓ Navigate   Enter Edit   Ctrl+S Save   Esc Back")
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    frame.render_widget(help, chunks[4]);
}
