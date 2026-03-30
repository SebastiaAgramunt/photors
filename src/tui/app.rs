use std::path::PathBuf;

/// Which screen is currently shown.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    /// Main menu: choose action and enter paths.
    Home,
    /// A blocking operation is running (scan / hash). Shows a spinner.
    Working { message: String },
    /// Scan results: list of found files.
    ScanResults,
    /// Dedup results: groups of duplicates to review.
    DedupResults,
    /// Organize preview: list of planned moves.
    OrganizePreview,
    /// A modal message (error or info) with an OK prompt.
    Modal { title: String, body: String },
}

/// Which action the user selected on the home screen.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Scan,
    Dedup,
    Organize,
}

/// One planned file move produced by the organize step.
#[derive(Debug, Clone)]
pub struct PlannedMove {
    pub src: PathBuf,
    pub dest: PathBuf,
}

/// Global application state.
pub struct App {
    pub screen: Screen,
    pub should_quit: bool,

    // ── Home screen inputs ──────────────────────────────────────────────────
    pub selected_action: Action,
    pub src_input: String,
    pub dest_input: String,
    pub recursive: bool,
    pub copy_mode: bool,
    /// Which text field has focus: 0 = src, 1 = dest
    pub focused_field: usize,

    // ── Scan results ────────────────────────────────────────────────────────
    pub scan_files: Vec<PathBuf>,
    pub scan_cursor: usize,
    pub scan_offset: usize,

    // ── Dedup results ───────────────────────────────────────────────────────
    /// Each group: [keep, dup1, dup2, …]
    pub dedup_groups: Vec<Vec<PathBuf>>,
    /// Per-duplicate keep/delete decision (indexed flat across all dups)
    pub dedup_decisions: Vec<bool>, // true = delete
    pub dedup_cursor: usize,
    pub dedup_offset: usize,

    // ── Organize preview ────────────────────────────────────────────────────
    pub planned_moves: Vec<PlannedMove>,
    pub organize_cursor: usize,
    pub organize_offset: usize,
    pub organize_done: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Home,
            should_quit: false,

            selected_action: Action::Scan,
            src_input: String::new(),
            dest_input: String::new(),
            recursive: true,
            copy_mode: false,
            focused_field: 0,

            scan_files: Vec::new(),
            scan_cursor: 0,
            scan_offset: 0,

            dedup_groups: Vec::new(),
            dedup_decisions: Vec::new(),
            dedup_cursor: 0,
            dedup_offset: 0,

            planned_moves: Vec::new(),
            organize_cursor: 0,
            organize_offset: 0,
            organize_done: false,
        }
    }

    // ── Scroll helpers ──────────────────────────────────────────────────────

    pub fn scroll_up(&mut self, visible: usize) {
        match self.screen {
            Screen::ScanResults => {
                if self.scan_cursor > 0 {
                    self.scan_cursor -= 1;
                    if self.scan_cursor < self.scan_offset {
                        self.scan_offset = self.scan_cursor;
                    }
                }
            }
            Screen::DedupResults => {
                if self.dedup_cursor > 0 {
                    self.dedup_cursor -= 1;
                    if self.dedup_cursor < self.dedup_offset {
                        self.dedup_offset = self.dedup_cursor;
                    }
                }
            }
            Screen::OrganizePreview => {
                if self.organize_cursor > 0 {
                    self.organize_cursor -= 1;
                    if self.organize_cursor < self.organize_offset {
                        self.organize_offset = self.organize_cursor;
                    }
                }
            }
            _ => {}
        }
        let _ = visible;
    }

    pub fn scroll_down(&mut self, visible: usize) {
        match self.screen {
            Screen::ScanResults => {
                let max = self.scan_files.len().saturating_sub(1);
                if self.scan_cursor < max {
                    self.scan_cursor += 1;
                    if self.scan_cursor >= self.scan_offset + visible {
                        self.scan_offset = self.scan_cursor + 1 - visible;
                    }
                }
            }
            Screen::DedupResults => {
                let max = self.dedup_flat_len().saturating_sub(1);
                if self.dedup_cursor < max {
                    self.dedup_cursor += 1;
                    if self.dedup_cursor >= self.dedup_offset + visible {
                        self.dedup_offset = self.dedup_cursor + 1 - visible;
                    }
                }
            }
            Screen::OrganizePreview => {
                let max = self.planned_moves.len().saturating_sub(1);
                if self.organize_cursor < max {
                    self.organize_cursor += 1;
                    if self.organize_cursor >= self.organize_offset + visible {
                        self.organize_offset = self.organize_cursor + 1 - visible;
                    }
                }
            }
            _ => {}
        }
    }

    /// Total number of rows in the flat dedup view (header + dups per group).
    pub fn dedup_flat_len(&self) -> usize {
        self.dedup_groups.iter().map(|g| g.len()).sum()
    }

    /// Toggle delete decision for the dup currently under the cursor.
    pub fn dedup_toggle(&mut self) {
        // Build a flat index → (group, item) mapping
        let mut flat = 0usize;
        let mut dec_idx = 0usize;
        'outer: for group in &self.dedup_groups {
            for (i, _) in group.iter().enumerate() {
                if flat == self.dedup_cursor {
                    if i > 0 {
                        // Only dups (index > 0) have a decision entry
                        self.dedup_decisions[dec_idx] = !self.dedup_decisions[dec_idx];
                    }
                    break 'outer;
                }
                if i > 0 {
                    dec_idx += 1;
                }
                flat += 1;
            }
        }
    }
}
