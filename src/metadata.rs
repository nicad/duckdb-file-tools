use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use jwalk::WalkDir;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified_time: i64, // Unix timestamp in microseconds
    pub accessed_time: i64,
    pub created_time: i64,
    pub permissions: String,
    pub inode: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub hash: Option<String>,
}

pub fn collect_file_metadata(
    pattern: &str,
    hash_algorithm: Option<&str>,
) -> Result<Vec<FileMetadata>, Box<dyn Error>> {
    let mut results = Vec::new();
    
    // For now, implement basic glob matching using jwalk
    // This is a simplified implementation - proper glob matching would need a glob library
    let base_path = extract_base_path(pattern);
    let glob_pattern = extract_glob_pattern(pattern);
    
    for entry in WalkDir::new(base_path) {
        let entry = entry?;
        let path = entry.path();
        
        if !matches_pattern(&path, &glob_pattern) {
            continue;
        }
        
        let metadata = fs::metadata(&path)?;
        let file_type = metadata.file_type();
        
        let hash = if hash_algorithm.is_some() && file_type.is_file() {
            Some(compute_file_hash(&path)?)
        } else {
            None
        };
        
        let file_meta = FileMetadata {
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            modified_time: system_time_to_microseconds(metadata.modified()?),
            accessed_time: system_time_to_microseconds(metadata.accessed()?),
            created_time: system_time_to_microseconds(metadata.created().unwrap_or(metadata.modified()?)),
            permissions: format_permissions(&metadata),
            inode: get_inode(&metadata),
            is_file: file_type.is_file(),
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
            hash,
        };
        
        results.push(file_meta);
    }
    
    Ok(results)
}

fn extract_base_path(pattern: &str) -> &str {
    // Find the first glob character and extract the base directory
    let glob_chars = ['*', '?', '[', '{'];
    
    if let Some(pos) = pattern.find(|c| glob_chars.contains(&c)) {
        let base_end = pattern[..pos].rfind('/').unwrap_or(0);
        if base_end == 0 && pattern.starts_with('/') {
            "/"
        } else if base_end == 0 {
            "."
        } else {
            &pattern[..base_end]
        }
    } else {
        pattern
    }
}

fn extract_glob_pattern(pattern: &str) -> String {
    pattern.to_string()
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    // Simplified pattern matching - in production, use a proper glob library
    let path_str = path.to_string_lossy();
    
    if pattern.contains("**") {
        // Recursive glob - simplified check
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');
            
            let prefix_match = prefix.is_empty() || path_str.starts_with(prefix);
            let suffix_match = suffix.is_empty() || path_str.ends_with(suffix);
            
            return prefix_match && suffix_match;
        }
    }
    
    if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return path_str.starts_with(prefix) && path_str.ends_with(suffix);
        }
    }
    
    // Exact match
    path_str == pattern
}

fn compute_file_hash(path: &Path) -> Result<String, Box<dyn Error>> {
    let contents = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn system_time_to_microseconds(time: SystemTime) -> i64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}

fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        format!("{:o}", metadata.permissions().mode())
    }
    
    #[cfg(windows)]
    {
        if metadata.permissions().readonly() {
            "r--r--r--".to_string()
        } else {
            "rw-rw-rw-".to_string()
        }
    }
}

fn get_inode(metadata: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.ino()
    }
    
    #[cfg(windows)]
    {
        0 // Windows doesn't have inodes
    }
}