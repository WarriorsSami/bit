use std::io::{Read, Write};
use std::ops::DerefMut;
use anyhow::anyhow;
use bytes::Bytes;
use file_guard::FileGuard;
use sha1::{Digest, Sha1};
use crate::domain::objects::CHECKSUM_SIZE;

#[derive(Debug)]
pub struct Checksum<'f> {
    file: FileGuard<&'f mut std::fs::File>,
    digest: Sha1,
}

impl<'f> Checksum<'f> {
    pub(crate) fn new(file: FileGuard<&'f mut std::fs::File>) -> Self {
        Checksum {
            file,
            digest: Sha1::new(),
        }
    }

    pub(crate) fn read(&mut self, size: usize) -> anyhow::Result<Bytes> {
        let mut buffer = vec![0; size];
        self.file
            .deref_mut()
            .read_exact(&mut buffer)
            .map_err(|_| anyhow!("Unexpected end-of-file while reading index"))?;

        self.digest.update(&buffer);
        Ok(Bytes::from(buffer))
    }

    pub(crate) fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.file.deref_mut().write_all(data)?;
        self.digest.update(data);
        Ok(())
    }

    pub(crate) fn write_checksum(&mut self) -> anyhow::Result<()> {
        let checksum = self.digest.clone().finalize();
        self.file
            .deref_mut()
            .write_all(checksum.as_slice())
            .map_err(|_| anyhow!("Failed to write checksum to index file"))?;

        Ok(())
    }

    pub(crate) fn verify(&mut self) -> anyhow::Result<()> {
        let mut expected_checksum = [0u8; CHECKSUM_SIZE];
        self.file.deref_mut().read_exact(&mut expected_checksum)?;

        let actual_checksum = self.digest.clone().finalize();
        let actual_checksum = actual_checksum.as_slice();

        if expected_checksum != actual_checksum {
            return Err(anyhow!("Checksum does not match value stored on disk"));
        }

        Ok(())
    }
}