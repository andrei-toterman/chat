use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    Join(String),
    Say(String),
    Leave,
}

pub struct MessageCodec;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct MessageCodecError(#[from] bincode::Error);

impl From<std::io::Error> for MessageCodecError {
    fn from(err: std::io::Error) -> Self {
        Self(err.into())
    }
}

impl Encoder<Message> for MessageCodec {
    type Error = MessageCodecError;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        bincode::serialize_into(dst.writer(), &item).map_err(Into::into)
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = MessageCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        bincode::deserialize_from(src.reader())
            .map(Option::Some)
            .map_err(Into::into)
    }
}

#[tokio::test]
async fn test_message_codec() {
    use futures::{SinkExt, StreamExt};
    use tokio_util::codec::{FramedRead, FramedWrite};

    let message1_send = Message::Join("username".to_owned());
    let message2_send = Message::Say("something".to_owned());
    let message3_send = Message::Leave;

    let buffer = Vec::new();
    let mut frame_writer = FramedWrite::new(buffer, MessageCodec);
    frame_writer.send(message1_send.clone()).await.unwrap();
    frame_writer.send(message2_send.clone()).await.unwrap();
    frame_writer.send(message3_send.clone()).await.unwrap();

    let buffer = frame_writer.into_inner();
    let mut frame_reader = FramedRead::new(buffer.as_slice(), MessageCodec);
    let message1_receive = frame_reader.next().await.unwrap().unwrap();
    let message2_receive = frame_reader.next().await.unwrap().unwrap();
    let message3_receive = frame_reader.next().await.unwrap().unwrap();

    assert_eq!(message1_send, message1_receive);
    assert_eq!(message2_send, message2_receive);
    assert_eq!(message3_send, message3_receive);
}
