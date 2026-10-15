use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Length-prefixed JSON-RPC 2.0 codec: 4-byte Big-Endian length followed by JSON payload
pub struct LengthPrefixedJsonCodec;

impl Decoder for LengthPrefixedJsonCodec {
    type Item = serde_json::Value;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if src.len() < 4 + length {
            src.reserve(4 + length - src.len());
            return Ok(None);
        }
        src.advance(4);
        let payload = src.split_to(length);
        let value = serde_json::from_slice(&payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(value))
    }
}

impl Encoder<serde_json::Value> for LengthPrefixedJsonCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: serde_json::Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json_bytes = serde_json::to_vec(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        dst.reserve(4 + json_bytes.len());
        dst.put_u32(json_bytes.len() as u32);
        dst.put_slice(&json_bytes);
        Ok(())
    }
}
