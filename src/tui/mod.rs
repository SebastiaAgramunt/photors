mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    collections::HashSet,
    io,
    path::PathBuf,
    time::Duration,
};

use app::{Action, App, PlannedMove, Screen};

use crate::core::{
    dedup::{delete_duplicates, find_duplicates},
    organize::{organize, OrganizeOptions},
    scan::{parse_exts, scan_media_files},
};

const DEFAULT_EXTS: &str = "jpg,jpeg,png,heic,mp4,mov,m4v,avi";

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match &app.screen.clone() {
            Screen::Home => handle_home(app, key.code)?,
            Screen::ScanResults => handle_scan(app, key.code),
            Screen::DedupResults => handle_dedup(app, key.code)?,
            Screen::OrganizePreview => handle_organize(app, key.code)?,
            Screen::Modal { .. } => {
                // Any key dismisses the modal and returns to home
                app.screen = Screen::Home;
            }
            Screen::Working { .. } => {} // no input while working
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

// ── Home ─────────────────────────────────────────────────────────────────────

fn handle_home(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') => app.should_quit = true,

        KeyCode::Left => {
            app.selected_action = match app.selected_action {
                Action::Scan => Action::Organize,
                Action::Dedup => Action::Scan,
                Action::Organize => Action::Dedup,
            };
        }
        KeyCode::Right => {
            app.selected_action = match app.selected_action {
                Action::Scan => Action::Dedup,
                Action::Dedup => Action::Organize,
                Action::Organize => Action::Scan,
            };
        }

        KeyCode::Tab => {
            app.focused_field = match app.selected_action {
                Action::Organize => 1 - app.focused_field,
                _ => 0,
            };
        }

        KeyCode::Char('r') => app.recursive = !app.recursive,
        KeyCode::Char('c') => app.copy_mode = !app.copy_mode,

        KeyCode::Char(ch) => {
            if app.focused_field == 0 {
                app.src_input.push(ch);
            } else {
                app.dest_input.push(ch);
            }
        }
        KeyCode::Backspace => {
            if app.focused_field == 0 {
                app.src_input.pop();
            } else {
                app.dest_input.pop();
            }
        }

        KeyCode::Enter => run_action(app)?,

        _ => {}
    }
    Ok(())
}

fn run_action(app: &mut App) -> Result<()> {
    let src = PathBuf::from(app.src_input.trim());
    if !src.is_dir() {
        app.screen = Screen::Modal {
            title: "Error".into(),
            body: format!("Not a directory:\n{}", src.display()),
        };
        return Ok(());
    }

    let allowed: HashSet<String> = parse_exts(DEFAULT_EXTS);

    match app.selected_action {
        Action::Scan => {
            app.screen = Screen::Working {
                message: format!("Scanning {}…", src.display()),
            };
            // Redraw so the user sees "Working" before we block
            let files = scan_media_files(&src, app.recursive, &allowed)?;
            app.scan_files = files;
            app.scan_cursor = 0;
            app.scan_offset = 0;
            app.screen = Screen::ScanResults;
        }

        Action::Dedup => {
            app.screen = Screen::Working {
                message: format!("Scanning & hashing {}…", src.display()),
            };
            let files = scan_media_files(&src, app.recursive, &allowed)?;
            let groups = find_duplicates(&files)?;
            let total_dups: usize = groups.iter().map(|g| g.len() - 1).sum();
            app.dedup_decisions = vec![true; total_dups]; // default: delete all dups
            app.dedup_groups = groups;
            app.dedup_cursor = 0;
            app.dedup_offset = 0;
            if app.dedup_groups.is_empty() {
                app.screen = Screen::Modal {
                    title: "Dedup".into(),
                    body: "No duplicates found.".into(),
                };
            } else {
                app.screen = Screen::DedupResults;
            }
        }

        Action::Organize => {
            let dest_str = app.dest_input.trim();
            if dest_str.is_empty() {
                app.screen = Screen::Modal {
                    title: "Error".into(),
                    body: "Please enter a destination directory.".into(),
                };
                return Ok(());
            }
            let dest = PathBuf::from(dest_str);

            app.screen = Screen::Working {
                message: format!("Scanning {}…", src.display()),
            };

            // Build the planned moves using dry-run organize
            let files = scan_media_files(&src, app.recursive, &allowed)?;
            let mut moves = Vec::new();
            for src_path in &files {
                let ext = src_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                let (subdir, new_stem) = match crate::core::organize::resolve_date_pub(src_path) {
                    Some(date) => (dest.join(date.subdir()), date.filename_stem()),
                    None => {
                        let stem = src_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        (dest.join("unknown"), stem)
                    }
                };

                let filename = if ext.is_empty() {
                    new_stem.clone()
                } else {
                    format!("{new_stem}.{ext}")
                };
                moves.push(PlannedMove {
                    src: src_path.clone(),
                    dest: subdir.join(filename),
                });
            }

            app.planned_moves = moves;
            app.organize_cursor = 0;
            app.organize_offset = 0;
            app.organize_done = false;
            app.screen = Screen::OrganizePreview;
        }
    }
    Ok(())
}

