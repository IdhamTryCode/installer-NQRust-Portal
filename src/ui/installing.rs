use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::ui::{get_orange_accent, get_orange_color};

pub struct InstallingView<'a> {
    pub progress: f64,
    pub current_service: &'a str,
    pub completed_services: usize,
    pub total_services: usize,
    pub logs: &'a [String],
    pub phase: &'a str,
}

pub fn render_installing(frame: &mut Frame, view: &InstallingView<'_>) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    let title_text = format!("🔄  Installing {}... Please wait", view.phase);
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(get_orange_color())
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(get_orange_accent())),
        )
        .centered();
    frame.render_widget(title, chunks[0]);

    let bar_space = chunks[1].width.saturating_sub(10) as usize;
    let filled_width = ((bar_space as f64) * (view.progress / 100.0)).round() as usize;
    let filled = "█".repeat(filled_width.min(bar_space));
    let empty = "░".repeat(bar_space.saturating_sub(filled.len()));
    let progress_text = format!("[{}{}] {:.0}%", filled, empty, view.progress);

    let progress_widget = Paragraph::new(progress_text)
        .style(Style::default().fg(get_orange_color()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(get_orange_accent()))
                .title("Progress")
                .title_style(
                    Style::default()
                        .fg(get_orange_color())
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .centered();
    frame.render_widget(progress_widget, chunks[1]);

    let current = if !view.current_service.is_empty() {
        format!(
            "Current: {} ({}/{})",
            view.current_service, view.completed_services, view.total_services
        )
    } else {
        "Initializing...".to_string()
    };
    let current_widget = Paragraph::new(current)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .centered();
    frame.render_widget(current_widget, chunks[2]);

    let log_lines: Vec<Line> = view
        .logs
        .iter()
        .map(|log| {
            let style = if log.contains("❌") || log.to_lowercase().contains("error") {
                Style::default().fg(Color::Red)
            } else if log.contains("✅") || log.contains("started") {
                Style::default().fg(Color::Green)
            } else if log.contains("⬇️") {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(log.clone(), style))
        })
        .collect();

    let logs_widget = Paragraph::new(log_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(get_orange_accent()))
                .title("📋 Installation Logs")
                .title_style(
                    Style::default()
                        .fg(get_orange_color())
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: false })
        .scroll((
            view.logs
                .len()
                .saturating_sub(chunks[3].height as usize - 2) as u16,
            0,
        ));
    frame.render_widget(logs_widget, chunks[3]);

    let help = Paragraph::new("Press Ctrl+C to cancel")
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    frame.render_widget(help, chunks[4]);
}
