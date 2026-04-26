pub mod rle {
    //! Run-length encoding
    //!
    //! Format:
    //! ```text
    //! varint := LEB128 unsigned integer
    //! run := [value][count: varint]
    //! runs := run*
    //! size: Number of bytes to skip to skip the entire structure
    //! offset: Number of bytes to skip to reach the last run
    //! ```
    //! Layout:
    //! `[size: varint][offset: u32 BE][runs]`

    use super::super::{
        primitives::{DecodeVarintError, decode_varint_u64, encode_varint_u64},
        utils::add_size_header,
    };
    use std::{io::Read, num::TryFromIntError};

    #[allow(dead_code)]
    #[derive(Debug)]
    pub enum Error {
        Varint(DecodeVarintError),
        Io(std::io::Error),
        OutOfRange(TryFromIntError),
        ReadData(()),
    }

    impl From<DecodeVarintError> for Error {
        fn from(value: DecodeVarintError) -> Self {
            Error::Varint(value)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(value: std::io::Error) -> Self {
            Error::Io(value)
        }
    }

    impl From<TryFromIntError> for Error {
        fn from(value: TryFromIntError) -> Self {
            Error::OutOfRange(value)
        }
    }

    impl From<()> for Error {
        fn from((): ()) -> Self {
            Error::ReadData(())
        }
    }

    pub fn update<R: Read, F>(reader: &mut R, data: &[u8], read_fn: F) -> Result<Vec<u8>, Error>
    where
        F: Fn(&mut R) -> Result<Vec<u8>, ()>,
    {
        // Size
        let size = decode_varint_u64(reader)?;
        // Offset
        let mut offset_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut offset_bytes)?;
        let offset = u32::from_be_bytes(offset_bytes);
        // Runs (excluding last)
        let mut field_data: Vec<u8> = vec![0; offset as usize];
        reader.read_exact(&mut field_data)?;

        let read_size = u32::try_from(size)? - 4 - offset;
        if read_size > 0 {
            // Last run
            let prev_data = read_fn(reader)?;
            let count = decode_varint_u64(reader)?;

            if prev_data == data {
                // Same value, increment count
                Ok(add_size_header([
                    &offset_bytes,
                    &field_data,
                    &prev_data,
                    &encode_varint_u64(count + 1),
                ]))
            } else {
                // Different value, add item
                let new_offset_bytes = u32::to_be_bytes(offset + read_size);
                Ok(add_size_header([
                    &new_offset_bytes,
                    &field_data,
                    &prev_data,
                    &encode_varint_u64(count),
                    data,
                    &encode_varint_u64(1),
                ]))
            }
        } else {
            // First item, so simply add item
            Ok(add_size_header([
                &offset_bytes,
                &field_data,
                data,
                &encode_varint_u64(1),
            ]))
        }
    }

    pub fn empty() -> Vec<u8> {
        add_size_header([&u32::to_be_bytes(0)])
    }

    #[cfg(test)]
    mod test {
        use super::*;

        mod update {
            use std::io::{Cursor, Read};

            use super::*;

            fn read_byte(reader: &mut impl Read) -> Result<Vec<u8>, ()> {
                let mut buffer = [0];
                reader.read_exact(&mut buffer).map_err(|_| ())?;
                Ok(buffer.to_vec())
            }

            #[test]
            fn update_empty() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    4, // Size: 4 (varint)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                ]);
                let result = update(&mut cursor, &[64], read_byte);

                let expected: Vec<u8> = vec![
                    6, // Size: 6 (varint)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                    // Run 1
                    0x40, // Value
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_same_value() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    8, // Size: 8 (varint)
                    0x00, 0x00, 0x00, 0x02, // Offset: 2 (u32 BE)
                    // Run 1
                    123, // Value
                    36,  // Count: 36 (varint)
                    // Run 2
                    87,  // Value
                    127, // Count: 127 (varint)
                ]);
                let result = update(&mut cursor, &[87], read_byte);

                let expected: Vec<u8> = vec![
                    9, // Size: 9 (varint)
                    0x00, 0x00, 0x00, 0x02, // Offset: 2 (u32 BE)
                    // Run 1
                    123, // Value
                    36,  // Count: 36 (varint)
                    // Run 2
                    87, // Value
                    128, 1, // Count: 128 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_different_value() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    9, // Size: 9 (varint)
                    0x00, 0x00, 0x00, 0x02, // Offset: 2 (u32 BE)
                    // Run 1
                    93, // Value
                    0x6c, 0xb7, 0xae, 0x0a, // Count (varint)
                ]);
                let result = update(&mut cursor, &[223], read_byte);

                let expected: Vec<u8> = vec![
                    11, // Size: 11 (varint)
                    0x00, 0x00, 0x00, 0x05, // Offset: 5 (u32 BE)
                    // Run 1
                    93, // Value
                    0x6c, 0xb7, 0xae, 0x0a, // Count (varint)
                    // Run 2
                    223, // Value
                    1,   // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }
        }

        mod empty {
            use super::*;

            #[test]
            fn create_empty_structure() {
                let expected = vec![
                    4, // Size: 4 (varint)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                ];
                let result = empty();
                assert_eq!(result, expected);
            }
        }
    }
}

