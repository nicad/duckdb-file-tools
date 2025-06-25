use std::sync::atomic::{AtomicUsize, Ordering};
use std::ffi::CString;

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, InitInfo, TableFunctionInfo, VTab},
};

use crate::metadata::FileMetadata;

#[repr(C)]
pub struct GlobStatBindData {
    pattern: String,
    hash_algorithm: Option<String>,
    files: Vec<FileMetadata>,
}

#[repr(C)]
pub struct GlobStatInitData {
    current_index: AtomicUsize,
}

pub struct GlobStatVTab;

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
            let hash_param = bind.get_parameter(1).to_string();
            if hash_param.to_lowercase() == "sha256" {
                Some(hash_param)
            } else {
                None
            }
        } else {
            None
        };

        let files = crate::metadata::collect_file_metadata(&pattern, hash_algorithm.as_deref())?;

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

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        let current_idx = init_data.current_index.load(Ordering::Relaxed);
        
        if current_idx >= bind_data.files.len() {
            output.set_len(0);
            return Ok(());
        }

        let file_meta = &bind_data.files[current_idx];
        
        // Insert all data as strings for now
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