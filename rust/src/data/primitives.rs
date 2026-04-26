use std::{
    io::{Read, Seek, SeekFrom},
    num::TryFromIntError,
    string::FromUtf8Error,
};

#[allow(clippy::cast_sign_loss)] // Allow since this is intentional
fn encode_zigzag(value: i64) -> u64 {
    ((value >> 63) ^ (value << 1)) as u64
}

pub fn encode_varint_u64(mut value: u64) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();

    while value >= 0x80 {
        #[allow(clippy::cast_possible_truncation)] // Allow truncation as & 0x7f would do the same
        bytes.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    #[allow(clippy::cast_possible_truncation)] // value is guaranteed to be less than 0x80
    bytes.push(value as u8);
    bytes
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum DecodeVarintError {
    Io(std::io::Error),
    Size(String),
}

impl From<std::io::Error> for DecodeVarintError {
    fn from(value: std::io::Error) -> Self {
        DecodeVarintError::Io(value)
    }
}
impl From<String> for DecodeVarintError {
    fn from(value: String) -> Self {
        DecodeVarintError::Size(value)
    }
}

pub fn decode_varint_u64(reader: &mut impl Read) -> Result<u64, DecodeVarintError> {
    let mut value: u64 = 0;

    for shift in (0..=63).step_by(7) {
        let mut buffer: [u8; 1] = [0];
        reader.read_exact(&mut buffer)?;
        let byte = buffer[0];

        let add_value = (u128::from(byte & 0x7f)) << shift;
        value |= match u64::try_from(add_value) {
            Ok(x) => x,
            Err(_) => break,
        };
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("Varint is too large for u64".to_string())?
}

pub fn read_varint_bytes(reader: &mut impl Read) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let mut buffer: [u8; 1] = [0];
        reader.read_exact(&mut buffer)?;
        let byte = buffer[0];
        bytes.push(byte);
        if byte & 0x80 == 0 {
            return Ok(bytes);
        }
    }
}

fn encode_varint_i64(value: i64) -> Vec<u8> {
    encode_varint_u64(encode_zigzag(value))
}

pub fn encode_varint_optional_i64(value: Option<i64>) -> Vec<u8> {
    encode_varint_u64(match value {
        None => 0,
        Some(x) => encode_zigzag(x) + 1,
    })
}

