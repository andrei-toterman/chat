mod client;
mod server;

use bytes::{Buf, BufMut, BytesMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Default)]
pub struct BincodeCodec<T>(std::marker::PhantomData<T>);

impl<T> BincodeCodec<T> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<T: Serialize> Encoder<T> for BincodeCodec<T> {
    type Error = bincode::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        bincode::serialize_into(dst.writer(), &item).map_err(Into::into)
    }
}

impl<T: DeserializeOwned> Decoder for BincodeCodec<T> {
    type Item = T;
    type Error = bincode::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        bincode::deserialize_from(src.reader())
            .map_err(Into::into)
            .map(Option::Some)
    }
}
