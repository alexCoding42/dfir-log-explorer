//! Bounded physical-line reading and explicit native-audit framing.
use crate::{security_formats::audit_boundary, EngineError, ParserMode, MAX_LINE_BYTES};
use std::{
    io::{BufRead, Read},
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) fn read_record(
    reader: &mut impl BufRead,
    mode: ParserMode,
    buffer: &mut Vec<u8>,
    pending: &mut Option<Vec<u8>>,
    cancelled: &AtomicBool,
) -> Result<(), EngineError> {
    buffer.clear();
    if let Some(line) = pending.take() {
        *buffer = line;
    } else {
        read_line(reader, buffer)?;
    }
    if mode != ParserMode::Modsecurity {
        return Ok(());
    }
    let first = String::from_utf8_lossy(buffer);
    let Some((id, 'A')) = audit_boundary(first.trim_end_matches(['\r', '\n'])) else {
        return Ok(());
    };
    let id = id.to_owned();
    let mut next = Vec::new();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(EngineError::Cancelled);
        }
        next.clear();
        read_line(reader, &mut next)?;
        if next.is_empty() {
            return Ok(());
        }
        let display = String::from_utf8_lossy(&next);
        let boundary = audit_boundary(display.trim_end_matches(['\r', '\n']));
        if matches!(boundary, Some((_, 'A'))) {
            *pending = Some(next);
            return Ok(());
        }
        if buffer.len() + next.len() > MAX_LINE_BYTES {
            return Err(EngineError::RecordTooLong);
        }
        buffer.extend_from_slice(&next);
        if matches!(boundary, Some((token, 'Z')) if token == id) {
            return Ok(());
        }
    }
}
fn read_line(reader: &mut impl BufRead, buffer: &mut Vec<u8>) -> Result<(), EngineError> {
    let size = reader
        .take((MAX_LINE_BYTES + 1) as u64)
        .read_until(b'\n', buffer)?;
    if size > MAX_LINE_BYTES {
        return Err(EngineError::LineTooLong);
    }
    Ok(())
}
pub(crate) fn physical_lines(bytes: &[u8]) -> u64 {
    bytes.iter().filter(|b| **b == b'\n').count() as u64
        + u64::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct CancelReader<'a> {
        bytes: &'a [u8],
        cancelled: &'a AtomicBool,
    }
    impl Read for CancelReader<'_> {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            unreachable!()
        }
    }
    impl BufRead for CancelReader<'_> {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            let end = self
                .bytes
                .iter()
                .position(|b| *b == b'\n')
                .map_or(self.bytes.len(), |i| i + 1);
            Ok(&self.bytes[..end])
        }
        fn consume(&mut self, count: usize) {
            self.bytes = &self.bytes[count..];
            if self.bytes.starts_with(b"third") {
                self.cancelled.store(true, Ordering::Relaxed);
            }
        }
    }
    #[test]
    fn cancellation_is_checked_inside_an_unfinished_multiline_record() {
        let cancelled = AtomicBool::new(false);
        let mut reader = CancelReader {
            bytes: b"--a-A--\nsecond\nthird\n--a-Z--\n",
            cancelled: &cancelled,
        };
        let result = read_record(
            &mut reader,
            ParserMode::Modsecurity,
            &mut Vec::new(),
            &mut None,
            &cancelled,
        );
        assert!(matches!(result, Err(EngineError::Cancelled)));
        assert!(reader.bytes.starts_with(b"third"));
    }
}