pub fn encode_optional_string(value: Option<String>) -> Vec<u8> {
    match value {
        None => encode_varint_u64(0),
        Some(string) => {
            // Encode length
            let mut bytes = encode_varint_u64(string.len() as u64 + 1_u64);
            bytes.append(&mut string.into_bytes()); // Append string content
            bytes
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum DecodeStringError {
    Varint(DecodeVarintError),
    OutOfRange(TryFromIntError),
    Io(std::io::Error),
    Utf8(FromUtf8Error),
}

impl From<DecodeVarintError> for DecodeStringError {
    fn from(value: DecodeVarintError) -> Self {
        DecodeStringError::Varint(value)
    }
}

impl From<TryFromIntError> for DecodeStringError {
    fn from(value: TryFromIntError) -> Self {
        DecodeStringError::OutOfRange(value)
    }
}

impl From<std::io::Error> for DecodeStringError {
    fn from(value: std::io::Error) -> Self {
        DecodeStringError::Io(value)
    }
}

impl From<FromUtf8Error> for DecodeStringError {
    fn from(value: FromUtf8Error) -> Self {
        DecodeStringError::Utf8(value)
    }
}

pub fn decode_optional_string(reader: &mut impl Read) -> Result<Option<String>, DecodeStringError> {
    match decode_varint_u64(reader)? {
        0 => Ok(None),
        1 => Ok(Some(String::new())),
        value => {
            // Read number of bytes equal to the length given (value - 1)
            let mut buffer = vec![0; usize::try_from(value)? - 1];
            reader.read_exact(&mut buffer)?;

            Ok(Some(String::from_utf8(buffer)?))
        }
    }
}

pub fn encode_optional_varint_array(array: Option<Vec<i64>>) -> Vec<u8> {
    match array {
        None => encode_varint_u64(0),
        Some(arr) => {
            let mut data_bytes: Vec<u8> = arr
                .iter()
                .flat_map(|value| encode_varint_i64(*value))
                .collect();
            let mut bytes = encode_varint_u64(data_bytes.len() as u64 + 1);
            bytes.append(&mut data_bytes);
            bytes
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ReadBytesError {
    Varint(DecodeVarintError),
    Io(std::io::Error),
    OutOfRange(TryFromIntError),
}

impl From<DecodeVarintError> for ReadBytesError {
    fn from(value: DecodeVarintError) -> Self {
        ReadBytesError::Varint(value)
    }
}

impl From<std::io::Error> for ReadBytesError {
    fn from(value: std::io::Error) -> Self {
        ReadBytesError::Io(value)
    }
}

impl From<TryFromIntError> for ReadBytesError {
    fn from(value: TryFromIntError) -> Self {
        ReadBytesError::OutOfRange(value)
    }
}

pub fn read_bytes(
    reader: &mut (impl Read + Seek),
    size_offset: u64,
) -> Result<Vec<u8>, ReadBytesError> {
    let start = reader.stream_position()?;
    let header_value = decode_varint_u64(reader)?;
    let header_size = reader.stream_position()? - start;
    reader.seek(SeekFrom::Start(start))?;

    let mut buffer: Vec<u8> = vec![
        0;
        usize::try_from(if header_value <= size_offset {
            header_size
        } else {
            header_size + header_value - size_offset
        })?
    ];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod test {
    use super::*;

    mod encode_zigzag {
        use super::*;

        #[test]
        fn handle_positive_number() {
            let cases: [(i64, u64); 3] = [(0, 0), (46, 92), (1474, 2948)];
            for (value, expected) in cases {
                let result = encode_zigzag(value);
                assert_eq!(result, expected);
            }
        }

        #[test]
        fn handle_negative_number() {
            let cases: [(i64, u64); 3] = [(-5, 9), (-139, 277), (-1849, 3697)];
            for (value, expected) in cases {
                let result = encode_zigzag(value);
                assert_eq!(result, expected);
            }
        }
    }

    mod encode_varint_u64 {
        use super::*;

        #[test]
        fn correct_result() {
            let cases: [(u64, Vec<u8>); 4] = [
                (0, vec![0]),
                (1, vec![1]),
                (128, vec![128, 1]),
                (4529, vec![177, 35]),
            ];
            for (value, expected) in cases {
                let result = encode_varint_u64(value);
                assert_eq!(result, expected);
            }
        }
    }

    mod decode_varint_u64 {
        use std::io::Cursor;

        use super::*;

        #[test]
        fn decode_valid_varint() {
            let cases: [(Vec<u8>, u64); 4] = [
                (vec![0], 0),
                (vec![43], 43),
                (vec![152, 32], 4120),
                (
                    vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 1],
                    u64::try_from(2u128.pow(64) - 1).unwrap(),
                ),
            ];
            for (vec, expected) in cases {
                let mut cursor = Cursor::new(vec);
                let result = decode_varint_u64(&mut cursor);
                assert_eq!(result.unwrap(), expected);
            }
        }

        #[test]
        fn return_error_if_too_large() {
            // 2 ** 64
            let mut cursor: Cursor<Vec<u8>> =
                Cursor::new(vec![128, 128, 128, 128, 128, 128, 128, 128, 128, 2]);
            let result = decode_varint_u64(&mut cursor);
            result.expect_err("should fail when value cannot fit in u64");
        }
    }

    mod read_varint_bytes {
        use std::io::Cursor;

        use super::*;

        #[test]
        fn correct_result() {
            let cases: [(Vec<u8>, Vec<u8>); _] = [
                (vec![20], vec![20]),
                (vec![143, 166, 35], vec![143, 166, 35]),
                (vec![156, 14, 193, 52], vec![156, 14]),
            ];
            for (vec, expected) in cases {
                let mut cursor = Cursor::new(vec);
                let result = read_varint_bytes(&mut cursor);
                assert_eq!(result.unwrap(), expected);
            }
        }
    }

    mod encode_varint_i64 {
        use super::*;

        #[test]
        fn correct_result() {
            let cases: [(i64, Vec<u8>); 4] = [
                (0, vec![0]),
                (62, vec![124]),
                (4733, vec![250, 73]),
                (-643, vec![133, 10]),
            ];
            for (value, expected) in cases {
                let result = encode_varint_i64(value);
                assert_eq!(result, expected);
            }
        }
    }

    mod encode_varint_optional_i64 {
        use super::*;

        #[test]
        fn encode_number() {
            let cases: [(Option<i64>, Vec<u8>); 2] =
                [(Some(376), vec![241, 5]), (Some(-652), vec![152, 10])];
            for (value, expected) in cases {
                let result = encode_varint_optional_i64(value);
                assert_eq!(result, expected);
            }
        }

        #[test]
        fn encode_none() {
            let result = encode_varint_optional_i64(None);
            assert_eq!(result, vec![0]);
        }
    }

    mod encode_optional_string {
        use super::*;

        #[test]
        fn encode_string() {
            let cases: [(String, Vec<u8>); 2] = [
                (String::new(), vec![1]),
                ("hello".to_string(), vec![6, 104, 101, 108, 108, 111]),
            ];
            for (value, expected) in cases {
                let result = encode_optional_string(Some(value));
                assert_eq!(result, expected);
            }
        }

        #[test]
        fn encode_none() {
            let result = encode_optional_string(None);
            assert_eq!(result, vec![0]);
        }
    }

    mod decode_optional_string {
        use std::io::Cursor;

        use super::*;

        #[test]
        fn decode_valid_bytes() {
            let cases: [(Vec<u8>, Option<String>); 3] = [
                (vec![4, 120, 121, 122], Some("xyz".to_string())),
                (vec![1], Some(String::new())),
                (vec![0], None),
            ];
            for (bytes, expected) in cases {
                let mut cursor = Cursor::new(bytes);
                let result = decode_optional_string(&mut cursor);
                assert_eq!(result.unwrap(), expected);
            }
        }

        #[test]
        fn return_error_on_bad_length() {
            let mut cursor = Cursor::new(vec![146, 218, 193]);
            let result = decode_optional_string(&mut cursor);
            assert!(matches!(result, Err(DecodeStringError::Varint(_))));
        }

        #[test]
        fn return_error_on_bad_string() {
            // 181 (0b1011011) cannot be the first byte of a character in utf-8
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![3, 181, 101]);
            let result = decode_optional_string(&mut cursor);
            assert!(matches!(result, Err(DecodeStringError::Utf8(_))));
        }

        #[test]
        fn return_error_if_too_short() {
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![5, 97, 98, 99]);
            let result = decode_optional_string(&mut cursor);
            assert!(matches!(result, Err(DecodeStringError::Io(_))));
        }
    }

    mod encode_optional_varint_array {
        use super::*;

        #[test]
        fn encode_array() {
            let cases: [(Vec<i64>, Vec<u8>); 2] = [
                (vec![20, -185, -2417], vec![6, 40, 241, 2, 225, 37]),
                (vec![], vec![1]),
            ];
            for (array, expected) in cases {
                let result = encode_optional_varint_array(Some(array));
                assert_eq!(result, expected);
            }
        }

        #[test]
        fn encode_none() {
            let result = encode_optional_varint_array(None);
            assert_eq!(result, vec![0]);
        }
    }

    mod read_bytes {
        use std::io::Cursor;

        use super::*;

        #[test]
        fn read_valid_bytes() {
            let cases: [(Vec<u8>, u64, Vec<u8>); 2] = [
                (vec![3, 53, 104, 64, 216], 0, vec![3, 53, 104, 64]),
                (vec![4, 204, 58, 15, 177], 1, vec![4, 204, 58, 15]),
            ];
            for (bytes, offset, expected) in cases {
                let mut cursor = Cursor::new(bytes);
                let result = read_bytes(&mut cursor, offset);
                assert_eq!(result.unwrap(), expected);
            }
        }

        #[test]
        fn return_error_on_bad_length() {
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![241, 163, 192, 174]);
            let result = read_bytes(&mut cursor, 0);
            assert!(matches!(result, Err(ReadBytesError::Varint(_))));
        }

        #[test]
        fn return_error_if_too_short() {
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![7, 45, 204, 182]);
            let result = read_bytes(&mut cursor, 0);
            assert!(matches!(result, Err(ReadBytesError::Io(_))));
        }
    }
}
