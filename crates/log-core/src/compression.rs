//! Gzip decoding is streamed to bounded local scratch storage before parsing.
use crate::EngineError;
use flate2::read::MultiGzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_DECODED_BYTES: u64 = 10 * 1024 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionInfo {
    pub format: String,
    pub compressed_bytes: u64,
    pub compressed_sha256: String,
}

pub(crate) fn digest(path: &Path, cancelled: &AtomicBool) -> Result<String, EngineError> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 256 * 1024];
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(EngineError::Cancelled);
        }
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn decode(
    path: &Path,
    cancelled: &AtomicBool,
    limit: u64,
) -> Result<Option<(tempfile::NamedTempFile, CompressionInfo)>, EngineError> {
    let mut input = BufReader::new(File::open(path)?);
    if !input.get_ref().metadata()?.is_file() {
        return Err(EngineError::NotRegularFile);
    }
    if !input.fill_buf()?.starts_with(b"\x1f\x8b") {
        return Ok(None);
    }
    let before = input.get_ref().metadata()?;
    let original_hash = digest(path, cancelled)?;
    let mut decoder = MultiGzDecoder::new(input);
    let mut output = tempfile::NamedTempFile::new()?;
    let mut buffer = [0; 256 * 1024];
    let mut bytes = 0u64;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(EngineError::Cancelled);
        }
        let n = decoder
            .read(&mut buffer)
            .map_err(|_| EngineError::InvalidCompression)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > limit {
            return Err(EngineError::DecompressionLimit);
        }
        output.write_all(&buffer[..n])?;
    }
    output.flush()?;
    let after = std::fs::metadata(path)?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || digest(path, cancelled)? != original_hash
    {
        return Err(EngineError::SourceChanged);
    }
    Ok(Some((
        output,
        CompressionInfo {
            format: "gzip".into(),
            compressed_bytes: before.len(),
            compressed_sha256: original_hash,
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_limit_accepts_exact_bound_and_rejects_next_byte() {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(b"12345").unwrap();
        let mut input = tempfile::NamedTempFile::new().unwrap();
        input.write_all(&encoder.finish().unwrap()).unwrap();
        assert!(decode(input.path(), &AtomicBool::new(false), 5)
            .unwrap()
            .is_some());
        assert!(matches!(
            decode(input.path(), &AtomicBool::new(false), 4),
            Err(EngineError::DecompressionLimit)
        ));
    }
}
