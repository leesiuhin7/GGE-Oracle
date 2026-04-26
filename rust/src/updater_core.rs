use std::io::{Cursor, Read, Seek, Write};
use std::num::TryFromIntError;

use crate::data::DecodeStringError;
use crate::{
    data::{Block, DecodeVarintError, Interface},
    index::{Index, Key, Value},
    types::Document,
};

struct KeyValuePair {
    key: Key,
    value: Value,
}

enum GetBufferError {
    NotFound,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum InitError {
    GetBufferError,
    Io(std::io::Error),
    Varint(DecodeVarintError),
    String(DecodeStringError),
}

impl From<GetBufferError> for InitError {
    fn from(_: GetBufferError) -> Self {
        InitError::GetBufferError
    }
}

impl From<std::io::Error> for InitError {
    fn from(value: std::io::Error) -> Self {
        InitError::Io(value)
    }
}

impl From<DecodeVarintError> for InitError {
    fn from(value: DecodeVarintError) -> Self {
        InitError::Varint(value)
    }
}

impl From<DecodeStringError> for InitError {
    fn from(value: DecodeStringError) -> Self {
        InitError::String(value)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum UpdateError {
    GetBufferError,
    Io(std::io::Error),
    Update(crate::data::UpdateError),
}

impl From<GetBufferError> for UpdateError {
    fn from(_: GetBufferError) -> Self {
        UpdateError::GetBufferError
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(value: std::io::Error) -> Self {
        UpdateError::Io(value)
    }
}

impl From<crate::data::UpdateError> for UpdateError {
    fn from(value: crate::data::UpdateError) -> Self {
        UpdateError::Update(value)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum FinalizeError {
    Io(std::io::Error),
    GetBufferError,
    OutOfRange(TryFromIntError),
}

impl From<std::io::Error> for FinalizeError {
    fn from(value: std::io::Error) -> Self {
        FinalizeError::Io(value)
    }
}

impl From<GetBufferError> for FinalizeError {
    fn from(_: GetBufferError) -> Self {
        FinalizeError::GetBufferError
    }
}

impl From<TryFromIntError> for FinalizeError {
    fn from(value: TryFromIntError) -> Self {
        FinalizeError::OutOfRange(value)
    }
}

struct Buffers<'a, R: Read + Seek, W: Write> {
    input_buffer: Result<&'a mut R, GetBufferError>,
    output_buffer: Result<&'a mut W, GetBufferError>,
}

pub struct Updater<R: Read + Seek, W: Write> {
    input_buffer: Option<R>,
    output_buffer: Option<W>,
    index: Index,
}

impl<R: Read + Seek, W: Write> Updater<R, W> {
    pub fn new() -> Self {
        Updater {
            input_buffer: None,
            output_buffer: None,
            index: Index::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), InitError> {
        let Buffers {
            input_buffer,
            output_buffer: _,
        } = self.get_buffers();
        let input_buffer = input_buffer?;
        let mut interface = Interface::new(input_buffer);

        let mut key_value_pairs: Vec<KeyValuePair> = Vec::new();
        loop {
            let start = interface.stream_position()?;
            let size = match interface.get_size() {
                // Exit loop if EOF
                Err(DecodeVarintError::Io(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                result => result,
            }?;
            let next_pos = interface.stream_position()? + size;

            let id = interface.get_id()?;
            let server = interface.get_server()?;

            // Pushing to temp vec as self.index cannot be accessed
            key_value_pairs.push(KeyValuePair {
                key: Key { server, id },
                value: Value {
                    start,
                    size: next_pos - start,
                },
            });

            interface.seek(std::io::SeekFrom::Start(next_pos))?;
        }
        for KeyValuePair { key, value } in key_value_pairs {
            self.index.add(key, value);
        }
        Ok(())
    }

    pub fn update(&mut self, document: Document) -> Result<(), UpdateError> {
        let key = Key {
            server: document.server.clone(),
            id: document.id,
        };
        if self.index.use_value(key.clone()) {
            return Ok(());
        }
        let value = self.index.get(&key);

        let Buffers {
            input_buffer,
            output_buffer,
        } = self.get_buffers();
        let output_buffer = output_buffer?;

        if let Some(Value { start, size: _ }) = value {
            let input_buffer = input_buffer?;
            input_buffer.seek(std::io::SeekFrom::Start(start))?;

            let block = Block::new(input_buffer);
            block.update(output_buffer, document)
        } else {
            let mut input_buffer = Block::<Cursor<Vec<u8>>>::new_buffer()?;
            let block = Block::new(&mut input_buffer);
            block.update(output_buffer, document)
        }?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<(), FinalizeError> {
        // Sorting positions to reduce seek movement
        let mut values: Vec<Value> = self.index.iter_unused().collect();
        values.sort_by(|Value { start: a, size: _ }, Value { start: b, size: _ }| a.cmp(b));

        let Buffers {
            input_buffer,
            output_buffer,
        } = self.get_buffers();
        let mut input_buffer = input_buffer?;
        let output_buffer = output_buffer?;

        for Value { start, size } in values {
            input_buffer.seek(std::io::SeekFrom::Start(start))?;
            let mut take = input_buffer.take(size);
            std::io::copy(&mut take, output_buffer)?;
            input_buffer = take.into_inner();
        }
        Ok(())
    }

    pub fn set_input_buffer(&mut self, input_buffer: R) {
        self.input_buffer = Some(input_buffer);
    }

    pub fn set_output_buffer(&mut self, output_buffer: W) {
        self.output_buffer = Some(output_buffer);
    }

    fn get_buffers(&mut self) -> Buffers<'_, R, W> {
        let input_buffer = self.input_buffer.as_mut().ok_or(GetBufferError::NotFound);
        let output_buffer = self.output_buffer.as_mut().ok_or(GetBufferError::NotFound);
        Buffers {
            input_buffer,
            output_buffer,
        }
    }
}
