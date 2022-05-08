use json::{object, JsonValue};
use md5::{Digest, Md5};
use std::env;
use std::io::Result;
use std::iter::Iterator;
use std::path::Path;
use std::process::Command;
use tempfile::*;

const REDO_DATA: &str = ".redo.json";

fn _program_name() -> String {
    if let Some(exe) = env::current_exe().ok() {
        if let Some(file) = exe.file_name() {
            return String::from(file.to_str().unwrap());
        }
    }
    String::from("redo")
}

fn do_file(target: &str) -> String {
    let do_script = String::from(target) + ".do";
    if !Path::new(&do_script).exists() {
        eprintln!(
            "[REDO ERROR] Couldn't find '{}' script for given target '{}'",
            do_script, target
        );
        std::process::exit(1);
    }
    do_script
}

fn basename(target: &str) -> String {
    std::path::Path::new(target)
        .file_name()
        .unwrap()
        .to_owned()
        .into_string()
        .unwrap()
}

fn read() -> JsonValue {
    if let Ok(content) = std::fs::read_to_string(REDO_DATA) {
        json::parse(&content)
            .ok()
            .unwrap_or_else(|| panic!("Invalid JSON in file {}", REDO_DATA))
    } else {
        JsonValue::Array(vec![])
    }
}

fn write(target: &str, depenency: &str, hash: &str) -> Result<()> {
    let mut entries = read();

    if let Some(ref mut entry) = entries
        .members_mut()
        .find(|entry| entry["target"] == target)
    {
        if let Some(ref mut dep) = entry["dependencies"]
            .members_mut()
            .find(|entry| entry["name"] == depenency)
        {
            dep["hash"] = JsonValue::String(String::from(hash));
        } else {
            entry["dependencies"]
                .push(object! { name: depenency, hash: hash })
                .expect("failed to push new dependency");
        }
    } else {
        entries
            .push(object! {
                target: target,
                dependencies: [ object! { name: depenency, hash: hash } ]
            })
            .unwrap();
    }

    std::fs::write(REDO_DATA, json::stringify_pretty(entries, 2))
}

fn needs_update(target: &str) -> bool {
    let entries = read();

    if let Some(target) = entries.members().find(|entry| entry["target"] == target) {
        let mut at_least_one_iteration = false;
        for dep in target["dependencies"].members() {
            at_least_one_iteration = true;
            if dep["name"]
                .as_str()
                .and_then(|name| hash(name).ok())
                .map(|hash| hash != dep["hash"])
                .unwrap_or(true)
            {
                return true;
            }
        }
        !at_least_one_iteration
    } else {
        true
    }
}

fn hash(path: &str) -> Result<String> {
    let mut hasher = Md5::new();
    std::io::copy(&mut std::fs::File::open(path)?, &mut hasher).unwrap();
    let hash = hasher.finalize();
    Ok(base16ct::lower::encode_string(&hash))
}

fn redo(target: &str) -> Result<()> {
    if !needs_update(target) {
        println!("{} is up to date", target);
        return Ok(());
    }

    let do_file = do_file(target);
    write(target, &do_file, &hash(&do_file)?)?;

    println!("redo {}", do_file);
    let tmp = NamedTempFile::new()?;
    let tmp_path = tmp.into_temp_path();

    let status = Command::new("sh")
        .args([
            "-e",
            &do_file,
            "",
            &basename(target),
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

    std::fs::rename(tmp_path, target)
}

fn main() -> Result<()> {
    for arg in env::args().skip(1) {
        redo(&arg)?
    }
    Ok(())
}
