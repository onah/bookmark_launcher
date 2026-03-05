use crate::app::App;
use crate::app::Entry;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::error::Error;
use std::io;
use std::time::Duration;

pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn Error>> {
    // initialize app state
    let mut app = App::new(bookmarks);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected: usize = 0;
    // drain any pending events before starting
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    loop {
        // compute search results once per iteration
        let search_results = app.fuzzy_search(app.query());
        let filtered_indices: Vec<usize> = search_results.iter().map(|(i, _)| *i).collect();

        // clamp selected before drawing
        if !filtered_indices.is_empty() && selected >= filtered_indices.len() {
            selected = filtered_indices.len() - 1;
        }

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

            let items: Vec<ListItem> = filtered_indices
                .iter()
                .filter_map(|idx| app.bookmarks().get(*idx))
                .map(|e| ListItem::new(e.display()))
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            let mut state = ratatui::widgets::ListState::default();
            if filtered_indices.is_empty() {
                state.select(None);
            } else {
                state.select(Some(selected));
            }

            f.render_stateful_widget(list, chunks[1], &mut state);
        })?;

        // input
        if event::poll(Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
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
                        if selected + 1 < search_results.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        let query = app.query().trim().to_string();
                        if !query.is_empty()
                            && (query.starts_with("http://")
                                || query.starts_with("https://")
                                || query.contains('.'))
                            && search_results.is_empty()
                        {
                            // URL-like input: save to bookmarks and open
                            let _ = app.add_bookmark(query.clone());
                            let _ = open::that(&query);
                            break;
                        } else if let Some((idx, _)) = search_results.get(selected) {
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
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
