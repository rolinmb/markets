use std::path::Path;
use std::fs;
use std::io;

pub fn str_to_float(s: &str) -> f64 {
  s.replace(",", "").parse::<f64>().unwrap_or(0.0)
}

pub fn clear_directory_or_create(dir_name: &str) -> io::Result<()> {
    let dir = Path::new(dir_name);
    if dir.exists() {
        println!("clear_directory_or_create() :: Cleaning directory {}", dir_name);
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    } else {
        println!("clean_directory_or_create() :: Creating directory {}", dir_name);
        fs::create_dir_all(dir)?;
    }
    println!("clean_directory_or_create() :: Successfully created/cleaned directory {}", dir_name);
    Ok(())
}

pub fn create_directory_if_dne(dir_name: &str) -> io::Result<()> {
    let dir = Path::new(dir_name);
    if !dir.exists() {
        println!("create_directory_if_dne() :: Creating directory {}", dir_name);
        fs::create_dir_all(dir)?;
    }
    println!("create_directory_if_dne() :: Directory {} already exists; nothing to create", dir_name);
    Ok(())
}