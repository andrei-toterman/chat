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

impl State {
    fn broadcast(&self, message: Message) {
        let _ = self.sender.send(message);
    }
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

    let username = loop {
        match from_client.next().await {
            None => return Ok(()),
            Some(Ok(client::Message::Join(username))) => {
                let username = Arc::new(username);
                if state.usernames.write().await.insert(username.clone()) {
                    break username;
                }
                to_client.send(Message::Err(Error::UsernameTaken)).await?;
            }
            Some(Err(error)) => {
                eprintln!("client {address} erred asking for username: {error:?}");
                to_client.send(Message::Err(Error::Internal)).await?;
            }
            _ => {}
        }
    };

    let result: std::io::Result<()> = async {
        let mut receiver = state.sender.subscribe();
        state.broadcast(Message::Joined(username.clone()));
        println!("client {address} joined as {username}");

        loop {
            tokio::select! {
                server_event = receiver.recv() => match server_event {
                    Ok(message @ (Message::Said(_, _) | Message::Joined(_) | Message::Left(_))) => {
                        to_client.send(message).await?
                    }
                    Err(RecvError::Lagged(lost)) => {
                        to_client.send(Message::Err(Error::Lost(lost))).await?;
                    }
                    _ => {}
                },
                client_event = from_client.next() => match client_event {
                    None => break,
                    Some(Ok(client::Message::Say(what))) => {
                        let what = Arc::new(what);
                        state.broadcast(Message::Said(username.clone(), what.clone()));
                        println!("client {address} aka {username} said: {what}");
                    },
                    Some(Err(error))=> {
                        eprintln!("client {address} erred asking for message: {error:?}");
                        to_client.send(Message::Err(Error::Internal)).await?;
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }
    .await;

    state.broadcast(Message::Left(username.clone()));
    state.usernames.write().await.remove(&username);
    println!("client {address} aka {username} left");

    result
}
