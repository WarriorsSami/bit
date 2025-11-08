use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn try_parse(id: String) -> anyhow::Result<Self> {
        if id.len() != 40 {
            return Err(anyhow::anyhow!("Invalid object ID length: {}", id.len()));
        }
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow::anyhow!("Invalid object ID characters: {}", id));
        }
        Ok(Self(id.to_string()))
    }

    pub fn write_h40_to<W: io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let hex40 = self.as_ref();

        // Process a nibble at a time
        for i in (0..40).step_by(2) {
            let byte = u8::from_str_radix(&hex40[i..i + 2], 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid hex digit"))?;
            writer.write_all(&[byte])?;
        }

        Ok(())
    }

    pub fn read_h40_from<R: io::Read + ?Sized>(reader: &mut R) -> anyhow::Result<Self> {
        let mut hex40 = String::with_capacity(40);
        let mut buffer = [0; 1];

        for _ in 0..20 {
            reader.read_exact(&mut buffer)?;
            let hex_pair = &format!("{:02x}", u8::from_be_bytes(buffer));
            hex40.push_str(hex_pair);
        }

        Self::try_parse(hex40)
    }

    pub fn to_path(&self) -> PathBuf {
        let (dir, file) = self.0.split_at(2);
        PathBuf::from(dir).join(file)
    }

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
