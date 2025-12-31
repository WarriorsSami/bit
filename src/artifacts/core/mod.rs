use derive_new::new;
use minus::Pager;
use std::io::{self, Write};

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
