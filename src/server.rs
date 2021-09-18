use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use futures::{SinkExt, StreamExt};
use parse_display::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{
        broadcast::{self, error::RecvError},
        RwLock,
    },
};
use tokio_serde::{formats::SymmetricalBincode, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::client;

#[derive(Debug, Display, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    #[display("{0} has joined the chat")]
    Joined(Arc<String>),
    #[display("{0}: {1}")]
    Said(Arc<String>, Arc<String>),
    #[display("{0} has left the chat")]
    Left(Arc<String>),
    #[display("{0}")]
    Err(Error),
}

#[derive(Debug, Error, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Error {
    #[error("that username is already taken")]
    UsernameTaken,
    #[error("internal server error")]
    Internal,
    #[error("lost {0} messages due to connection issues")]
    Lost(u64),
}

pub struct State {
    usernames: RwLock<HashSet<Arc<String>>>,
    sender: broadcast::Sender<Message>,
}

impl Default for State {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(64);
        Self {
            usernames: Default::default(),
            sender,
        }
    }
}

impl State {
    fn broadcast(&self, message: Message) {
        let _ = self.sender.send(message);
    }
}

pub async fn handle_client(
    state: Arc<State>,
    stream: TcpStream,
    address: SocketAddr,
) -> std::io::Result<()> {
    let (reader, writer) = stream.into_split();
    let mut from_client = SymmetricallyFramed::new(
        FramedRead::new(reader, LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );
    let mut to_client = SymmetricallyFramed::new(
        FramedWrite::new(writer, LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );

    // loop label is here only because my IDE has a bug and it needs it
    let username = 'username: loop {
        match from_client.next().await {
            None => return Ok(()),
            Some(result) => match result {
                Ok(client::Message::Join(username)) => {
                    let username = Arc::new(username);
                    if state.usernames.write().await.insert(username.clone()) {
                        break 'username username;
                    }
                    to_client.send(Message::Err(Error::UsernameTaken)).await?;
                }
                Err(error) => {
                    eprintln!("client {} erred asking for username: {:?}", address, error);
                    to_client.send(Message::Err(Error::Internal)).await?;
                }
                _ => {}
            },
        }
    };

    // everything is in one big block so that i can run some additional code no matter how it ends
    // bootleg defer
    let result: std::io::Result<()> = async {
        let mut receiver = state.sender.subscribe();
        state.broadcast(Message::Joined(username.clone()));
        println!("client {} joined as {}", address, username);

        loop {
            tokio::select! {
                result = receiver.recv() => match result {
                    Ok(message) => {
                        if matches!(
                            message,
                            Message::Said(_, _) | Message::Joined(_) | Message::Left(_)
                        ) {
                            to_client.send(message).await?;
                        }
                    }
                    Err(RecvError::Lagged(lost)) => {
                        to_client.send(Message::Err(Error::Lost(lost))).await?;
                    }
                    _ => {}
                },
                option = from_client.next() => match option {
                    None => break,
                    Some(result) => match result {
                        Ok(message) => match message {
                            client::Message::Leave => break,
                            client::Message::Say(what) => {
                                let what = Arc::new(what);
                                state.broadcast(Message::Said(username.clone(), what.clone()));
                                println!("client {} aka {} said: {}", address, username, what);
                            }
                            _ => {}
                        },
                        Err(error) => {
                            eprintln!("client {} erred asking for message: {:?}", address,  error);
                            to_client.send(Message::Err(Error::Internal)).await?;
                        }
                    },
                }
            }
        }
        Ok(())
    }
    .await;

    state.broadcast(Message::Left(username.clone()));
    state.usernames.write().await.remove(&username);
    println!("client {} aka {} left", address, username);

    result
}