pub mod delta {
    //! Delta encoding for integers
    //!
    //! `Option<i32>` encoding:
    //! - None -> 0
    //! - Some(x) -> zigzag(x) + 1
    //! - Uses LEB128 to convert value to varint
    //!
    //! Format:
    //! ```text
    //! varint := LEB128 unsigned integer
    //! delta := Difference in value (current - prev) as Option<i32> varint, prev is the last
    //! non-None value defaulting to 0
    //! deltas := delta*
    //! size: Number of bytes to skip to skip the entire structure
    //! value: Last non-None value, defaults to 0
    //! ```
    //! Layout:
    //! `[size: varint][value: i32 BE][deltas]`

    use super::super::{
        primitives::{DecodeVarintError, decode_varint_u64, encode_varint_optional_i64},
        utils::add_size_header,
    };
    use std::io::Read;
    use std::num::TryFromIntError;

    #[allow(dead_code)]
    #[derive(Debug)]
    pub enum Error {
        Varint(DecodeVarintError),
        Io(std::io::Error),
        OutOfRange(TryFromIntError),
    }

    impl From<DecodeVarintError> for Error {
        fn from(value: DecodeVarintError) -> Self {
            Error::Varint(value)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(value: std::io::Error) -> Self {
            Error::Io(value)
        }
    }

    impl From<TryFromIntError> for Error {
        fn from(value: TryFromIntError) -> Self {
            Error::OutOfRange(value)
        }
    }

    pub fn update(reader: &mut impl Read, data: Option<i32>) -> Result<Vec<u8>, Error> {
        // Size
        let size = decode_varint_u64(reader)?;
        // Value
        let mut value_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut value_bytes)?;
        let prev_value = i32::from_be_bytes(value_bytes);
        // Deltas
        let mut deltas = vec![0; usize::try_from(size)? - 4];
        reader.read_exact(&mut deltas)?;

        match data {
            None => Ok(add_size_header([
                &value_bytes,
                &deltas,
                &encode_varint_optional_i64(None),
            ])),
            Some(x) => Ok(add_size_header([
                &x.to_be_bytes(),
                &deltas,
                &encode_varint_optional_i64(Some(i64::from(x - prev_value))),
            ])),
        }
    }

    pub fn empty() -> Vec<u8> {
        add_size_header([&u32::to_be_bytes(0)])
    }

    #[cfg(test)]
    mod test {
        use super::*;

        mod update {
            use std::io::Cursor;

            use super::*;

            #[test]
            fn update_empty() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    4, // Size: 4 (varint)
                    0x00, 0x00, 0x00, 0x00, // Value: 0 (i32 BE)
                ]);
                let result = update(&mut cursor, Some(1_234_567));

