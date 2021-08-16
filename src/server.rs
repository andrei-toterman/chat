use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    Joined(Arc<String>),
    Said(Arc<String>, Arc<String>),
    Left(Arc<String>),
}

#[tokio::test]
async fn test_message_codec() {
    use futures::{SinkExt, StreamExt};
    use tokio_serde::{formats::SymmetricalBincode, SymmetricallyFramed};
    pub use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

    let message1_send = Message::Joined("username".to_owned().into());
    let message2_send = Message::Said("username".to_owned().into(), "something".to_owned().into());
    let message3_send = Message::Left("username".to_owned().into());

    let buffer = Vec::new();
    let mut frame_writer = SymmetricallyFramed::new(
        FramedWrite::new(buffer, LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );
    frame_writer.send(message1_send.clone()).await.unwrap();
    frame_writer.send(message2_send.clone()).await.unwrap();
    frame_writer.send(message3_send.clone()).await.unwrap();

    let buffer = frame_writer.into_inner().into_inner();
    let mut frame_reader = SymmetricallyFramed::new(
        FramedRead::new(buffer.as_slice(), LengthDelimitedCodec::new()),
        SymmetricalBincode::default(),
    );
    let message1_receive = frame_reader.next().await.unwrap().unwrap();
    let message2_receive = frame_reader.next().await.unwrap().unwrap();
    let message3_receive = frame_reader.next().await.unwrap().unwrap();

    assert_eq!(message1_send, message1_receive);
    assert_eq!(message2_send, message2_receive);
    assert_eq!(message3_send, message3_receive);
}
