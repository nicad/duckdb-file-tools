extern crate duckdb;
extern crate duckdb_loadable_macros;
extern crate libduckdb_sys;

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId, ListVector},
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
    io::Read,
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

        let pattern = bind.get_parameter(0).to_string();

        // Use DuckDB's built-in glob function for pattern matching (no hash computation)
        let files = collect_files_with_duckdb_glob(&pattern)?;

        Ok(GlobStatBindData {
            pattern,
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

        output.set_len(1);
        init_data.current_index.store(current_idx + 1, Ordering::Relaxed);

        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // pattern
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

fn collect_files_with_duckdb_glob(pattern: &str) -> Result<Vec<FileMetadata>, Box<dyn Error>> {
    let mut results = Vec::new();
    
    // Use the glob crate for pattern matching (same as DuckDB's glob implementation)
    for entry in glob(pattern)? {
        let path = entry?;
        
        // Skip if it's not a file (directories, symlinks, etc. can be included based on metadata)
        let metadata = fs::metadata(&path)?;
        
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
            hash: None, // No hash computation in glob_stat
        };
        
        results.push(file_meta);
    }
    
    Ok(results)
}

// Scalar file_stat function - returns STRUCT with file metadata
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
        
        let mut struct_vector = output.struct_vector();
        
        // Get child vectors for each field
        let mut size_vector = struct_vector.child(0, input.len());          // size: BIGINT
        let mut modified_vector = struct_vector.child(1, input.len());      // modified_time: TIMESTAMP
        let mut accessed_vector = struct_vector.child(2, input.len());      // accessed_time: TIMESTAMP  
        let mut created_vector = struct_vector.child(3, input.len());       // created_time: TIMESTAMP
        let permissions_vector = struct_vector.child(4, input.len());   // permissions: VARCHAR
        let mut inode_vector = struct_vector.child(5, input.len());         // inode: BIGINT
        let mut is_file_vector = struct_vector.child(6, input.len());       // is_file: BOOLEAN
        let mut is_dir_vector = struct_vector.child(7, input.len());        // is_dir: BOOLEAN
        let mut is_symlink_vector = struct_vector.child(8, input.len());    // is_symlink: BOOLEAN
        
        // Get raw data slices for direct assignment
        let size_data = size_vector.as_mut_slice::<i64>();
        let modified_data = modified_vector.as_mut_slice::<i64>();
        let accessed_data = accessed_vector.as_mut_slice::<i64>();
        let created_data = created_vector.as_mut_slice::<i64>();
        let inode_data = inode_vector.as_mut_slice::<u64>();
        let is_file_data = is_file_vector.as_mut_slice::<bool>();
        let is_dir_data = is_dir_vector.as_mut_slice::<bool>();
        let is_symlink_data = is_symlink_vector.as_mut_slice::<bool>();
        
        for i in 0..input.len() {
            let mut filename_duck_string = input_data[i];
            let filename = DuckString::new(&mut filename_duck_string).as_str();
            
            // Handle file stat with error handling as specified:
            // - file doesn't exist -> return NULL
            // - permission error -> return NULL
            // - other errors -> return error
            match get_file_metadata_struct(&filename) {
                Ok(Some(metadata)) => {
                    // Set all fields in the struct
                    size_data[i] = metadata.size as i64;
                    modified_data[i] = metadata.modified_time;
                    accessed_data[i] = metadata.accessed_time;
                    created_data[i] = metadata.created_time;
                    permissions_vector.insert(i, metadata.permissions.as_str());
                    inode_data[i] = metadata.inode;
                    is_file_data[i] = metadata.is_file;
                    is_dir_data[i] = metadata.is_dir;
                    is_symlink_data[i] = metadata.is_symlink;
                }
                Ok(None) => {
                    // Set entire struct row as NULL
                    struct_vector.set_null(i);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        // Create STRUCT return type with named fields
        let struct_type = LogicalTypeHandle::struct_type(&[
            ("size", LogicalTypeHandle::from(LogicalTypeId::Bigint)),
            ("modified_time", LogicalTypeHandle::from(LogicalTypeId::Timestamp)),
            ("accessed_time", LogicalTypeHandle::from(LogicalTypeId::Timestamp)),
            ("created_time", LogicalTypeHandle::from(LogicalTypeId::Timestamp)),
            ("permissions", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("inode", LogicalTypeHandle::from(LogicalTypeId::Bigint)),
            ("is_file", LogicalTypeHandle::from(LogicalTypeId::Boolean)),
            ("is_dir", LogicalTypeHandle::from(LogicalTypeId::Boolean)),
            ("is_symlink", LogicalTypeHandle::from(LogicalTypeId::Boolean)),
        ]);
        
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            struct_type,
        )]
    }
}

// Scalar file_sha256 function - returns SHA256 hash as lowercase hex string
struct FileSha256Scalar;

impl VScalar for FileSha256Scalar {
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
            
            // Handle file hashing with error handling as specified:
            // - file doesn't exist -> return NULL
            // - permission error -> return NULL
            // - other errors -> return error
            match compute_file_sha256(&filename) {
                Ok(Some(hash_str)) => {
                    output_vector.insert(i, hash_str.as_str());
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

// Scalar path_parts function - returns STRUCT with path component information
struct PathPartsScalar;

impl VScalar for PathPartsScalar {
    type State = ();

    unsafe fn invoke(
        _: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let input_vector = input.flat_vector(0);
        let input_data = input_vector.as_slice_with_len::<duckdb_string_t>(input.len());
        
        let mut struct_vector = output.struct_vector();
        
        // Get child vectors for each field
        let drive_vector = struct_vector.child(0, input.len());           // drive: VARCHAR
        let root_vector = struct_vector.child(1, input.len());            // root: VARCHAR
        let anchor_vector = struct_vector.child(2, input.len());          // anchor: VARCHAR
        let parent_vector = struct_vector.child(3, input.len());          // parent: VARCHAR
        let name_vector = struct_vector.child(4, input.len());            // name: VARCHAR
        let stem_vector = struct_vector.child(5, input.len());            // stem: VARCHAR
        let suffix_vector = struct_vector.child(6, input.len());          // suffix: VARCHAR
        let mut suffixes_list_vector = struct_vector.list_vector_child(7);  // suffixes: LIST<VARCHAR>
        let mut parts_list_vector = struct_vector.list_vector_child(8);     // parts: LIST<VARCHAR>
        let mut is_absolute_vector = struct_vector.child(9, input.len()); // is_absolute: BOOLEAN
        
        // Get raw data slice for boolean field
        let is_absolute_data = is_absolute_vector.as_mut_slice::<bool>();
        
        // First pass: collect all parsed components
        let mut all_components = Vec::new();
        let mut total_suffixes = 0;
        let mut total_parts = 0;
        
        for i in 0..input.len() {
            let mut path_duck_string = input_data[i];
            let path_str = DuckString::new(&mut path_duck_string).as_str();
            
            match parse_path_components(&path_str) {
                Ok(components) => {
                    total_suffixes += components.suffixes.len();
                    total_parts += components.parts.len();
                    all_components.push(Some(components));
                }
                Err(_) => {
                    all_components.push(None);
                }
            }
        }
        
        // Get child vectors for LIST fields with proper capacity
        let suffixes_child_vector = suffixes_list_vector.child(total_suffixes);
        let parts_child_vector = parts_list_vector.child(total_parts);
        
        // Second pass: populate all vectors
        let mut suffixes_offset = 0;
        let mut parts_offset = 0;
        
        for (i, components_opt) in all_components.iter().enumerate() {
            match components_opt {
                Some(components) => {
                    // Set scalar fields
                    drive_vector.insert(i, components.drive.as_str());
                    root_vector.insert(i, components.root.as_str());
                    anchor_vector.insert(i, components.anchor.as_str());
                    parent_vector.insert(i, components.parent.as_str());
                    name_vector.insert(i, components.name.as_str());
                    stem_vector.insert(i, components.stem.as_str());
                    suffix_vector.insert(i, components.suffix.as_str());
                    is_absolute_data[i] = components.is_absolute;
                    
                    // Populate suffixes LIST
                    for (j, suffix) in components.suffixes.iter().enumerate() {
                        suffixes_child_vector.insert(suffixes_offset + j, suffix.as_str());
                    }
                    suffixes_list_vector.set_entry(i, suffixes_offset, components.suffixes.len());
                    suffixes_offset += components.suffixes.len();
                    
                    // Populate parts LIST
                    for (j, part) in components.parts.iter().enumerate() {
                        parts_child_vector.insert(parts_offset + j, part.as_str());
                    }
                    parts_list_vector.set_entry(i, parts_offset, components.parts.len());
                    parts_offset += components.parts.len();
                }
                None => {
                    // Set entire struct row as NULL for truly invalid input
                    struct_vector.set_null(i);
                }
            }
        }
        
        // Set total lengths for LIST vectors
        suffixes_list_vector.set_len(total_suffixes);
        parts_list_vector.set_len(total_parts);
        
        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        // Create LIST<VARCHAR> type for suffixes and parts
        let varchar_type = LogicalTypeHandle::from(LogicalTypeId::Varchar);
        let list_varchar_type_1 = LogicalTypeHandle::list(&varchar_type);
        let list_varchar_type_2 = LogicalTypeHandle::list(&varchar_type);
        
        // Create STRUCT return type with named fields
        let struct_type = LogicalTypeHandle::struct_type(&[
            ("drive", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("root", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("anchor", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("parent", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("name", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("stem", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("suffix", LogicalTypeHandle::from(LogicalTypeId::Varchar)),
            ("suffixes", list_varchar_type_1),
            ("parts", list_varchar_type_2),
            ("is_absolute", LogicalTypeHandle::from(LogicalTypeId::Boolean)),
        ]);
        
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            struct_type,
        )]
    }
}

#[derive(Debug)]
struct PathComponents {
    drive: String,
    root: String,
    anchor: String,
    parent: String,
    name: String,
    stem: String,
    suffix: String,
    suffixes: Vec<String>,
    parts: Vec<String>,
    is_absolute: bool,
}

fn parse_path_components(path: &str) -> Result<PathComponents, Box<dyn std::error::Error>> {
    // Handle empty string
    if path.is_empty() {
        return Ok(PathComponents {
            drive: String::new(),
            root: String::new(),
            anchor: String::new(),
            parent: String::new(),
            name: String::new(),
            stem: String::new(),
            suffix: String::new(),
            suffixes: Vec::new(),
            parts: Vec::new(),
            is_absolute: false,
        });
    }
    
    // Determine drive and root (cross-platform)
    let (drive, root, rest) = parse_drive_and_root(path);
    let anchor = format!("{}{}", drive, root);
    let is_absolute = !root.is_empty();
    
    // Split remaining path into parts
    let parts: Vec<String> = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split(['/', '\\']).filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
    };
    
    // Get name (last component)
    let name = parts.last().cloned().unwrap_or_default();
    
    // Get parent (all parts except last, joined back)
    let parent = if parts.len() > 1 {
        format!("{}{}", anchor, parts[..parts.len()-1].join("/"))
    } else if !anchor.is_empty() && !parts.is_empty() {
        anchor.clone()
    } else {
        String::new()
    };
    
    // Parse name into stem and suffixes
    let (stem, suffix, suffixes) = parse_name_components(&name);
    
    Ok(PathComponents {
        drive,
        root,
        anchor,
        parent,
        name,
        stem,
        suffix,
        suffixes,
        parts,
        is_absolute,
    })
}

fn parse_drive_and_root(path: &str) -> (String, String, String) {
    #[cfg(windows)]
    {
        // Windows: Check for drive letter (C:)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path[..2].to_string();
            if path.len() > 2 && (path.chars().nth(2) == Some('\\') || path.chars().nth(2) == Some('/')) {
                let root = path.chars().nth(2).unwrap().to_string();
                let rest = if path.len() > 3 { &path[3..] } else { "" };
                return (drive, root, rest.to_string());
            } else {
                let rest = if path.len() > 2 { &path[2..] } else { "" };
                return (drive, String::new(), rest.to_string());
            }
        }
    }
    
    // POSIX or Windows without drive: Check for leading separator
    if path.starts_with('/') || path.starts_with('\\') {
        let root = path.chars().next().unwrap().to_string();
        let rest = if path.len() > 1 { &path[1..] } else { "" };
        (String::new(), root, rest.to_string())
    } else {
        (String::new(), String::new(), path.to_string())
    }
}

fn parse_name_components(name: &str) -> (String, String, Vec<String>) {
    if name.is_empty() {
        return (String::new(), String::new(), Vec::new());
    }
    
    // Find all dot positions (excluding leading dot for hidden files)
    let mut dot_positions = Vec::new();
    let chars: Vec<char> = name.chars().collect();
    
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '.' && i > 0 { // Skip leading dot
            dot_positions.push(i);
        }
    }
    
    if dot_positions.is_empty() {
        // No extensions
        return (name.to_string(), String::new(), Vec::new());
    }
    
    // Get last suffix (from last dot to end)
    let last_dot = *dot_positions.last().unwrap();
    let suffix = name[last_dot..].to_string();
    
    // Get stem (from start to last dot)
    let stem = name[..last_dot].to_string();
    
    // Get all suffixes: each extension from each dot position to the next
    let mut suffixes = Vec::new();
    for i in 0..dot_positions.len() {
        let start_pos = dot_positions[i];
        let end_pos = if i + 1 < dot_positions.len() {
            dot_positions[i + 1]
        } else {
            name.len()
        };
        suffixes.push(name[start_pos..end_pos].to_string());
    }
    
    (stem, suffix, suffixes)
}

fn compute_file_sha256(filename: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let path = Path::new(filename);
    
    match compute_file_hash_streaming(path) {
        Ok(hash) => Ok(Some(hash)),
        Err(e) => {
            use std::io::ErrorKind;
            if let Some(io_error) = e.downcast_ref::<std::io::Error>() {
                match io_error.kind() {
                    ErrorKind::NotFound => Ok(None), // File doesn't exist -> return NULL
                    ErrorKind::PermissionDenied => Ok(None), // Permission error -> return NULL
                    _ => Err(e), // Other errors -> return error
                }
            } else {
                Err(e) // Non-IO errors -> return error
            }
        }
    }
}

fn get_file_metadata_struct(filename: &str) -> Result<Option<FileMetadata>, Box<dyn std::error::Error>> {
    let path = Path::new(filename);
    
    match fs::metadata(path) {
        Ok(metadata) => {
            // Successfully got metadata, create FileMetadata struct
            let file_meta = FileMetadata {
                path: filename.to_string(),
                size: metadata.len(),
                modified_time: system_time_to_microseconds(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
                accessed_time: system_time_to_microseconds(metadata.accessed().unwrap_or(SystemTime::UNIX_EPOCH)),
                created_time: system_time_to_microseconds(metadata.created().unwrap_or(SystemTime::UNIX_EPOCH)),
                permissions: format_permissions(&metadata),
                inode: get_inode(&metadata),
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                is_symlink: metadata.file_type().is_symlink(),
                hash: None, // Not needed for this function
            };
            Ok(Some(file_meta))
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

// Streaming SHA256 computation with adaptive chunk sizes
fn compute_file_hash_streaming(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    
    // Adaptive chunk strategy: 1MB -> 2MB -> 4MB -> 8MB max
    let mut chunk_size = 1024 * 1024; // Start with 1MB
    const MAX_CHUNK_SIZE: usize = 8 * 1024 * 1024; // Max 8MB
    
    loop {
        let mut buffer = vec![0u8; chunk_size];
        let bytes_read = file.read(&mut buffer)?;
        
        if bytes_read == 0 {
            break; // EOF
        }
        
        // Update hasher with the data we actually read
        hasher.update(&buffer[..bytes_read]);
        
        // Double chunk size for next read (up to max)
        if chunk_size < MAX_CHUNK_SIZE {
            chunk_size = std::cmp::min(chunk_size * 2, MAX_CHUNK_SIZE);
        }
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

// Legacy function kept for compatibility (not used anymore)
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
    
    con.register_scalar_function::<FileSha256Scalar>("file_sha256")
        .expect("Failed to register file_sha256 scalar function");
    
    con.register_scalar_function::<PathPartsScalar>("path_parts")
        .expect("Failed to register path_parts scalar function");
    
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
        
        let files1 = collect_files_with_duckdb_glob(pattern1).unwrap_or_default();
        let files2 = collect_files_with_duckdb_glob(pattern2).unwrap_or_default();
        
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

    #[test]
    fn test_streaming_file_hash() {
        // Test streaming hash computation
        let result = compute_file_hash_streaming(Path::new("Cargo.toml"));
        assert!(result.is_ok(), "Should successfully compute hash for existing file");
        
        let hash = result.unwrap();
        assert_eq!(hash.len(), 64, "SHA256 hash should be 64 characters long");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), 
                "Hash should contain only hex digits");
        assert!(hash.chars().all(|c| !c.is_uppercase()), 
                "Hash should be lowercase");
        
        // Test file_sha256 helper function
        let result = compute_file_sha256("Cargo.toml");
        assert!(result.is_ok(), "Should successfully process existing file");
        assert!(result.unwrap().is_some(), "Should return Some for existing file");
        
        // Test non-existent file
        let result = compute_file_sha256("nonexistent_file.txt");
        assert!(result.is_ok(), "Should handle non-existent file gracefully");
        assert!(result.unwrap().is_none(), "Should return None for non-existent file");
    }

    #[test]
    fn test_path_parsing() {
        // Test basic path parsing functionality
        let result = parse_path_components("archive.tar.gz");
        assert!(result.is_ok(), "Should successfully parse simple filename");
        
        let components = result.unwrap();
        assert_eq!(components.name, "archive.tar.gz");
        assert_eq!(components.stem, "archive.tar");
        assert_eq!(components.suffix, ".gz");
        assert_eq!(components.suffixes, vec![".tar", ".gz"]);
        assert!(!components.is_absolute);
        
        // Test absolute path
        let result = parse_path_components("/home/user/file.txt");
        assert!(result.is_ok(), "Should successfully parse absolute path");
        
        let components = result.unwrap();
        assert_eq!(components.name, "file.txt");
        assert_eq!(components.suffix, ".txt");
        assert_eq!(components.suffixes, vec![".txt"]);
        assert_eq!(components.parts, vec!["home", "user", "file.txt"]);
        assert_eq!(components.root, "/");
        assert!(components.is_absolute);
        
        // Test empty string
        let result = parse_path_components("");
        assert!(result.is_ok(), "Should handle empty string");
        let components = result.unwrap();
        assert_eq!(components.name, "");
        assert!(!components.is_absolute);
    }
}