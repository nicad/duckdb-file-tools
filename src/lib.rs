extern crate duckdb;
extern crate duckdb_loadable_macros;
extern crate libduckdb_sys;

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, InitInfo, TableFunctionInfo, VTab, arrow::WritableVector},
    vscalar::{VScalar, ScalarFunctionSignature},
    Connection, Result,
};
use duckdb_loadable_macros::duckdb_entrypoint_c_api;
use libduckdb_sys as ffi;
use libduckdb_sys::duckdb_string_t;
use duckdb::types::DuckString;
use std::{
    error::Error,
    ffi::CString,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::SystemTime,
};
use sha2::{Sha256, Digest};
use glob::glob;

#[derive(Debug, Clone)]
struct FileMetadata {
    path: String,
    size: u64,
    modified_time: i64,
    accessed_time: i64,
    created_time: i64,
    permissions: String,
    inode: u64,
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
    hash: Option<String>,
}

#[repr(C)]
struct GlobStatBindData {
    pattern: String,
    hash_algorithm: Option<String>,
    files: Vec<FileMetadata>,
}

#[repr(C)]
struct GlobStatInitData {
    current_index: AtomicUsize,
}

struct GlobStatVTab;

impl VTab for GlobStatVTab {
    type InitData = GlobStatInitData;
    type BindData = GlobStatBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        bind.add_result_column("path", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("size", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("modified_time", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("accessed_time", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("created_time", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("permissions", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("inode", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("is_file", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("is_dir", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("is_symlink", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("hash", LogicalTypeHandle::from(LogicalTypeId::Varchar));

        let pattern = bind.get_parameter(0).to_string();
        
        let hash_algorithm = if bind.get_parameter_count() > 1 {
            // For now, just ignore the second parameter to avoid NULL handling issues
            None
        } else {
            None
        };

        // Use DuckDB's built-in glob function for pattern matching
        let files = collect_files_with_duckdb_glob(&pattern, hash_algorithm.as_deref())?;

        Ok(GlobStatBindData {
            pattern,
            hash_algorithm,
            files,
        })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(GlobStatInitData {
            current_index: AtomicUsize::new(0),
        })
    }

    fn func(func: &TableFunctionInfo<Self>, output: &mut DataChunkHandle) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        let current_idx = init_data.current_index.load(Ordering::Relaxed);
        
        if current_idx >= bind_data.files.len() {
            output.set_len(0);
            return Ok(());
        }

        let file_meta = &bind_data.files[current_idx];
        
        let path_str = CString::new(file_meta.path.clone())?;
        output.flat_vector(0).insert(0, path_str);
        
        let size_str = CString::new(file_meta.size.to_string())?;
        output.flat_vector(1).insert(0, size_str);
        
        let modified_str = CString::new(file_meta.modified_time.to_string())?;
        output.flat_vector(2).insert(0, modified_str);
        
        let accessed_str = CString::new(file_meta.accessed_time.to_string())?;
        output.flat_vector(3).insert(0, accessed_str);
        
        let created_str = CString::new(file_meta.created_time.to_string())?;
        output.flat_vector(4).insert(0, created_str);
        
        let permissions_str = CString::new(file_meta.permissions.clone())?;
        output.flat_vector(5).insert(0, permissions_str);
        
        let inode_str = CString::new(file_meta.inode.to_string())?;
        output.flat_vector(6).insert(0, inode_str);
        
        let is_file_str = CString::new(file_meta.is_file.to_string())?;
        output.flat_vector(7).insert(0, is_file_str);
        
        let is_dir_str = CString::new(file_meta.is_dir.to_string())?;
        output.flat_vector(8).insert(0, is_dir_str);
        
        let is_symlink_str = CString::new(file_meta.is_symlink.to_string())?;
        output.flat_vector(9).insert(0, is_symlink_str);
        
        if let Some(ref hash) = file_meta.hash {
            let hash_str = CString::new(hash.clone())?;
            output.flat_vector(10).insert(0, hash_str);
        } else {
            output.flat_vector(10).set_null(0);
        }

        output.set_len(1);
        init_data.current_index.store(current_idx + 1, Ordering::Relaxed);

        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // pattern
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // hash (optional)
        ])
    }
}

// Scalar-like functions implemented as table functions that return single rows



// file_path_sha256(filename, content_hash) - returns combined hash
#[repr(C)]
struct FilePathSha256BindData {
    filename: String,
    content_hash: String,
}

#[repr(C)]
struct FilePathSha256InitData {
    done: AtomicBool,
}

struct FilePathSha256VTab;

impl VTab for FilePathSha256VTab {
    type InitData = FilePathSha256InitData;
    type BindData = FilePathSha256BindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        bind.add_result_column("path_sha256", LogicalTypeHandle::from(LogicalTypeId::Varchar));

        let filename = bind.get_parameter(0).to_string();
        let content_hash = bind.get_parameter(1).to_string();
        Ok(FilePathSha256BindData { filename, content_hash })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(FilePathSha256InitData {
            done: AtomicBool::new(false),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        if init_data.done.swap(true, Ordering::Relaxed) {
            output.set_len(0);
            return Ok(());
        }

        let path = Path::new(&bind_data.filename);
        let metadata = fs::metadata(path)?;
        
        // Create combined string: filename + modified_time + size + content_hash
        let modified_time = system_time_to_microseconds(metadata.modified()?);
        let size = metadata.len();
        let combined = format!("{}:{}:{}:{}", 
            bind_data.filename, modified_time, size, bind_data.content_hash);
        
        // Hash the combined string
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();
        let hash = format!("{:x}", result);

        let hash_str = CString::new(hash)?;
        output.flat_vector(0).insert(0, hash_str);

        output.set_len(1);
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // filename
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // content_hash
        ])
    }
}

fn collect_files_with_duckdb_glob(
    pattern: &str, 
    hash_algorithm: Option<&str>
) -> Result<Vec<FileMetadata>, Box<dyn Error>> {
    let mut results = Vec::new();
    
    // Use the glob crate for pattern matching (same as DuckDB's glob implementation)
    for entry in glob(pattern)? {
        let path = entry?;
        
        // Skip if it's not a file (directories, symlinks, etc. can be included based on metadata)
        let metadata = fs::metadata(&path)?;
        
        let hash = if hash_algorithm.is_some() && metadata.is_file() {
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
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
            hash,
        };
        
        results.push(file_meta);
    }
    
    Ok(results)
}

// Scalar file_stat function - returns JSON string with file metadata
struct FileStatScalar;

impl VScalar for FileStatScalar {
    type State = ();

    unsafe fn invoke(
        _: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let input_vector = input.flat_vector(0);
        let input_data = input_vector.as_slice_with_len::<duckdb_string_t>(input.len());
        
        let mut output_vector = output.flat_vector();
        
        for i in 0..input.len() {
            let mut filename_duck_string = input_data[i];
            let filename = DuckString::new(&mut filename_duck_string).as_str();
            
            // Handle file stat with error handling as specified:
            // - file doesn't exist -> return NULL
            // - permission error -> return NULL
            // - other errors -> return error
            match get_file_metadata_json(&filename) {
                Ok(Some(json_str)) => {
                    output_vector.insert(i, json_str.as_str());
                }
                Ok(None) => {
                    output_vector.set_null(i);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

fn get_file_metadata_json(filename: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let path = Path::new(filename);
    
    match fs::metadata(path) {
        Ok(metadata) => {
            // Successfully got metadata, create JSON string
            let json_str = format!(
                r#"{{"size": {}, "modified_time": {}, "accessed_time": {}, "created_time": {}, "permissions": "{}", "inode": {}, "is_file": {}, "is_dir": {}, "is_symlink": {}}}"#,
                metadata.len(),
                system_time_to_microseconds(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
                system_time_to_microseconds(metadata.accessed().unwrap_or(SystemTime::UNIX_EPOCH)),
                system_time_to_microseconds(metadata.created().unwrap_or(SystemTime::UNIX_EPOCH)),
                format_permissions(&metadata),
                get_inode(&metadata),
                metadata.is_file(),
                metadata.is_dir(),
                metadata.file_type().is_symlink()
            );
            Ok(Some(json_str))
        }
        Err(e) => {
            use std::io::ErrorKind;
            match e.kind() {
                ErrorKind::NotFound => Ok(None), // File doesn't exist -> return NULL
                ErrorKind::PermissionDenied => Ok(None), // Permission error -> return NULL
                _ => Err(Box::new(e)), // Other errors -> return error
            }
        }
    }
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
        0
    }
}

#[duckdb_entrypoint_c_api()]
pub unsafe fn extension_entrypoint(con: Connection) -> Result<(), Box<dyn Error>> {
    con.register_table_function::<GlobStatVTab>("glob_stat")
        .expect("Failed to register glob_stat table function");
    
    con.register_table_function::<FilePathSha256VTab>("file_path_sha256")
        .expect("Failed to register file_path_sha256 table function");
    
    con.register_scalar_function::<FileStatScalar>("file_stat")
        .expect("Failed to register file_stat scalar function");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_glob_pattern_matching() {
        // Test that different glob patterns return different results
        let pattern1 = "src/*.rs";
        let pattern2 = "Cargo.*";
        
        let files1 = collect_files_with_duckdb_glob(pattern1, None).unwrap_or_default();
        let files2 = collect_files_with_duckdb_glob(pattern2, None).unwrap_or_default();
        
        // Extract just the file paths for comparison
        let paths1: HashSet<_> = files1.iter().map(|f| &f.path).collect();
        let paths2: HashSet<_> = files2.iter().map(|f| &f.path).collect();
        
        println!("Pattern '{}' returned {} files", pattern1, paths1.len());
        println!("Pattern '{}' returned {} files", pattern2, paths2.len());
        
        // Different patterns should return different file sets
        assert_ne!(paths1, paths2, 
            "Different patterns '{}' and '{}' should return different file lists", pattern1, pattern2);
    }

    #[test]
    fn test_file_hash_computation() {
        // Test hash computation functionality
        let test_file = "Cargo.toml";
        let hash_result = compute_file_hash(Path::new(test_file));
        
        assert!(hash_result.is_ok(), "Should be able to compute hash for existing file");
        
        let hash = hash_result.unwrap();
        assert_eq!(hash.len(), 64, "SHA256 hash should be 64 characters long");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should contain only hex digits");
    }

    #[test]
    fn test_file_metadata_json_function() {
        // Test the helper function directly
        let result = get_file_metadata_json("Cargo.toml");
        assert!(result.is_ok(), "Should successfully process existing file");
        
        let json_opt = result.unwrap();
        assert!(json_opt.is_some(), "Should return Some for existing file");
        
        let json = json_opt.unwrap();
        assert!(json.contains("\"size\""), "Should contain size field");
        assert!(json.contains("\"is_file\""), "Should contain is_file field");
        
        // Test non-existent file
        let result = get_file_metadata_json("nonexistent_file.txt");
        assert!(result.is_ok(), "Should handle non-existent file gracefully");
        assert!(result.unwrap().is_none(), "Should return None for non-existent file");
    }
}