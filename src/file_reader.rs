// Placeholder for future file reading functions
// This will implement read_file_text() and read_file_binary() scalar functions

use std::error::Error;
use std::fs;

pub fn read_file_as_text(path: &str) -> Result<String, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

pub fn read_file_as_binary(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let content = fs::read(path)?;
    Ok(content)
}