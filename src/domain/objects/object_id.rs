use std::io;

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

    pub fn write_h40_to<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let hex40 = self.as_ref();

        // Process a nibble at a time
        for i in (0..40).step_by(2) {
            let byte = u8::from_str_radix(&hex40[i..i + 2], 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid hex digit"))?;
            writer.write_all(&[byte])?;
        }

        Ok(())
    }
}

impl AsRef<str> for ObjectId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
