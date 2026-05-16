use std::io::Write;

use zstd::Encoder as ZstdEncoder;

pub struct Encoder<W: Write> {
    zstd_encoder: Option<ZstdEncoder<'static, W>>,
}

impl<W: Write> Encoder<W> {
    pub fn new(writer: W, level: i32) -> std::io::Result<Self> {
        let zstd_encoder = ZstdEncoder::new(writer, level)?;
        Ok(Encoder {
            zstd_encoder: Some(zstd_encoder),
        })
    }
}

impl<W: Write> Drop for Encoder<W> {
    fn drop(&mut self) {
        self.zstd_encoder.take().unwrap().finish().unwrap();
    }
}

impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.zstd_encoder.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.zstd_encoder.as_mut().unwrap().flush()
    }
}
