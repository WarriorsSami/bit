use crate::domain::areas::repository::Repository;
use std::collections::BTreeSet;
use std::path::PathBuf;

impl Repository {
    pub async fn status(&mut self) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;
        index.rehydrate()?;

        let mut untracked_files = BTreeSet::new();

        self.scan_workspace(None, &mut untracked_files).await?;

        untracked_files.iter().for_each(|file| {
            writeln!(self.writer(), "?? {}", file.display()).unwrap();
        });

        Ok(())
    }

    async fn scan_workspace(
        &self,
        prefix_path: Option<PathBuf>,
        untracked_files: &mut BTreeSet<PathBuf>,
    ) -> anyhow::Result<()> {
        let index = self.index();
        let index = index.lock().await;

        let files = self.workspace().list_dir(prefix_path.clone())?;

        println!("Scanning {:?}", prefix_path);
        for path in files.iter() {
            println!("Found {:?}", path);
            if index.is_tracked(path) && path.is_dir() {
                Box::pin(self.scan_workspace(Some(path.clone()), untracked_files)).await?;
            } else if !index.is_tracked(path) {
                // add the file separator if it's a directory
                let path = if path.is_dir() {
                    let mut p = path.clone();
                    p.push("");
                    p
                } else {
                    path.clone()
                };
                untracked_files.insert(path);
            }
        }

        Ok(())
    }
}
