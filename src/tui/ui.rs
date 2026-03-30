use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::app::{Action, App, Screen};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const ERR: Color = Color::Red;
const OK: Color = Color::Green;

pub fn draw(f: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Home => draw_home(f, app),
        Screen::Working { message } => draw_working(f, message),
        Screen::ScanResults => draw_scan(f, app),
        Screen::DedupResults => draw_dedup(f, app),
        Screen::OrganizePreview => draw_organize(f, app),
        Screen::Modal { title, body } => draw_modal(f, title, body),
    }
}

// ── Home ────────────────────────────────────────────────────────────────────

fn draw_home(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Length(5),  // action picker
            Constraint::Length(3),  // src input
            Constraint::Length(3),  // dest input (shown only for Organize)
            Constraint::Length(3),  // toggles
            Constraint::Min(0),     // spacer
            Constraint::Length(1),  // help bar
        ])
        .split(area);

    // Title
    let title = Paragraph::new("photors — photo organizer")
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    // Action picker
    let actions = ["  Scan  ", "  Dedup  ", "  Organize  "];
    let action_spans: Vec<Span> = actions
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let selected = match (&app.selected_action, i) {
                (Action::Scan, 0) | (Action::Dedup, 1) | (Action::Organize, 2) => true,
                _ => false,
            };
            if selected {
                Span::styled(*label, Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(*label, Style::default().fg(DIM))
            }
        })
        .collect();
    let action_block = Block::default()
        .borders(Borders::ALL)
        .title(" Action  (←/→ to switch) ");
    let action_para = Paragraph::new(Line::from(action_spans))
        .block(action_block);
    f.render_widget(action_para, chunks[1]);

    // Source input
    let src_style = if app.focused_field == 0 {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(DIM)
    };
    let src = Paragraph::new(app.src_input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Source directory ").border_style(src_style));
    f.render_widget(src, chunks[2]);

    // Dest input (only relevant for Organize)
    let dest_label = if app.selected_action == Action::Organize {
        " Destination directory "
    } else {
        " Destination directory (only for Organize) "
    };
    let dest_style = if app.focused_field == 1 && app.selected_action == Action::Organize {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(DIM)
    };
    let dest = Paragraph::new(app.dest_input.as_str())
        .block(Block::default().borders(Borders::ALL).title(dest_label).border_style(dest_style));
    f.render_widget(dest, chunks[3]);

    // Toggles
    let rec_label = toggle_span("r", "ecursive", app.recursive);
    let copy_label = toggle_span("c", "opy (don't move)", app.copy_mode);
    let toggles = Paragraph::new(Line::from(vec![
        rec_label,
        Span::raw("    "),
        copy_label,
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Options "));
    f.render_widget(toggles, chunks[4]);

    // Help bar
    let help = Paragraph::new(
        "Tab: switch field  ←/→: action  r: recursive  c: copy  Enter: run  q: quit",
    )
    .style(Style::default().fg(DIM));
    f.render_widget(help, chunks[6]);

    // Cursor in active field
    let (field_chunk, input_len) = if app.focused_field == 0 {
        (chunks[2], app.src_input.len())
    } else {
        (chunks[3], app.dest_input.len())
    };
    f.set_cursor_position((
        field_chunk.x + 1 + input_len as u16,
        field_chunk.y + 1,
    ));
}

fn toggle_span<'a>(key: &'a str, label: &'a str, on: bool) -> Span<'a> {
    if on {
        Span::styled(
            format!("[{key}]{label}"),
            Style::default().fg(OK).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!("[{key}]{label}"),
            Style::default().fg(DIM),
        )
    }
}

// ── Working ─────────────────────────────────────────────────────────────────

fn draw_working(f: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, f.area());
    let para = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL).title(" Working… "))
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, area);
    f.render_widget(para, area);
}

// ── Scan results ─────────────────────────────────────────────────────────────

