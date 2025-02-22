use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::commit::Commit;
use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use crate::domain::objects::tree::Tree;

impl Repository {
    pub fn cat_file(&mut self, object_id: &str) -> anyhow::Result<()> {
        // read object file
        let object_data = self.database().load(object_id)?;
        let object_data = String::from_utf8(object_data.to_vec())?;

        // deserialize based on object type extracted from header
        let object_type: ObjectType = object_data
            .split_whitespace()
            .next()
            .unwrap_or("")
            .try_into()?;
        let object: Box<dyn Object> = match object_type {
            ObjectType::Blob => Box::new(Blob::try_from(object_data)?),
            ObjectType::Tree => Box::new(Tree::try_from(object_data)?),
            ObjectType::Commit => Box::new(Commit::try_from(object_data)?),
        };

        write!(self.writer(), "{}", object.display())?;

        Ok(())
    }
}
