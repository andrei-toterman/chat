use crossterm::{
    event::{Event, EventStream, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_serde::{formats::SymmetricalBincode, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use tui_input::{backend::crossterm as input_backend, Input};

use chat::{client::Message, server};

#[derive(Default)]
struct State {
    input: Input,
    chat: Vec<server::Message>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = std::env::args().nth(1).expect("Usage: client <address>");

    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let (reader, writer) = TcpStream::connect(address).await?.into_split();
    let mut from_server = SymmetricallyFramed::new(
        FramedRead::new(reader, LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );
    let mut to_server = SymmetricallyFramed::new(
        FramedWrite::new(writer, LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );

    let mut state = State::default();
    let mut events = EventStream::new().fuse();

    // state.username = loop {
    //     terminal.draw(|f| {
    //         Lay
    //     })?;
    //     match events.next().await {
    //         None => break None,
    //         Some()
    //     }
    // }

    to_server
        .send(Message::Join(std::env::args().nth(2).expect("No username")))
        .await?;
    loop {
        terminal.draw(|f| draw(f, &state))?;

        tokio::select! {
            server_event = from_server.next() => match server_event {
                None => break,
                Some(message_result) => state.chat.push(message_result?),
            },
            client_event = events.next() => match client_event {
                None => break,
                Some(event_result) => {
                    if let Event::Key(key) = event_result? {
                        match key.code {
                            KeyCode::Esc => break,
                            KeyCode::Enter => {
                                to_server.send(Message::Say(state.input.value().into())).await?;
                                state.input.reset();
                            }
                            _ => {
                                    input_backend::to_input_request(Event::Key(key))
                                        .and_then(|req| state.input.handle(req));
                            }
                        }
                    }
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn draw<B: Backend>(f: &mut Frame<B>, state: &State) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.size());

    let scroll_messages = state
        .chat
        .len()
        .saturating_sub(chunks[0].height as usize - 2);
    f.render_widget(
        List::new(
            state
                .chat
                .iter()
                .skip(scroll_messages)
                .map(|m| ListItem::new(m.to_string()))
                .collect::<Vec<_>>(),
        )
        .block(Block::default().borders(Borders::ALL).title("Chat")),
        chunks[0],
    );

    let width = chunks[2].width.saturating_sub(3);
    f.render_widget(
        Paragraph::new(state.input.value())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .scroll((0, (state.input.cursor() as u16).saturating_sub(width)))
            .style(Style::default().fg(Color::Yellow)),
        chunks[1],
    );
    f.set_cursor(
        chunks[1].x + (state.input.cursor() as u16).min(width) + 1,
        chunks[1].y + 1,
    );

    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::raw("Press"),
            Span::styled(" Esc ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("to exit,"),
            Span::styled(" Tab ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("to toggle between writing and scrolling"),
        ])),
        chunks[2],
    );
}