                let expected: Vec<u8> = vec![
                    8, // Size: 8 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_none() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    8, // Size: 8 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                ]);
                let result = update(&mut cursor, None);

                let expected: Vec<u8> = vec![
                    9, // Size: 9 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                    0x00, // Delta: None (Option<i32> varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_positive_delta() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    8, // Size: 8 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                ]);

                let result = update(&mut cursor, Some(2_000_000));

                let expected: Vec<u8> = vec![
                    11, // Size: 11 (varint)
                    0x00, 0x1e, 0x84, 0x80, // Value: 2_000_000 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                    0xf3, 0xb7, 0x5d, // Delta: 735433 (Option<i32> varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_negative_delta() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    8, // Size: 8 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                ]);

                let result = update(&mut cursor, Some(1_000_000));

                let expected: Vec<u8> = vec![
                    11, // Size: 11 (varint)
                    0x00, 0x0f, 0x42, 0x40, // Value: 1_000_000 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                    0x8e, 0xd1, 0x1c, // Delta: -234_567 (Option<i32> varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_from_none() {
                let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![
                    9, // Size: 9 (varint)
                    0x00, 0x12, 0xd6, 0x87, // Value: 1_234_567 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                    0x00, // Delta: None (Option<i32> varint)
                ]);

                let result = update(&mut cursor, Some(2_000_000));

                let expected: Vec<u8> = vec![
                    12, // Size: 12 (varint)
                    0x00, 0x1e, 0x84, 0x80, // Value: 2_000_000 (i32 BE)
                    0x8f, 0xda, 0x96, 0x01, // Delta: 1_234_567 (Option<i32> varint)
                    0x00, // Delta: None (Option<i32> varint)
                    0xf3, 0xb7, 0x5d, // Delta: 735433 (Option<i32> varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }
        }

        mod empty {
            use super::*;

            #[test]
            fn create_empty_structure() {
                let expected = vec![
                    4, // Size: 4 (varint)
                    0x00, 0x00, 0x00, 0x00, // Value: 0 (i32 BE)
                ];
                let result = empty();
                assert_eq!(result, expected);
            }
        }
    }
}

pub mod delta_rle {
    //! Run-length encoding of deltas for integers
    //!
    //! `Option<i64>` encoding:
    //! - None -> 0
    //! - Some(x) -> zigzag(x) + 1
    //! - Uses LEB128 to convert value to varint
    //!
    //! Format:
    //! ```text
    //! varint := LEB128 unsigned integer
    //! delta := Difference in value (current - prev) as Option<i64> varint, prev is the last non-None value defaulting to 0
    //! run := [delta][count: varint]
    //! runs := run*
    //! size: Number of bytes to skip to skip the entire structure
    //! value: Last non-None value, defaults to 0
    //! offset: Number of bytes to skip to reach the last run
    //! ```
    //! Layout:
    //! `[size: varint][value: i64 BE][offset: u32 BE][runs]`

    use std::{io::Read, num::TryFromIntError};

    use super::super::{
        primitives::{
            DecodeVarintError, decode_varint_u64, encode_varint_optional_i64, encode_varint_u64,
            read_varint_bytes,
        },
        utils::add_size_header,
    };

    #[allow(dead_code)]
    #[derive(Debug)]
    pub enum Error {
        Varint(DecodeVarintError),
        Io(std::io::Error),
        OutOfRange(TryFromIntError),
    }

    impl From<DecodeVarintError> for Error {
        fn from(value: DecodeVarintError) -> Self {
            Error::Varint(value)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(value: std::io::Error) -> Self {
            Error::Io(value)
        }
    }

    impl From<TryFromIntError> for Error {
        fn from(value: TryFromIntError) -> Self {
            Error::OutOfRange(value)
        }
    }

    pub fn update(reader: &mut impl Read, data: Option<i64>) -> Result<Vec<u8>, Error> {
        // Size
        let size = decode_varint_u64(reader)?;
        // Value
        let mut value_bytes: [u8; 8] = [0; 8];
        reader.read_exact(&mut value_bytes)?;
        let acc_value = i64::from_be_bytes(value_bytes);
        // Offset
        let mut offset_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut offset_bytes)?;
        let offset = u32::from_be_bytes(offset_bytes);
        // Runs (excluding last)
        let mut runs: Vec<u8> = vec![0; offset as usize];
        reader.read_exact(&mut runs)?;

        let delta: Vec<u8>;
        let new_value: [u8; 8];
        match data {
            None => {
                delta = encode_varint_optional_i64(None);
                new_value = value_bytes; // Keep old value
            }
            Some(x) => {
                delta = encode_varint_optional_i64(Some(x - acc_value));
                new_value = x.to_be_bytes(); // Update value
            }
        }
        if size > 12 {
            // Last run
            let prev_delta = read_varint_bytes(reader)?;
            let count = decode_varint_u64(reader)?;

            if prev_delta == delta {
                Ok(add_size_header([
                    &new_value,
                    &offset_bytes,
                    &runs,
                    &prev_delta,
                    &encode_varint_u64(count + 1),
                ]))
            } else {
                let count_bytes = encode_varint_u64(count);
                let new_offset = offset + u32::try_from(prev_delta.len() + count_bytes.len())?;
                Ok(add_size_header([
                    &new_value,
                    &new_offset.to_be_bytes(),
                    &runs,
                    &prev_delta,
                    &count_bytes,
                    &delta,
                    &encode_varint_u64(1),
                ]))
            }
        } else {
            Ok(add_size_header([
                &new_value,
                &offset_bytes,
                &runs,
                &delta,
                &encode_varint_u64(1),
            ]))
        }
    }

    pub fn empty() -> Vec<u8> {
        add_size_header([&i64::to_be_bytes(0), &u32::to_be_bytes(0)])
    }

    #[cfg(test)]
    mod test {
        use super::*;

        mod update {
            use std::io::Cursor;

            use super::*;

            #[test]
            fn update_empty() {
                let mut cursor = Cursor::new(vec![
                    12, // Size: 12 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Value: 0 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                ]);
                let result = update(&mut cursor, Some(1500));

                let expected: Vec<u8> = vec![
                    15, // Size: 15 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_none() {
                let mut cursor = Cursor::new(vec![
                    15, // Size: 15 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ]);
                let result = update(&mut cursor, None);

                let expected: Vec<u8> = vec![
                    17, // Size: 17 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x03, // Offset: 3 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0x00, // Delta: None (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_positive_delta() {
                let mut cursor = Cursor::new(vec![
                    15, // Size: 15 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ]);
                let result = update(&mut cursor, Some(6296));

                let expected: Vec<u8> = vec![
                    18, // Size: 18 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x98, // Value: 6296 (i64 BE)
                    0x00, 0x00, 0x00, 0x03, // Offset: 3 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0xf9, 0x4a, // Delta: 4796 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_negative_delta() {
                let mut cursor = Cursor::new(vec![
                    15, // Size: 15 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ]);
                let result = update(&mut cursor, Some(834));

                let expected: Vec<u8> = vec![
                    18, // Size: 15 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x42, // Value: 834 (i64 BE)
                    0x00, 0x00, 0x00, 0x03, // Offset: 3 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0xb4, 0x0a, // Delta: -666 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_from_none() {
                let mut cursor = Cursor::new(vec![
                    17, // Size: 17 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xdc, // Value: 1500 (i64 BE)
                    0x00, 0x00, 0x00, 0x03, // Offset: 3 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0x00, // Delta: None (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ]);
                let result = update(&mut cursor, Some(2637));

                let expected: Vec<u8> = vec![
                    20, // Size: 20 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x4d, // Value: 2637 (i64 BE)
                    0x00, 0x00, 0x00, 0x05, // Offset: 5 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0x00, // Delta: None (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 3
                    0xe3, 0x11, // Delta: 1137 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }

            #[test]
            fn update_with_same_delta() {
                let mut cursor = Cursor::new(vec![
                    20, // Size: 20 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7e, 0x69, // Value: 32361 (i64 BE)
                    0x00, 0x00, 0x00, 0x05, // Offset: 5 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0x00, // Delta: None (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 3
                    0xe7, 0x03, // Delta: 243 (Option<i64> varint)
                    0x7f, // Count: 127 (varint)
                ]);
                let result = update(&mut cursor, Some(32604));

                let expected: Vec<u8> = vec![
                    21, // Size: 20 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7f, 0x5c, // Value: 32604 (i64 BE)
                    0x00, 0x00, 0x00, 0x05, // Offset: 5 (u32 BE)
                    // Run 1
                    0xb9, 0x17, // Delta: 1500 (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 2
                    0x00, // Delta: None (Option<i64> varint)
                    0x01, // Count: 1 (varint)
                    // Run 3
                    0xe7, 0x03, // Delta: 243 (Option<i64> varint)
                    0x80, 0x01, // Count: 128 (varint)
                ];
                assert_eq!(result.unwrap(), expected);
            }
        }

        mod empty {
            use super::*;

            #[test]
            fn create_empty_structure() {
                let expected = vec![
                    12, // Size: 12 (varint)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Value: 0 (i64 BE)
                    0x00, 0x00, 0x00, 0x00, // Offset: 0 (u32 BE)
                ];
                let result = empty();
                assert_eq!(result, expected);
            }
        }
    }
}
