use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct PathFilter {
    path_trie: SharedTrie<String>,
    root_path: PathBuf,
}

impl PathFilter {
    pub fn empty() -> Self {
        Self {
            path_trie: SharedTrie::with_matching(true),
            root_path: PathBuf::new(),
        }
    }

    pub fn new(paths: Vec<PathBuf>) -> Self {
        let mut trie = SharedTrie::new();

        for path in paths {
            let components: Vec<String> = path
                .components()
                .map(|comp| comp.as_os_str().to_string_lossy().to_string())
                .collect();
            trie.insert(&components);
        }

        Self {
            path_trie: trie,
            root_path: PathBuf::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.root_path
    }

    pub fn filter_matching_entries<'e, Entry: 'e>(
        &self,
        entries: impl Iterator<Item = (&'e String, &'e Entry)>,
    ) -> impl Iterator<Item = (&'e String, &'e Entry)> {
        entries.filter(move |(path_str, _)| self.path_trie.partly_contains(path_str))
    }

    pub fn join_subpath_filter(&self, subpath: &String) -> Self {
        let new_trie = if self.path_trie.is_root_matching() {
            self.path_trie.clone()
        } else {
            let node = self.path_trie.root.borrow();
            match node.children.get(subpath) {
                Some(child_node) => SharedTrie {
                    root: Rc::clone(child_node),
                },
                None => SharedTrie::new(),
            }
        };

        let mut new_root_path = self.root_path.clone();
        new_root_path.push(subpath);

        Self {
            path_trie: new_trie,
            root_path: new_root_path,
        }
    }
}

pub type TrieNodeRef<K> = Rc<RefCell<TrieNode<K>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedTrie<K: Hash + Eq + Clone> {
    root: TrieNodeRef<K>,
}

impl<K: Hash + Eq + Clone> SharedTrie<K> {
    pub fn new() -> Self {
        Self {
            root: Rc::new(RefCell::new(TrieNode::new())),
        }
    }

    pub fn with_matching(is_matching: bool) -> Self {
        Self {
            root: Rc::new(RefCell::new(TrieNode::with_matching(is_matching))),
        }
    }

    pub fn is_root_matching(&self) -> bool {
        self.root.borrow().is_end
    }

    pub fn insert(&mut self, path: &[K]) {
        let mut current = Rc::clone(&self.root);

        for part in path {
            let next = {
                let mut node = current.borrow_mut();

                node.children
                    .entry(part.clone())
                    .or_insert_with(|| Rc::new(RefCell::new(TrieNode::new())))
                    .clone()
            };
            current = next;
        }

        current.borrow_mut().is_end = true;
    }

    pub fn contains(&self, path: &[K]) -> bool {
        let mut current = Rc::clone(&self.root);

        for part in path {
            let next = {
                let node = current.borrow();

                match node.children.get(part) {
                    Some(child) => child.clone(),
                    None => return false,
                }
            };
            current = next;
        }

        current.borrow().is_end
    }

