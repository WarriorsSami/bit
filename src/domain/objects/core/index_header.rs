use std::io::Write;
use anyhow::anyhow;
use byteorder::{ByteOrder, WriteBytesExt};
use bytes::Bytes;
use derive_new::new;
use crate::domain::objects::{HEADER_SIZE, SIGNATURE, VERSION};
use crate::domain::objects::object::{Packable, Unpackable};

#[derive(Debug, Clone, new)]
pub struct IndexHeader {
    pub(crate) marker: String,
    pub(crate) version: u32,
    pub(crate) entries_count: u32,
}

impl IndexHeader {
    pub(crate) fn empty() -> Self {
        IndexHeader {
            marker: String::from(SIGNATURE),
            version: VERSION,
            entries_count: 0,
        }
    }
}

impl Packable for IndexHeader {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        // pack!(self.marker, self.version, self.entries_count => "a4N2")
        let mut bytes = Vec::new();
        bytes.write_all(self.marker.as_bytes())?;
        bytes.write_u32::<byteorder::NetworkEndian>(self.version)?;
        bytes.write_u32::<byteorder::NetworkEndian>(self.entries_count)?;

        Ok(Bytes::from(bytes))
    }
}

impl Unpackable for IndexHeader {
    fn deserialize(bytes: Bytes) -> anyhow::Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(anyhow!("Invalid header size"));
        }

        let marker = String::from_utf8(bytes[0..4].to_vec())
            .map_err(|_| anyhow!("Invalid marker in index header"))?;
        let version = byteorder::NetworkEndian::read_u32(&bytes[4..8]);
        let entries_count = byteorder::NetworkEndian::read_u32(&bytes[8..12]);

        Ok(IndexHeader {
            marker,
            version,
            entries_count,
        })
    }
}
