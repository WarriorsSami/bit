//! Core utilities and shared types
//!
//! This module contains shared utilities used across the application.

use derive_new::new;
use minus::Pager;
use std::io::{self, Write};

/// Wrapper that implements `Write` for the minus pager
///
/// The minus pager doesn't implement `std::io::Write` directly, so this wrapper
/// adapts it to be compatible with Rust's standard I/O traits. This allows
/// using the pager as a drop-in replacement for stdout in commands that produce
/// long output.
///
/// ## Usage
///
/// ```ignore
/// let pager = Pager::new();
/// let mut writer = PagerWriter::new(pager.clone());
/// writeln!(writer, "Some long output...")?;
/// page_all(pager)?;
/// ```
#[derive(new)]
pub struct PagerWriter {
    pager: Pager,
}

impl PagerWriter {
    pub fn pager(&self) -> &Pager {
        &self.pager
    }
}

impl Write for PagerWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.pager.push_str(s).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
