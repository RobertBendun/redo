use std::env;
use std::path::Path;
use std::process::Command;
use tempfile::*;
use json::{JsonValue, object};
use std::io::{Result};
use md5::{Md5, Digest};

const REDO_DATA : &str = ".redo.json";

fn _program_name() -> String {
    if let Some(exe) = env::current_exe().ok() {
        if let Some(file) = exe.file_name() {
            return String::from(file.to_str().unwrap())
        }
    }
    String::from("redo")
}

fn do_file(target: &str) -> String {
    let do_script = String::from(target) + ".do";
    if !Path::new(&do_script).exists() {
        eprintln!("[REDO ERROR] Couldn't find '{}' script for given target '{}'", do_script, target);
        std::process::exit(1);
    }
    do_script
}

fn basename(target: &str) -> String {
    std::path::Path::new(target)
        .file_name().unwrap()
        .to_owned()
        .into_string().unwrap()
}

fn read() -> JsonValue {
    if let Ok(content) = std::fs::read_to_string(REDO_DATA) {
        json::parse(&content).ok().unwrap_or_else(|| panic!("Invalid JSON in file {}", REDO_DATA))
    } else {
        JsonValue::Array(vec![])
    }
}

fn write(target: &str, depenency: &str, hash: &str) -> Result<()> {
    let mut entries = match read() {
        JsonValue::Array(vec) => vec,
        _ => panic!("invalid json: array must be a top level element"),
    };

    let mut found = false;
    for entry in &mut entries {
        if !entry.is_object() {
            panic!("Entry {:?} is not an object", entry);
        }

        if entry["target"] == target {
            if !entry.has_key("dependencies") {
                entry["dependencies"] = JsonValue::Array(vec![]);
            }
            entry["dependencies"]
                .push(object! { name: depenency, hash: hash })
                .expect("failed to push new dependency");
            found = true;
            break;
        }
    }
    if !found {
        entries.push(object! {
            target: target,
            dependencies: [ object! { name: depenency, hash: hash } ]
        });
    }

    std::fs::write(REDO_DATA, json::stringify_pretty(entries, 2))
}

fn hash(path: &str) -> Result<String> {
    let mut hasher = Md5::new();
    std::io::copy(&mut std::fs::File::open(path)?, &mut hasher).unwrap();
    let hash = hasher.finalize();
    Ok(base16ct::lower::encode_string(&hash))
}

fn redo(target: &str) -> Result<()> {
    let do_file = do_file(target);
    write(target, &do_file, &hash(&do_file)?)?;

    println!("redo {}", do_file);
    let tmp = NamedTempFile::new()?;
    let tmp_path = tmp.into_temp_path();

    let status = Command::new("sh")
        .args([ "-e", &do_file, "", &basename(target), tmp_path.to_str().unwrap()])
        .status();

    let exit_code = status?.code().unwrap_or(1);
    if exit_code != 0 {
        eprintln!("[REDO ERROR] Do script ended with non-zero exit code: {}", exit_code);
        std::process::exit(1);
    }

    std::fs::rename(tmp_path, target)
}

fn main() -> Result<()> {
    let _top = env::current_dir()?;
    for arg in env::args().skip(1) {
        redo(&arg)?
    }
    Ok(())
}
