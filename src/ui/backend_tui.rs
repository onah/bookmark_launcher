use crate::app::Entry;
use crate::app::{App, SearchMode};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

struct PendingSearch {
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<(usize, i64)>>,
}

fn highlighted_spans(
    text: &str,
    matched: &std::collections::HashSet<usize>,
    highlight_style: Style,
    normal_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut cur_highlighted = false;

    for (i, c) in text.chars().enumerate() {
        let is_highlighted = matched.contains(&i);
        if is_highlighted != cur_highlighted && !buf.is_empty() {
            let style = if cur_highlighted { highlight_style } else { normal_style };
            spans.push(Span::styled(buf.clone(), style));
            buf.clear();
        }
        cur_highlighted = is_highlighted;
        buf.push(c);
    }
    if !buf.is_empty() {
        let style = if cur_highlighted { highlight_style } else { normal_style };
        spans.push(Span::styled(buf, style));
    }
    spans
}

fn build_highlighted_item(
    entry: &Entry,
    title_matched: &std::collections::HashSet<usize>,
    secondary_matched: &std::collections::HashSet<usize>,
) -> ListItem<'static> {
    let highlight_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default();
    let count_style = Style::default().fg(Color::Cyan);

    let count_prefix = format!("{:>4} ", entry.access_count());
    let mut spans: Vec<Span<'static>> = vec![Span::styled(count_prefix, count_style)];

    spans.extend(highlighted_spans(entry.title(), title_matched, highlight_style, normal_style));

    let (secondary_text, args_suffix) = match entry {
        Entry::Bookmark { url, .. } => (url.as_str(), String::new()),
        Entry::App { command, args, .. } => {
            let suffix = if args.is_empty() {
                String::new()
            } else {
                format!(" {}", args.join(" "))
            };
            (command.as_str(), suffix)
        }
    };

    spans.push(Span::raw(" ("));
    spans.extend(highlighted_spans(secondary_text, secondary_matched, highlight_style, normal_style));
    spans.push(Span::raw(format!("{args_suffix})")));

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

    // count prefix is 5 chars ("   0 "), so title starts at x=5

    #[test]
    fn no_matched_chars_renders_no_yellow() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = HashSet::new();
        let item = build_highlighted_item(&entry, &matched, &HashSet::new());

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // title "GitHub" occupies x=5..11
        for x in 5..11u16 {
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
        let item = build_highlighted_item(&entry, &matched, &HashSet::new());

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // title starts at x=5: G(5) i(6) t(7) H(8) u(9) b(10)
        assert_eq!(buffer.get(5, 0).fg, Color::Yellow, "'G' at x=5 should be Yellow");
        assert_eq!(buffer.get(8, 0).fg, Color::Yellow, "'H' at x=8 should be Yellow");
        assert_ne!(buffer.get(6, 0).fg, Color::Yellow, "'i' at x=6 should not be Yellow");
        assert_ne!(buffer.get(7, 0).fg, Color::Yellow, "'t' at x=7 should not be Yellow");
    }

    #[test]
    fn suffix_separator_is_not_yellow() {
        let entry = bookmark_entry("Go");
        let matched: HashSet<usize> = [0, 1].into_iter().collect(); // both chars matched
        let item = build_highlighted_item(&entry, &matched, &HashSet::new());

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let list = List::new(vec![item]);
                f.render_widget(list, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // "Go" is at x=5,6; " (" separator at x=7,8; URL starts at x=9
        // separator should not be yellow (secondary_matched is empty)
        assert_ne!(
            buffer.get(7, 0).fg,
            Color::Yellow,
            "suffix separator should not be yellow"
        );
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
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
    let mut confirm_delete_idx: Option<usize> = None;
    // drain any pending events before starting
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    let mut search_results: Vec<(usize, i64)> = app.search(app.query());
    let mut pending: Option<PendingSearch> = None;
    let mut searched_query = app.query().to_string();
    let mut searched_mode = app.search_mode();

    loop {
        // Trigger new search when query or mode changed
        {
            let q = app.query().to_string();
            let m = app.search_mode();
            if q != searched_query || m != searched_mode {
                if let Some(ref p) = pending {
                    p.cancel.store(true, Ordering::Relaxed);
                }
                pending = None;
                match m {
                    SearchMode::Fuzzy => {
                        search_results = app.search(&q);
                    }
                    SearchMode::Migemo if q.is_empty() => {
                        search_results = app.search(&q);
                    }
                    SearchMode::Migemo => {
                        let cancel = Arc::new(AtomicBool::new(false));
                        if let Some(rx) = app.start_migemo_search(&q, Arc::clone(&cancel)) {
                            search_results = vec![];
                            pending = Some(PendingSearch { cancel, rx });
                        } else {
                            search_results = vec![];
                        }
                    }
                }
                searched_query = q;
                searched_mode = m;
            }
        }

        // Receive completed async search results
        if let Some(ref p) = pending {
            match p.rx.try_recv() {
                Ok(results) => {
                    search_results = results;
                    pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }

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
            let sort_label = if app.search_mode() == SearchMode::Fuzzy {
                app.sort_mode_label()
            } else {
                "Count"
            };

            let input_title = format!("Query [{} | {}]", mode_label, sort_label);
            let input = Paragraph::new(app.query())
                .block(Block::default().borders(Borders::ALL).title(input_title));
            f.render_widget(input, chunks[0]);

            let items: Vec<ListItem> = filtered_indices
                .iter()
                .filter_map(|idx| app.bookmarks().get(*idx))
                .map(|entry| {
                    let title_positions: std::collections::HashSet<usize> = app
                        .match_char_positions(entry.title(), app.query())
                        .into_iter()
                        .collect();
                    let secondary_positions: std::collections::HashSet<usize> = app
                        .match_secondary_positions(entry, app.query())
                        .into_iter()
                        .collect();
                    build_highlighted_item(entry, &title_positions, &secondary_positions)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(60, 60, 60))
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ratatui::widgets::ListState::default();
            if filtered_indices.is_empty() {
                state.select(None);
            } else {
                state.select(Some(selected));
            }

            f.render_stateful_widget(list, chunks[1], &mut state);

            if let Some(del_idx) = confirm_delete_idx {
                let title = app
                    .bookmarks()
                    .get(del_idx)
                    .map(|e| e.title().to_string())
                    .unwrap_or_default();
                let popup_area = centered_rect(50, 7, size);
                let text = vec![
                    Line::raw(""),
                    Line::styled(
                        format!("Delete \"{}\"?", title),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Line::raw(""),
                    Line::raw("[y] Delete    [n / Esc] Cancel"),
                    Line::raw(""),
                ];
                let popup = Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Confirm")
                            .style(Style::default().bg(Color::DarkGray)),
                    )
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false });
                f.render_widget(Clear, popup_area);
                f.render_widget(popup, popup_area);
            }
        })?;

        // input — shorter timeout while search is in flight for snappier result display
        let poll_timeout = if pending.is_some() {
            Duration::from_millis(10)
        } else {
            Duration::from_millis(50)
        };
        if event::poll(poll_timeout)?
            && let CEvent::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // confirm-delete popup handling
            if confirm_delete_idx.is_some() {
                match key.code {
                    KeyCode::Char('y') => {
                        if let Some(del_idx) = confirm_delete_idx.take() {
                            let _ = app.delete_bookmark_by_index(del_idx);
                            break;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Esc => {
                        confirm_delete_idx = None;
                    }
                    _ => {}
                }
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
                    KeyCode::Char('p') => {
                        selected = selected.saturating_sub(1);
                        continue;
                    }
                    KeyCode::Char('n') => {
                        if selected + 1 < search_results.len() {
                            selected += 1;
                        }
                        continue;
                    }
                    KeyCode::Char('s') => {
                        app.toggle_sort_mode();
                        selected = 0;
                        continue;
                    }
                    KeyCode::Char('q') => {
                        // Ctrl+Q to quit
                        break;
                    }
                    KeyCode::Char('d') => {
                        if let Some((real_idx, _)) = search_results.get(selected) {
                            confirm_delete_idx = Some(*real_idx);
                        }
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
                KeyCode::Down if selected + 1 < search_results.len() => {
                    selected += 1;
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