fn draw_scan(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let visible = chunks[0].height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .scan_files
        .iter()
        .skip(app.scan_offset)
        .take(visible)
        .enumerate()
        .map(|(i, p)| {
            let abs = i + app.scan_offset;
            let style = if abs == app.scan_cursor {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else {
                Style::default()
            };
            ListItem::new(p.display().to_string()).style(style)
        })
        .collect();

    let title = format!(
        " Scan results — {} files (↑/↓ scroll, Esc/q back) ",
        app.scan_files.len()
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    let mut state = ListState::default();
    f.render_stateful_widget(list, chunks[0], &mut state);

    let help = Paragraph::new("↑/↓: scroll   q: back to home")
        .style(Style::default().fg(DIM));
    f.render_widget(help, chunks[1]);
}

// ── Dedup results ────────────────────────────────────────────────────────────

fn draw_dedup(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let visible = chunks[0].height.saturating_sub(2) as usize;

    // Build flat rows: for each group, first row = "keep", rest = "dup (delete?)"
    let mut rows: Vec<(String, bool /* is_dup */, bool /* marked_delete */)> = Vec::new();
    let mut dec_idx = 0usize;
    for group in &app.dedup_groups {
        for (i, path) in group.iter().enumerate() {
            if i == 0 {
                rows.push((format!("KEEP  {}", path.display()), false, false));
            } else {
                let del = app.dedup_decisions.get(dec_idx).copied().unwrap_or(true);
                rows.push((format!("  dup {}", path.display()), true, del));
                dec_idx += 1;
            }
        }
    }

    let items: Vec<ListItem> = rows
        .iter()
        .skip(app.dedup_offset)
        .take(visible)
        .enumerate()
        .map(|(i, (label, is_dup, del))| {
            let abs = i + app.dedup_offset;
            let style = if abs == app.dedup_cursor {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else if *is_dup && *del {
                Style::default().fg(ERR)
            } else if *is_dup {
                Style::default().fg(OK)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let prefix = if *is_dup {
                if *del { "[x] " } else { "[ ] " }
            } else {
                "    "
            };
            ListItem::new(format!("{prefix}{label}")).style(style)
        })
        .collect();

    let total_dups: usize = app.dedup_groups.iter().map(|g| g.len() - 1).sum();
    let marked: usize = app.dedup_decisions.iter().filter(|&&d| d).count();
    let title = format!(
        " Dedup — {} groups, {} duplicates, {} marked for deletion ",
        app.dedup_groups.len(),
        total_dups,
        marked
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    let mut state = ListState::default();
    f.render_stateful_widget(list, chunks[0], &mut state);

    let help = Paragraph::new(
        "↑/↓: move   Space: toggle delete   Enter: apply deletions   q: back",
    )
    .style(Style::default().fg(DIM));
    f.render_widget(help, chunks[1]);
}

// ── Organize preview ─────────────────────────────────────────────────────────

fn draw_organize(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let visible = chunks[0].height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .planned_moves
        .iter()
        .skip(app.organize_offset)
        .take(visible)
        .enumerate()
        .map(|(i, m)| {
            let abs = i + app.organize_offset;
            let style = if abs == app.organize_cursor {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else {
                Style::default()
            };
            let src = m.src.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            let dest = m.dest.display().to_string();
            ListItem::new(format!("{src}  →  {dest}")).style(style)
        })
        .collect();

    let status = if app.organize_done { " ✓ done" } else { "" };
    let title = format!(
        " Organize preview — {} moves{status}  (Enter to confirm) ",
        app.planned_moves.len()
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    let mut state = ListState::default();
    f.render_stateful_widget(list, chunks[0], &mut state);

    let help = if app.organize_done {
        "Done!   q: back to home"
    } else {
        "↑/↓: scroll   Enter: confirm & move files   q: back"
    };
    let help = Paragraph::new(help).style(Style::default().fg(DIM));
    f.render_widget(help, chunks[1]);
}

// ── Modal ────────────────────────────────────────────────────────────────────

fn draw_modal(f: &mut Frame, title: &str, body: &str) {
    let area = centered_rect(70, 40, f.area());
    let para = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} "))
                .border_style(Style::default().fg(ERR)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, area);
    f.render_widget(para, area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
