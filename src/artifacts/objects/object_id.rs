//! Git object identifier (SHA-1 hash)
//!
//! Object IDs are 40-character hexadecimal strings representing SHA-1 hashes.
//! They uniquely identify all objects in Git (blobs, trees, commits).
//!
//! ## Format
//!
//! - Full: 40 hex characters (e.g., "abc123...def")
//! - Short: First 7 characters (e.g., "abc123")
//!
//! ## Storage
//!
//! Objects are stored in `.git/objects/<first-2-chars>/<remaining-38-chars>`

use crate::artifacts::objects::OBJECT_ID_LENGTH;
use std::io;
use std::path::PathBuf;

/// Git object identifier (SHA-1 hash)
///
/// A 40-character hexadecimal string that uniquely identifies an object.
/// Implements various utilities for parsing, serialization, and path conversion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ObjectId(String);

impl ObjectId {
    /// Parse and validate an object ID from a string
    ///
    /// # Arguments
    ///
    /// * `id` - 40-character hexadecimal string
    ///
    /// # Returns
    ///
    /// Validated ObjectId or error if invalid length/characters
    pub fn try_parse(id: String) -> anyhow::Result<Self> {
        if id.len() != OBJECT_ID_LENGTH {
            return Err(anyhow::anyhow!("Invalid object ID length: {}", id.len()));
        }
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow::anyhow!("Invalid object ID characters: {}", id));
        }
        Ok(Self(id.to_string()))
    }

    /// Write the object ID in binary format (20 bytes)
    ///
    /// Converts the 40-char hex string to 20 bytes and writes to the given writer.
    /// Used when serializing tree and commit objects.
    ///
    /// # Arguments
    ///
    /// * `writer` - Destination for the binary data
    pub fn write_h40_to<W: io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let hex40 = self.as_ref();

        // Process a nibble at a time
        for i in (0..OBJECT_ID_LENGTH).step_by(2) {
            let byte = u8::from_str_radix(&hex40[i..i + 2], 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid hex digit"))?;
            writer.write_all(&[byte])?;
        }

        Ok(())
    }

    /// Read an object ID from binary format (20 bytes)
    ///
    /// Reads 20 bytes and converts to a 40-character hex string.
    /// Used when deserializing tree and commit objects.
    ///
    /// # Arguments
    ///
    /// * `reader` - Source of the binary data
    pub fn read_h40_from<R: io::Read + ?Sized>(reader: &mut R) -> anyhow::Result<Self> {
        let mut hex40 = String::with_capacity(OBJECT_ID_LENGTH);
        let mut buffer = [0; 1];

        for _ in 0..(OBJECT_ID_LENGTH / 2) {
            reader.read_exact(&mut buffer)?;
            let hex_pair = &format!("{:02x}", u8::from_be_bytes(buffer));
            hex40.push_str(hex_pair);
        }

        Self::try_parse(hex40)
    }

    /// Convert to file system path for object storage
    ///
    /// Splits the hash as `XX/YYYYYY...` where XX is the first 2 chars.
    /// For example, `abc123...` becomes `ab/c123...`
    pub fn to_path(&self) -> PathBuf {
        let (dir, file) = self.0.split_at(2);
        PathBuf::from(dir).join(file)
    }

    /// Get abbreviated form of the object ID
    ///
    /// # Returns
    ///
    /// First 7 characters of the hash (standard Git abbreviation)
    pub fn to_short_oid(&self) -> String {
        self.0.split_at(7).0.to_string()
    }
}

impl AsRef<str> for ObjectId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
