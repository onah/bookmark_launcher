use crate::app::App;
use crate::app::Entry;
use crossterm::event::{self, Event as CEvent, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::error::Error;
use std::io::{self};
use std::time::{Duration, Instant};

pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn Error>> {
    // initialize app state
    let mut app = App::new(bookmarks);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    let mut selected: usize = 0;
    // clear any previous typed input events left in the terminal and reset query
    while event::poll(Duration::from_millis(0))? {
        // drain pending events
        let _ = event::read()?;
    }
    app.query_mut().clear();

    loop {
        // render
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                .split(size);

            let input = Paragraph::new(app.query())
                .block(Block::default().borders(Borders::ALL).title("Query"));
            f.render_widget(input, chunks[0]);

            // compute fuzzy search results and display filtered list
            let search_results = app.fuzzy_search(app.query());
            let filtered_indices: Vec<usize> = search_results.iter().map(|(i, _)| *i).collect();

            let items: Vec<ListItem> = filtered_indices
                .iter()
                .filter_map(|idx| app.bookmarks().get(*idx))
                .map(|e| match e {
                    Entry::Bookmark { title, url, .. } => {
                        ListItem::new(Span::raw(format!("{} ({})", title, url)))
                    }
                    Entry::App { title, command, .. } => {
                        ListItem::new(Span::raw(format!("{} (app: {})", title, command)))
                    }
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            let mut state = ratatui::widgets::ListState::default();
            if !filtered_indices.is_empty() {
                // clamp selected to available range
                if selected >= filtered_indices.len() {
                    selected = filtered_indices.len() - 1;
                }
                state.select(Some(selected));
            } else {
                state.select(None);
            }

            f.render_stateful_widget(list, chunks[1], &mut state);
        })?;

        // input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(c) => {
                        app.query_mut().push(c);
                        // reset selection when query changes
                        selected = 0;
                    }
                    KeyCode::Backspace => {
                        app.query_mut().pop();
                        selected = 0;
                    }
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        // move within filtered results
                        let max = app.fuzzy_search(app.query()).len();
                        if selected + 1 < max {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // operate on filtered selection
                        let results = app.fuzzy_search(app.query());
                        if let Some((idx, _)) = results.get(selected) {
                            let real_idx = *idx;
                            // increment access count and persist
                            let _ = app.increment_access_count_by_index(real_idx);
                            if let Some(entry) = app.bookmarks().get(real_idx) {
                                match entry {
                                    Entry::Bookmark { url, .. } => {
                                        let _ = open::that(url);
                                    }
                                    Entry::App { command, args, .. } => {
                                        let mut cmd = std::process::Command::new(command);
                                        if !args.is_empty() {
                                            cmd.args(args);
                                        }
                                        let _ = cmd.spawn();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // restore terminal
    disable_raw_mode()?;
    terminal.show_cursor()?;

    Ok(())
}
