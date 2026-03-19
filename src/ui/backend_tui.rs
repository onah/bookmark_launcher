use crate::app::Entry;
use crate::app::{App, SearchMode};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::error::Error;
use std::io;
use std::time::Duration;

fn build_highlighted_item(
    entry: &Entry,
    matched: &std::collections::HashSet<usize>,
) -> ListItem<'static> {
    let highlight_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default();

    let title = entry.title();
    let suffix = match entry {
        Entry::Bookmark { url, .. } => format!(" ({})", url),
        Entry::App { command, args, .. } => {
            if args.is_empty() {
                format!(" ({})", command)
            } else {
                format!(" ({} {})", command, args.join(" "))
            }
        }
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut cur_highlighted = false;

    for (i, c) in title.chars().enumerate() {
        let is_highlighted = matched.contains(&i);
        if is_highlighted != cur_highlighted && !buf.is_empty() {
            let style = if cur_highlighted {
                highlight_style
            } else {
                normal_style
            };
            spans.push(Span::styled(buf.clone(), style));
            buf.clear();
        }
        cur_highlighted = is_highlighted;
        buf.push(c);
    }
    if !buf.is_empty() {
        let style = if cur_highlighted {
            highlight_style
        } else {
            normal_style
        };
        spans.push(Span::styled(buf, style));
    }
    spans.push(Span::raw(suffix));

    ListItem::new(Line::from(spans))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::collections::HashSet;

    fn bookmark_entry(title: &str) -> Entry {
        Entry::Bookmark {
            title: title.to_string(),
            url: "https://example.com".to_string(),
            access_count: 0,
        }
    }

    #[test]
    fn no_matched_chars_renders_no_yellow() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = HashSet::new();
        let item = build_highlighted_item(&entry, &matched);

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        for x in 0..6u16 {
            assert_ne!(
                buffer.get(x, 0).fg,
                Color::Yellow,
                "char at x={} should not be yellow",
                x
            );
        }
    }

    #[test]
    fn matched_chars_are_rendered_yellow() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = [0, 3].into_iter().collect(); // G, H
        let item = build_highlighted_item(&entry, &matched);

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        assert_eq!(
            buffer.get(0, 0).fg,
            Color::Yellow,
            "'G' at x=0 should be Yellow"
        );
        assert_eq!(
            buffer.get(3, 0).fg,
            Color::Yellow,
            "'H' at x=3 should be Yellow"
        );
        assert_ne!(
            buffer.get(1, 0).fg,
            Color::Yellow,
            "'i' at x=1 should not be Yellow"
        );
        assert_ne!(
            buffer.get(2, 0).fg,
            Color::Yellow,
            "'t' at x=2 should not be Yellow"
        );
    }

    #[test]
    fn suffix_is_not_yellow() {
        let entry = bookmark_entry("Go");
        let matched: HashSet<usize> = [0, 1].into_iter().collect(); // both chars matched
        let item = build_highlighted_item(&entry, &matched);

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // suffix " (https://example.com)" starts at x=2
        // x=2 is ' ' (space before paren) — should not be yellow
        assert_ne!(
            buffer.get(2, 0).fg,
            Color::Yellow,
            "suffix space should not be yellow"
        );
    }
}

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
        let search_results = app.search(app.query());
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

            let mode_label = if app.search_mode() == SearchMode::Migemo && !app.is_migemo_ready() {
                "Migemo (dict missing)"
            } else {
                app.search_mode_label()
            };

            let input_title = format!("Query [{}]", mode_label);
            let input = Paragraph::new(app.query())
                .block(Block::default().borders(Borders::ALL).title(input_title));
            f.render_widget(input, chunks[0]);

            let items: Vec<ListItem> = filtered_indices
                .iter()
                .filter_map(|idx| app.bookmarks().get(*idx))
                .map(|entry| {
                    let positions: std::collections::HashSet<usize> = app
                        .match_char_positions(entry.title(), app.query())
                        .into_iter()
                        .collect();
                    build_highlighted_item(entry, &positions)
                })
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
        if event::poll(Duration::from_millis(50))?
            && let CEvent::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('f') => {
                        app.set_search_mode(SearchMode::Fuzzy);
                        selected = 0;
                        continue;
                    }
                    KeyCode::Char('t') => {
                        // Toggle to Migemo mode
                        app.set_search_mode(SearchMode::Migemo);
                        selected = 0;
                        continue;
                    }
                    KeyCode::Char('q') => {
                        // Ctrl+Q to quit
                        break;
                    }
                    _ => {}
                }
            }

            match key.code {
                KeyCode::Esc => break,
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
                    selected = selected.saturating_sub(1);
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

    // restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
