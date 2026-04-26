use super::primitives::encode_varint_u64;

pub fn add_size_header<const N: usize>(chunks: [&[u8]; N]) -> Vec<u8> {
    let len: usize = chunks.iter().map(|bytes| bytes.len()).sum();
    let len_bytes = encode_varint_u64(len as u64);
    let mut vec: Vec<u8> = Vec::with_capacity(len_bytes.len() + len);
    vec.extend(len_bytes);
    for chunk in chunks {
        vec.extend(chunk);
    }
    vec
}

#[cfg(test)]
mod test {
    use super::*;

    mod add_size_header {
        use super::*;

        #[test]
        fn normal_chunks() {
            let chunks: [&[u8]; 3] = [&[45u8, 195u8, 34u8], &[247u8], &[153u8, 83u8]];
            let reuslt = add_size_header(chunks);
            assert_eq!(reuslt, vec![6, 45, 195, 34, 247, 153, 83]);
        }

        #[test]
        fn no_chunks() {
            let result = add_size_header([]);
            assert_eq!(result, vec![0]);
        }

        #[test]
        fn some_empty_chunks() {
            let chunks: [&[u8]; 5] = [&[45u8, 195u8, 34u8], &[], &[], &[153u8, 83u8], &[]];
            let reuslt = add_size_header(chunks);
            assert_eq!(reuslt, vec![5, 45, 195, 34, 153, 83]);
        }

        #[test]
        fn all_empty_chunks() {
            let chunks: [&[u8]; 3] = [&[], &[], &[]];
            let reuslt = add_size_header(chunks);
            assert_eq!(reuslt, vec![0]);
        }
    }
}
