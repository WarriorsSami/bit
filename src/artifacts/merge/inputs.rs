use crate::areas::database::CommitCache;
use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::merge::bca_finder::BCAFinder;
use crate::artifacts::objects::object_id::ObjectId;

pub struct MergeInputs<'r> {
    left_name: &'r str,
    right_name: &'r str,
    left_oid: ObjectId,
    right_oid: ObjectId,
    base_oid: ObjectId,
}

impl<'r> MergeInputs<'r> {
    pub fn new(
        repository: &'r Repository,
        left_name: &'r str,
        right_name: &'r str,
    ) -> anyhow::Result<Self> {
        let left_oid = Self::resolve(repository, left_name)?;
        let right_oid = Self::resolve(repository, right_name)?;
        let base_oid = Self::find_best_common_ancestor(repository, &left_oid, &right_oid)?;

        Ok(Self {
            left_name,
            right_name,
            left_oid,
            right_oid,
            base_oid,
        })
    }

    pub fn left_name(&self) -> &str {
        self.left_name
    }

    pub fn right_name(&self) -> &str {
        self.right_name
    }

    pub fn left_oid(&self) -> &ObjectId {
        &self.left_oid
    }

    pub fn right_oid(&self) -> &ObjectId {
        &self.right_oid
    }

    pub fn base_oid(&self) -> &ObjectId {
        &self.base_oid
    }

    fn resolve(repository: &'r Repository, ref_name: &'r str) -> anyhow::Result<ObjectId> {
        Revision::try_parse(ref_name)?
            .resolve(repository)?
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve reference: {}", ref_name))
    }

    fn find_best_common_ancestor<'m>(
        repository: &'r Repository,
        left_oid: &'m ObjectId,
        right_oid: &'m ObjectId,
    ) -> anyhow::Result<ObjectId> {
        let commit_cache = CommitCache::new();
        let database = repository.database();

        let best_common_ancestor_finder = BCAFinder::new(|oid| {
            commit_cache
                .get_or_load_slim_commit(database, oid)
                .expect("Failed to load commit")
        });

        best_common_ancestor_finder
            .find_best_common_ancestor(left_oid, right_oid)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No common ancestor found between {} and {}",
                    left_oid.to_short_oid(),
                    right_oid.to_short_oid()
                )
            })
    }
}