    pub fn partly_contains(&self, path_part: &K) -> bool {
        let node = self.root.borrow();

        if node.is_end {
            return true;
        }

        node.children.contains_key(path_part)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrieNode<K: Hash + Eq + Clone> {
    is_end: bool,
    children: HashMap<K, TrieNodeRef<K>>,
}

impl<K: Hash + Eq + Clone> TrieNode<K> {
    pub fn new() -> Self {
        Self {
            is_end: false,
            children: HashMap::new(),
        }
    }

    pub fn with_matching(is_matching: bool) -> Self {
        Self {
            is_end: is_matching,
            children: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Trie Tests ==========

    #[test]
    fn trie_insert_and_contains_single_path() {
        let mut trie = SharedTrie::new();
        let path = vec!["src", "main", "rs"];
        trie.insert(&path);

        assert!(trie.contains(&path));
    }

    #[test]
    fn trie_does_not_contain_nonexistent_path() {
        let mut trie = SharedTrie::new();
        let path = vec!["src", "main", "rs"];
        trie.insert(&path);

        assert!(!trie.contains(&["src", "lib", "rs"]));
        assert!(!trie.contains(&["docs", "README", "md"]));
    }

    #[test]
    fn trie_does_not_match_partial_path() {
        let mut trie = SharedTrie::new();
        let path = vec!["src", "main", "rs"];
        trie.insert(&path);

        // Partial paths should not match
        assert!(!trie.contains(&["src"]));
        assert!(!trie.contains(&["src", "main"]));
    }

    #[test]
    fn trie_contains_multiple_paths() {
        let mut trie = SharedTrie::new();
        trie.insert(&["src", "main", "rs"]);
        trie.insert(&["src", "lib", "rs"]);
        trie.insert(&["tests", "integration", "rs"]);

        assert!(trie.contains(&["src", "main", "rs"]));
        assert!(trie.contains(&["src", "lib", "rs"]));
        assert!(trie.contains(&["tests", "integration", "rs"]));
    }

    #[test]
    fn trie_handles_shared_prefixes() {
        let mut trie = SharedTrie::new();
        trie.insert(&["src", "utils", "helper", "rs"]);
        trie.insert(&["src", "utils", "config", "rs"]);
        trie.insert(&["src", "main", "rs"]);

        assert!(trie.contains(&["src", "utils", "helper", "rs"]));
        assert!(trie.contains(&["src", "utils", "config", "rs"]));
        assert!(trie.contains(&["src", "main", "rs"]));

        // Shared prefix is not a complete path
        assert!(!trie.contains(&["src", "utils"]));
    }

    #[test]
    fn trie_partly_contains_returns_true_when_matching() {
        let trie = SharedTrie::with_matching(true);

        // When is_matching is true, any path part should match
        assert!(trie.partly_contains(&"anything"));
        assert!(trie.partly_contains(&"src"));
    }

    #[test]
    fn trie_partly_contains_checks_children() {
        let mut trie = SharedTrie::new();
        trie.insert(&["src", "main"]);

        // Should match first component
        assert!(trie.partly_contains(&"src"));
        // Should not match non-existent component
        assert!(!trie.partly_contains(&"docs"));
    }

    #[test]
    fn trie_empty_path() {
        let mut trie = SharedTrie::new();
        let empty_path: Vec<&str> = vec![];
        trie.insert(&empty_path);

        // Empty path should mark the root as matching
        assert!(trie.is_root_matching());
        assert!(trie.contains(&empty_path));
    }

    #[test]
    fn trie_with_numeric_types() {
        let mut trie = SharedTrie::new();
        trie.insert(&[1, 2, 3]);
        trie.insert(&[1, 2, 4]);
        trie.insert(&[1, 5, 6]);

        assert!(trie.contains(&[1, 2, 3]));
        assert!(trie.contains(&[1, 2, 4]));
        assert!(trie.contains(&[1, 5, 6]));
        assert!(!trie.contains(&[1, 2]));
        assert!(!trie.contains(&[2, 3, 4]));
    }

    // ========== PathFilter Tests ==========

    #[test]
    fn path_filter_new_creates_trie_from_paths() {
        let paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")];
        let filter = PathFilter::new(paths);

        assert_eq!(filter.path(), Path::new(""));
    }

    #[test]
    fn path_filter_matches_exact_file() {
        let paths = vec![PathBuf::from("src/main.rs")];
        let filter = PathFilter::new(paths);

        let src = "src".to_string();
        let docs = "docs".to_string();
        let tests = "tests".to_string();
        let entries = vec![(&src, &1), (&docs, &2), (&tests, &3)];

        let filtered: Vec<_> = filter
            .filter_matching_entries(entries.into_iter())
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "src");
    }

    #[test]
    fn path_filter_matches_multiple_files() {
        let paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("tests/test.rs")];
        let filter = PathFilter::new(paths);

        let src = "src".to_string();
        let docs = "docs".to_string();
        let tests = "tests".to_string();
        let entries = vec![(&src, &1), (&docs, &2), (&tests, &3)];

        let filtered: Vec<_> = filter
            .filter_matching_entries(entries.into_iter())
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|(name, _)| name.as_str() == "src"));
        assert!(filtered.iter().any(|(name, _)| name.as_str() == "tests"));
    }

