use redo::Target;
use std::env;
use std::io::Result;
use std::iter::Iterator;

fn _program_name() -> String {
    if let Some(exe) = env::current_exe().ok() {
        if let Some(file) = exe.file_name() {
            return String::from(file.to_str().unwrap());
        }
    }
    String::from("redo")
}

fn main() -> Result<()> {
    for arg in env::args().skip(1) {
        Target::load(&arg).redo()?;
    }
    Ok(())
}
