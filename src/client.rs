use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    Join(String),
    Say(String),
    Leave,
}

#[tokio::test]
async fn test_message_codec() {
    use crate::BincodeCodec;
    use futures::{SinkExt, StreamExt};
    use tokio_util::codec::{FramedRead, FramedWrite};

    let message1_send = Message::Join("username".to_owned());
    let message2_send = Message::Say("something".to_owned());
    let message3_send = Message::Leave;

    let buffer = Vec::new();
    let mut frame_writer = FramedWrite::new(buffer, BincodeCodec::new());
    frame_writer.send(message1_send.clone()).await.unwrap();
    frame_writer.send(message2_send.clone()).await.unwrap();
    frame_writer.send(message3_send.clone()).await.unwrap();

    let buffer = frame_writer.into_inner();
    let mut frame_reader = FramedRead::new(buffer.as_slice(), BincodeCodec::new());
    let message1_receive = frame_reader.next().await.unwrap().unwrap();
    let message2_receive = frame_reader.next().await.unwrap().unwrap();
    let message3_receive = frame_reader.next().await.unwrap().unwrap();

    assert_eq!(message1_send, message1_receive);
    assert_eq!(message2_send, message2_receive);
    assert_eq!(message3_send, message3_receive);
}
