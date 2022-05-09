use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: std::path::PathBuf,
    pub hash: String,
}

impl Dependency {
    pub fn compute_hash(&self) -> String {
        let mut hasher = Md5::new();
        std::fs::File::open(&self.name)
            .and_then(|mut file| std::io::copy(&mut file, &mut hasher))
            .map(|_| base16ct::lower::encode_string(&hasher.finalize()))
            .unwrap_or(String::new())
    }

    pub fn update_hash(&mut self) {
        self.hash = self.compute_hash();
    }

    pub fn needs_update(&self) -> bool {
        self.compute_hash() != self.hash
    }
}
