use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

/// Dependency of Target that determine if Target should be rebuilded
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: std::path::PathBuf,
    pub hash: String,
}

impl Dependency {
    /// Computes `hash` from file pointed by `name`
    pub fn compute_hash(&self) -> String {
        let mut hasher = Md5::new();
        std::fs::File::open(&self.name)
            .and_then(|mut file| std::io::copy(&mut file, &mut hasher))
            .map(|_| base16ct::lower::encode_string(&hasher.finalize()))
            .unwrap_or(String::new())
    }

    /// Sets `hash` as hash computed from file pointed by `name`
    pub fn update_hash(&mut self) {
        self.hash = self.compute_hash();
    }

    /// Dependency needs an update when cached `hash` is different then hash
    /// of file pointed by `name`
    pub fn needs_update(&self) -> bool {
        self.compute_hash() != self.hash
    }
}
