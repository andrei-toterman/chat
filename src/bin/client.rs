use crossterm::{
    event::{Event, EventStream, KeyCode},
    terminal::{self, EnterAlternateScreen},
    ExecutableCommand,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_serde::{formats::SymmetricalBincode, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use unicode_width::UnicodeWidthStr;

use chat::{client::Message, server};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // let address = std::env::args().nth(1).expect("Usage: client <address>");

    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // let (reader, writer) = TcpStream::connect(address).await?.into_split();

    // let mut from_server = SymmetricallyFramed::new(
    //     FramedRead::new(reader, LengthDelimitedCodec::new()),
    //     SymmetricalBincode::default(),
    // );
    // let mut to_server = SymmetricallyFramed::new(
    //     FramedWrite::new(writer, LengthDelimitedCodec::new()),
    //     SymmetricalBincode::default(),
    // );

    let mut events = EventStream::new().fuse();

    let mut input = String::new();
    let mut chat = Vec::<server::Message>::new();

    // to_server.send(Message::Join(input.clone())).await?;

    loop {
        terminal
            .draw(|frame| {
                let chunks = Layout::default()
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                    .split(frame.size());

                frame.render_widget(
                    List::new(
                        chat.iter()
                            .map(|message| ListItem::new(message.to_string()))
                            .collect::<Vec<_>>(),
                    )
                    .block(Block::default().borders(Borders::ALL).title("Chat")),
                    chunks[0],
                );

                frame.render_widget(
                    Paragraph::new(input.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::default().borders(Borders::ALL).title("Input")),
                    chunks[1],
                );

                frame.set_cursor(chunks[1].x + input.width() as u16 + 1, chunks[1].y + 1);
            })
            .expect("failed to draw");

        // match from_server.next().await {
        //     None => break,
        //     Some(message_result) => chat.push(message_result?),
        // }

        match events.next().await {
            None => break,
            Some(event_result) => {
                if let Event::Key(key) = event_result? {
                    match key.code {
                        KeyCode::Char(c) => input.push(c),
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Enter => {
                            chat.push(server::Message::Said(
                                Arc::new("aaaa".into()),
                                Arc::new(input.clone()),
                            ));
                            input.clear();
                        }
                        KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}