    #[test]
    fn path_filter_filters_out_non_matching_entries() {
        let paths = vec![PathBuf::from("src/main.rs")];
        let filter = PathFilter::new(paths);

        let docs = "docs".to_string();
        let config = "config".to_string();
        let entries = vec![(&docs, &1), (&config, &2)];

        let filtered: Vec<_> = filter
            .filter_matching_entries(entries.into_iter())
            .collect();

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn path_filter_join_path_and_advance_updates_root() {
        let paths = vec![PathBuf::from("src/utils/helper.rs")];
        let filter = PathFilter::new(paths);

        let advanced = filter.join_subpath_filter(&"src".to_string());

        assert_eq!(advanced.path(), Path::new("src"));
    }

    #[test]
    fn path_filter_join_path_and_advance_narrows_trie() {
        let paths = vec![
            PathBuf::from("src/utils/helper.rs"),
            PathBuf::from("src/main.rs"),
        ];
        let filter = PathFilter::new(paths);

        // Advance to "src"
        let filter_src = filter.join_subpath_filter(&"src".to_string());

        let utils = "utils".to_string();
        let main_rs = "main.rs".to_string();
        let lib_rs = "lib.rs".to_string();
        let entries = vec![(&utils, &1), (&main_rs, &2), (&lib_rs, &3)];

        let filtered: Vec<_> = filter_src
            .filter_matching_entries(entries.into_iter())
            .collect();

        // Should match "utils" and "main.rs" but not "lib.rs"
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|(name, _)| name.as_str() == "utils"));
        assert!(filtered.iter().any(|(name, _)| name.as_str() == "main.rs"));
    }

    #[test]
    fn path_filter_join_path_preserves_matching_state() {
        let paths = vec![PathBuf::from("src")];
        let filter = PathFilter::new(paths);

        // When the trie is already matching, advancing should preserve that state
        let filter_src = filter.join_subpath_filter(&"src".to_string());

        // After matching "src", everything under it should match
        let main_rs = "main.rs".to_string();
        let lib_rs = "lib.rs".to_string();
        let utils = "utils".to_string();
        let entries = vec![(&main_rs, &1), (&lib_rs, &2), (&utils, &3)];

        let filtered: Vec<_> = filter_src
            .clone()
            .join_subpath_filter(&"anything".to_string())
            .filter_matching_entries(entries.into_iter())
            .collect();

        // When is_matching is true, all entries should pass through
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn path_filter_with_directory_path() {
        let paths = vec![PathBuf::from("src/utils")];
        let filter = PathFilter::new(paths);

        let src = "src".to_string();
        let docs = "docs".to_string();
        let entries = vec![(&src, &1), (&docs, &2)];

        let filtered: Vec<_> = filter
            .filter_matching_entries(entries.into_iter())
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "src");
    }

    #[test]
    fn path_filter_handles_nested_paths() {
        let paths = vec![PathBuf::from("a/b/c/d.txt")];
        let filter = PathFilter::new(paths);

        let filter_a = filter.join_subpath_filter(&"a".to_string());
        assert_eq!(filter_a.path(), Path::new("a"));

        let filter_b = filter_a.join_subpath_filter(&"b".to_string());
        assert_eq!(filter_b.path(), Path::new("a/b"));

        let filter_c = filter_b.join_subpath_filter(&"c".to_string());
        assert_eq!(filter_c.path(), Path::new("a/b/c"));

        // At the "c" level, "d.txt" should match
        let d_txt = "d.txt".to_string();
        let other_txt = "other.txt".to_string();
        let entries = vec![(&d_txt, &1), (&other_txt, &2)];

        let filtered: Vec<_> = filter_c
            .filter_matching_entries(entries.into_iter())
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "d.txt");
    }

    #[test]
    fn path_filter_empty_filter_list() {
        let paths: Vec<PathBuf> = vec![];
        let filter = PathFilter::new(paths);

        let src = "src".to_string();
        let docs = "docs".to_string();
        let entries = vec![(&src, &1), (&docs, &2)];

        let filtered: Vec<_> = filter
            .filter_matching_entries(entries.into_iter())
            .collect();

        // No paths in filter means nothing matches
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn path_filter_join_path_with_non_matching_path() {
        let paths = vec![PathBuf::from("src/main.rs")];
        let filter = PathFilter::new(paths);

        // Advance with a path that doesn't match
        let filter_docs = filter.join_subpath_filter(&"docs".to_string());

        let readme_md = "README.md".to_string();
        let guide_md = "guide.md".to_string();
        let entries = vec![(&readme_md, &1), (&guide_md, &2)];

        let filtered: Vec<_> = filter_docs
            .filter_matching_entries(entries.into_iter())
            .collect();

        // Nothing should match since "docs" is not in the filter
        assert_eq!(filtered.len(), 0);
    }
}
