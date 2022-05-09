use serde::{Deserialize, Serialize};
use std::io::Result;
use std::path::PathBuf;
use std::process::Command;
use tempfile::*;

pub use crate::dependency::Dependency;

/// Cache file for dependencies hashes for targets (one per directory)
pub const REDO_DATA: &str = ".redo.json";

/// Target describes `redo` target that supposed to be build
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Target {
    #[serde(alias = "target")]
    pub path: PathBuf,
    pub dependencies: Vec<Dependency>,
}

type Targets = Vec<Target>;

impl Target {
    /// Load target from cache
    pub fn load(target: &str) -> Target {
        Target::load_from_redo_cache(target)
    }

    /// Determines if Target needs to be updated based on dependencies
    pub fn needs_update(&self) -> bool {
        let mut at_least_one_iteration = false;
        for dep in self.dependencies.iter() {
            at_least_one_iteration = true;
            if dep.needs_update() {
                return true;
            }
        }
        !at_least_one_iteration
    }

    /// Executes `redo` mechanics according to specification
    ///
    /// - If file is up to date then inform user and return
    /// - Otherwise:
    ///   - Update dependencies and their hashes
    ///   - Save dependencies to cache file
    ///   - run associated `.do` file to produce target
    pub fn redo(mut self) -> Result<()> {
        if !self.needs_update() {
            println!("{} is up to date", self.path.to_str().unwrap());
            return Ok(());
        }

        for dependency in &mut self.dependencies {
            dependency.update_hash();
        }

        let do_file = self.do_file_path();
        write(&self)?;
        println!("redo {}", self.path.to_str().unwrap());

        let tmp = NamedTempFile::new()?;
        let tmp_path = tmp.into_temp_path();

        let status = Command::new("sh")
            .args([
                "-e",
                &to_str(do_file),
                "",
                &to_str(PathBuf::from(self.path.file_stem().unwrap())),
                tmp_path.to_str().unwrap(),
            ])
            .status();

        let exit_code = status?.code().unwrap_or(1);
        if exit_code != 0 {
            eprintln!(
                "[REDO ERROR] Do script ended with non-zero exit code: {}",
                exit_code
            );
            std::process::exit(1);
        }

        std::fs::rename(tmp_path, self.path)
    }

    fn load_from_redo_cache(target: &str) -> Target {
        let target_path = std::path::PathBuf::from(target);
        let mut target = read()
            .unwrap()
            .into_iter()
            .find(|cached| cached.path == target_path)
            .unwrap_or_else(|| Target {
                path: PathBuf::from(target),
                dependencies: vec![],
            });

        target.ensure_do_dependency_exists();
        target
    }

    fn do_file_path(&self) -> PathBuf {
        self.path.with_extension("do")
    }

    fn ensure_do_dependency_exists(&mut self) {
        let do_path = self.do_file_path();
        if let None = self.dependencies.iter().find(|dep| dep.name == do_path) {
            self.dependencies.push(Dependency {
                name: do_path,
                hash: String::new(),
            })
        }
    }
}

fn into_io_result<T, E>(result: std::result::Result<T, E>) -> std::io::Result<T>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    use std::io::*;
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(Error::new(ErrorKind::Other, error)),
    }
}

fn read() -> Result<Targets> {
    let cache = std::fs::read_to_string(REDO_DATA)?;
    into_io_result(serde_json::from_str::<Targets>(&cache))
}

fn write(target: &Target) -> Result<()> {
    let mut cache = read()?;

    if let Some(entry) = cache.iter_mut().find(|entry| target.path == entry.path) {
        *entry = target.clone();
    } else {
        cache.push(target.clone());
    }

    let cache = into_io_result(serde_json::to_string_pretty(&cache))?;
    std::fs::write(REDO_DATA, cache)
}

fn to_str(path: PathBuf) -> String {
    path.into_os_string().into_string().unwrap()
}