// ── Scan results ──────────────────────────────────────────────────────────────

fn handle_scan(app: &mut App, code: KeyCode) {
    let visible = 20; // approximate; ui uses actual height
    match code {
        KeyCode::Up => app.scroll_up(visible),
        KeyCode::Down => app.scroll_down(visible),
        KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::Home,
        _ => {}
    }
}

// ── Dedup results ─────────────────────────────────────────────────────────────

fn handle_dedup(app: &mut App, code: KeyCode) -> Result<()> {
    let visible = 20;
    match code {
        KeyCode::Up => app.scroll_up(visible),
        KeyCode::Down => app.scroll_down(visible),
        KeyCode::Char(' ') => app.dedup_toggle(),
        KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::Home,
        KeyCode::Enter => {
            // Apply deletions for dups marked true
            let mut to_delete: Vec<Vec<PathBuf>> = Vec::new();
            let mut dec_idx = 0usize;
            for group in &app.dedup_groups {
                let mut del_group = vec![group[0].clone()];
                for path in group.iter().skip(1) {
                    if app.dedup_decisions[dec_idx] {
                        del_group.push(path.clone());
                    }
                    dec_idx += 1;
                }
                if del_group.len() > 1 {
                    to_delete.push(del_group);
                }
            }
            let n = delete_duplicates(&to_delete, false)?;
            app.screen = Screen::Modal {
                title: "Dedup done".into(),
                body: format!("Deleted {n} duplicate files."),
            };
        }
        _ => {}
    }
    Ok(())
}

// ── Organize preview ──────────────────────────────────────────────────────────

fn handle_organize(app: &mut App, code: KeyCode) -> Result<()> {
    let visible = 20;
    match code {
        KeyCode::Up => app.scroll_up(visible),
        KeyCode::Down => app.scroll_down(visible),
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.organize_done {
                app.screen = Screen::Home;
            } else {
                app.screen = Screen::Home;
            }
        }
        KeyCode::Enter if !app.organize_done => {
            let allowed: HashSet<String> = parse_exts(DEFAULT_EXTS);
            let src = PathBuf::from(app.src_input.trim());
            let dest = PathBuf::from(app.dest_input.trim());
            let opts = OrganizeOptions {
                src: &src,
                dest: &dest,
                recursive: app.recursive,
                allowed_exts: &allowed,
                dry_run: false,
                copy: app.copy_mode,
            };
            let result = organize(&opts)?;
            app.organize_done = true;
            app.screen = Screen::Modal {
                title: "Organize done".into(),
                body: format!(
                    "{} files {}.\n{} errors.",
                    result.moved,
                    if app.copy_mode { "copied" } else { "moved" },
                    result.errors
                ),
            };
        }
        _ => {}
    }
    Ok(())
}
