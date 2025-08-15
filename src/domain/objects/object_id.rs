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
}

impl AsRef<str> for ObjectId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
