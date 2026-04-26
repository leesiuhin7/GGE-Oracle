use std::io::{Read, Seek};

use crate::data::primitives::{
    encode_optional_string, encode_optional_varint_array, encode_varint_optional_i64,
    encode_varint_u64, read_bytes, read_varint_bytes,
};

use super::primitives::{
    DecodeStringError, DecodeVarintError, decode_optional_string, decode_varint_u64,
};

use super::structures;

pub struct Interface<'a, R: Read + Seek> {
    buffer: Vec<u8>,
    reader: &'a mut R,
}

impl<'a, R: Read + Seek> Interface<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Interface {
            buffer: Vec::new(),
            reader,
        }
    }

    pub fn finalize(mut self) -> Vec<u8> {
        let mut bytes = encode_varint_u64(self.buffer.len() as u64);
        bytes.append(&mut self.buffer);
        bytes
    }

    pub fn get_size(&mut self) -> Result<u64, DecodeVarintError> {
        decode_varint_u64(&mut self.reader)
    }

    pub fn get_id(&mut self) -> Result<u32, std::io::Error> {
        let mut buffer: [u8; 4] = [0; 4];
        self.reader.read_exact(&mut buffer)?;
        Ok(u32::from_be_bytes(buffer))
    }

    pub fn push_id(&mut self, id: u32) -> Result<(), std::io::Error> {
        let mut buffer: [u8; 4] = [0; 4];
        self.reader.read_exact(&mut buffer)?;
        self.buffer.extend(u32::to_be_bytes(id));
        Ok(())
    }

    pub fn get_server(&mut self) -> Result<String, DecodeStringError> {
        // Using unwrap here since under no circumstances should server be None
        Ok(decode_optional_string(self.reader)?.unwrap())
    }

    pub fn push_server(&mut self, server: String) -> Result<(), DecodeStringError> {
        self.get_server()?;
        self.buffer.extend(encode_optional_string(Some(server)));
        Ok(())
    }

    pub fn push_timestamp(&mut self, timestamp: i32) -> Result<(), structures::delta::Error> {
        let mut data = structures::delta::update(self.reader, Some(timestamp))?;
        self.buffer.append(&mut data);
        Ok(())
    }

    pub fn push_timer(&mut self, timer: Option<i64>) -> Result<(), structures::delta_rle::Error> {
        let mut data = structures::delta_rle::update(self.reader, timer)?;
        self.buffer.append(&mut data);
        Ok(())
    }

    pub fn push_delta_i64(
        &mut self,
        value: Option<i64>,
    ) -> Result<(), structures::delta_rle::Error> {
        let mut data = structures::delta_rle::update(self.reader, value)?;
        self.buffer.append(&mut data);
        Ok(())
    }

    pub fn push_i64(&mut self, value: Option<i64>) -> Result<(), structures::rle::Error> {
        let mut data =
            structures::rle::update(self.reader, &encode_varint_optional_i64(value), |reader| {
                read_varint_bytes(reader).map_err(|_| ())
            })?;
        self.buffer.append(&mut data);
        Ok(())
    }

    pub fn push_string(&mut self, value: Option<String>) -> Result<(), structures::rle::Error> {
        let mut data =
            structures::rle::update(self.reader, &encode_optional_string(value), |reader| {
                read_bytes(reader, 1).map_err(|_| ())
            })?;
        self.buffer.append(&mut data);
        Ok(())
    }

    pub fn push_vec(&mut self, vec: Option<Vec<i64>>) -> Result<(), structures::rle::Error> {
        let mut data =
            structures::rle::update(self.reader, &encode_optional_varint_array(vec), |reader| {
                read_bytes(reader, 1).map_err(|_| ())
            })?;
        self.buffer.append(&mut data);
        Ok(())
    }
}

impl<R: Read + Seek> Seek for Interface<'_, R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}
