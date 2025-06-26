# File Tools Extension Functions

The File Tools extension provides functions for file system operations, metadata extraction, and path manipulation in DuckDB.

## Table Functions

### `glob_stat(pattern)`

Scans files matching a glob pattern and returns metadata for each file.

**Syntax**
```sql
SELECT * FROM glob_stat(pattern)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files (e.g., `'*.txt'`, `'data/**/*.csv'`)

**Returns**
A table with the following columns:
- `path` (`VARCHAR`): Full path to the file
- `size` (`VARCHAR`): File size in bytes
- `modified_time` (`VARCHAR`): Last modification time
- `accessed_time` (`VARCHAR`): Last access time  
- `created_time` (`VARCHAR`): Creation time
- `permissions` (`VARCHAR`): File permissions
- `inode` (`VARCHAR`): File inode number
- `is_file` (`VARCHAR`): Whether the entry is a file
- `is_dir` (`VARCHAR`): Whether the entry is a directory
- `is_symlink` (`VARCHAR`): Whether the entry is a symbolic link

**Example**
```sql
-- List all CSV files in the current directory with metadata
SELECT path, size, modified_time 
FROM glob_stat('*.csv');

-- Find large files recursively
SELECT path, size 
FROM glob_stat('**/*') 
WHERE CAST(size AS BIGINT) > 1000000;
```

### `file_path_sha256(pattern)`

Scans files matching a glob pattern and computes SHA256 hashes along with metadata.

**Syntax**
```sql
SELECT * FROM file_path_sha256(pattern)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files

**Returns**
Same columns as `glob_stat` plus:
- `hash` (`VARCHAR`): SHA256 hash of the file contents (lowercase hex)

**Example**
```sql
-- Generate SHA256 hashes for all files
SELECT path, hash 
FROM file_path_sha256('*');

-- Find duplicate files by hash
SELECT hash, array_agg(path) as duplicate_files
FROM file_path_sha256('**/*')
WHERE is_file = 'true'
GROUP BY hash
HAVING count(*) > 1;
```

### `glob_stat_sha256_parallel(pattern)`

**High-performance parallel version** of file scanning with SHA256 hash computation. Uses multi-threading to dramatically improve performance on large directories.

**Syntax**
```sql
SELECT * FROM glob_stat_sha256_parallel(pattern)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files

**Returns**
Same columns as `file_path_sha256`:
- `path` (`VARCHAR`): Full path to the file
- `size` (`VARCHAR`): File size in bytes
- `modified_time` (`VARCHAR`): Last modification time
- `accessed_time` (`VARCHAR`): Last access time  
- `created_time` (`VARCHAR`): Creation time
- `permissions` (`VARCHAR`): File permissions
- `inode` (`VARCHAR`): File inode number
- `is_file` (`VARCHAR`): Whether the entry is a file
- `is_dir` (`VARCHAR`): Whether the entry is a directory
- `is_symlink` (`VARCHAR`): Whether the entry is a symbolic link
- `hash` (`VARCHAR`): SHA256 hash of the file contents (lowercase hex)

**Performance Features**
- **Multi-threaded hash computation**: Uses `rayon` to compute hashes on multiple CPU cores simultaneously
- **Parallel metadata extraction**: File metadata and hash computation runs in parallel across all CPU cores
- **Same glob patterns**: Uses identical pattern matching as `glob_stat` but with parallel processing
- **Memory efficient**: Streaming hash computation prevents memory issues with large files
- **Lock-free design**: Minimizes thread contention for maximum throughput

**When to Use**
- **Large directories**: Hundreds or thousands of files
- **Performance critical**: When speed is more important than resource usage
- **Batch processing**: Creating file manifests, backup verification, etc.
- **Multi-core systems**: Best performance on systems with multiple CPU cores

**Example**
```sql
-- Fast hash computation for large directories
SELECT path, hash 
FROM glob_stat_sha256_parallel('large_dataset/**/*');

-- Performance comparison with regular version
-- (Use this for large directories, use file_path_sha256 for small ones)
SELECT COUNT(*) as file_count, 'parallel' as method
FROM glob_stat_sha256_parallel('**/*.log')
UNION ALL
SELECT COUNT(*) as file_count, 'sequential' as method  
FROM file_path_sha256('**/*.log');

-- Create file integrity manifest quickly
CREATE TABLE backup_manifest AS
SELECT 
    path,
    hash,
    size,
    modified_time
FROM glob_stat_sha256_parallel('/backup/data/**/*')
WHERE is_file = 'true';
```

## Scalar Functions

### `file_stat(filename)`

Returns detailed metadata for a single file as a struct.

**Syntax**
```sql
file_stat(filename)
```

**Parameters**
- `filename` (`VARCHAR`): Path to the file

**Returns**
`STRUCT` with the following fields:
- `size` (`BIGINT`): File size in bytes
- `modified_time` (`TIMESTAMP`): Last modification time as timestamp
- `accessed_time` (`TIMESTAMP`): Last access time as timestamp
- `created_time` (`TIMESTAMP`): Creation time as timestamp
- `permissions` (`VARCHAR`): File permissions string
- `inode` (`BIGINT`): File inode number
- `is_file` (`BOOLEAN`): Whether the entry is a file
- `is_dir` (`BOOLEAN`): Whether the entry is a directory
- `is_symlink` (`BOOLEAN`): Whether the entry is a symbolic link

**Error Handling**
- Returns `NULL` if file doesn't exist or permission denied
- Throws error for other I/O errors

**Example**
```sql
-- Get metadata for a specific file
SELECT file_stat('data.csv') AS metadata;

-- Access individual fields using dot notation
SELECT 
    file_stat('data.csv').size AS file_size,
    file_stat('data.csv').modified_time AS last_modified,
    file_stat('data.csv').is_file AS is_regular_file;

-- Filter files by modification time
SELECT path 
FROM glob_stat('*') 
WHERE file_stat(path).modified_time > '2024-01-01'::TIMESTAMP;
```

### `file_sha256(filename)`

Computes SHA256 hash of a file using streaming algorithm for memory efficiency.

**Syntax**
```sql
file_sha256(filename)
```

**Parameters**
- `filename` (`VARCHAR`): Path to the file

**Returns**
- `VARCHAR`: SHA256 hash as lowercase hexadecimal string

**Features**
- **Streaming computation**: Uses adaptive chunk sizes (1MB→2MB→4MB→8MB) for memory efficiency
- **Large file support**: Can handle files larger than available RAM
- **Error handling**: Returns `NULL` for missing files, errors for I/O issues

**Example**
```sql
-- Get SHA256 hash of a file
SELECT file_sha256('important_document.pdf') AS hash;

-- Compare file hashes
SELECT 
    'file1.txt' AS file,
    file_sha256('file1.txt') AS hash
UNION ALL
SELECT 
    'file2.txt' AS file,
    file_sha256('file2.txt') AS hash;

-- Verify file integrity
SELECT 
    filename,
    file_sha256(filename) AS current_hash,
    expected_hash,
    file_sha256(filename) = expected_hash AS is_valid
FROM file_integrity_table;
```

### `path_parts(path)`

Decomposes a file path into its constituent components with cross-platform support.

**Syntax**
```sql
path_parts(path)
```

**Parameters**
- `path` (`VARCHAR`): File system path to decompose

**Returns**
`STRUCT` with the following fields:
- `drive` (`VARCHAR`): Drive letter (Windows) or empty string (Unix)
- `root` (`VARCHAR`): Root separator (`/` or `\`) or empty for relative paths
- `anchor` (`VARCHAR`): Combination of drive and root
- `parent` (`VARCHAR`): Parent directory path
- `name` (`VARCHAR`): Final component (filename with extension)
- `stem` (`VARCHAR`): Filename without extension
- `suffix` (`VARCHAR`): File extension including the dot
- `suffixes` (`LIST<VARCHAR>`): All file extensions as a list
- `parts` (`LIST<VARCHAR>`): All path components as a list
- `is_absolute` (`BOOLEAN`): Whether the path is absolute

**Platform Support**
- **Windows**: Handles drive letters (`C:\path\file.txt`)
- **Unix/Linux/macOS**: Standard Unix paths (`/path/file.txt`)
- **Cross-platform**: Handles both forward and back slashes

**Example**
```sql
-- Decompose a path
SELECT path_parts('/home/user/document.tar.gz') AS parts;

-- Access specific components
SELECT 
    path_parts('/home/user/document.tar.gz').name AS filename,
    path_parts('/home/user/document.tar.gz').stem AS basename,
    path_parts('/home/user/document.tar.gz').suffix AS extension,
    path_parts('/home/user/document.tar.gz').suffixes AS all_extensions;

-- Extract file extensions from a list of paths
SELECT 
    path,
    path_parts(path).suffix AS extension
FROM (VALUES 
    ('file.txt'),
    ('archive.tar.gz'),
    ('script.py')
) AS t(path);

-- Group files by extension
SELECT 
    path_parts(path).suffix AS extension,
    count(*) AS file_count
FROM glob_stat('**/*')
WHERE path_parts(path).suffix != ''
GROUP BY extension
ORDER BY file_count DESC;

-- Work with path components
SELECT 
    path,
    array_length(path_parts(path).parts) AS depth,
    path_parts(path).parts[1] AS first_component
FROM (VALUES 
    ('/usr/local/bin/python'),
    ('C:\Windows\System32\cmd.exe'),
    ('relative/path/file.txt')
) AS t(path);
```

### `blob_substr(blob_data, start, length)`

Extracts a substring from BLOB data, similar to the built-in `substr` function but for binary data.

**Syntax**
```sql
blob_substr(blob_data, start, length)
```

**Parameters**
- `blob_data` (`BLOB`): The source BLOB data
- `start` (`BIGINT`): Starting position (1-based indexing)
- `length` (`BIGINT`): Number of bytes to extract

**Returns**
- `BLOB`: Extracted substring as BLOB data

**Behavior**
- **1-based indexing**: Position 1 is the first byte (like SQL `substr`)
- **Bounds checking**: Returns empty BLOB if start position is beyond data
- **Negative length**: Takes all remaining bytes from start position
- **Zero length**: Returns empty BLOB

**Example**
```sql
-- Extract bytes from BLOB
SELECT blob_substr('ABCDEF'::BLOB, 2, 3) AS result;  -- Returns 'BCD'

-- Extract first byte
SELECT blob_substr('ABCDEF'::BLOB, 1, 1) AS first_byte;

-- Extract from position to end (negative length)
SELECT blob_substr('ABCDEF'::BLOB, 3, -1) AS from_third;  -- Returns 'CDEF'

-- Work with binary data
SELECT blob_substr(file_data, 1, 4) AS magic_bytes
FROM binary_files;

-- Extract header information
SELECT 
    filename,
    blob_substr(content, 1, 8) AS header,
    octet_length(content) AS total_size
FROM file_contents;
```

## Usage Patterns

### File Integrity Checking
```sql
-- Create a manifest of file hashes (fast parallel version)
CREATE TABLE file_manifest AS
SELECT 
    path,
    hash,
    CAST(size AS BIGINT) AS size,
    modified_time AS last_modified
FROM glob_stat_sha256_parallel('important_files/**/*')
WHERE is_file = 'true';

-- Later, verify integrity (sequential for individual files)
SELECT 
    path,
    hash AS stored_hash,
    file_sha256(path) AS current_hash,
    hash = file_sha256(path) AS is_valid
FROM file_manifest;

-- Alternative: Create manifest using sequential method (for small datasets)
CREATE TABLE small_file_manifest AS
SELECT 
    path,
    file_sha256(path) AS hash,
    file_stat(path).size AS size,
    file_stat(path).modified_time AS last_modified
FROM glob_stat('small_dataset/**/*')
WHERE file_stat(path).is_file;
```

### File Organization Analysis
```sql
-- Analyze file types and sizes by directory
SELECT 
    path_parts(path).parent AS directory,
    path_parts(path).suffix AS extension,
    count(*) AS file_count,
    sum(CAST(size AS BIGINT)) AS total_size
FROM glob_stat('**/*')
WHERE is_file = 'true'
GROUP BY directory, extension
ORDER BY total_size DESC;
```

### Duplicate File Detection
```sql
-- Find files with identical content (fast parallel version)
WITH file_hashes AS (
    SELECT 
        path,
        hash,
        CAST(size AS BIGINT) AS size
    FROM glob_stat_sha256_parallel('**/*')
    WHERE is_file = 'true' AND size != '0'
)
SELECT 
    hash,
    size,
    array_agg(path) AS duplicate_files,
    count(*) AS duplicate_count
FROM file_hashes
WHERE hash IS NOT NULL AND hash != ''
GROUP BY hash, size
HAVING count(*) > 1
ORDER BY duplicate_count DESC, size DESC;

-- Alternative: Sequential version for smaller datasets
WITH file_hashes_sequential AS (
    SELECT 
        path,
        file_sha256(path) AS hash,
        CAST(size AS BIGINT) AS size
    FROM glob_stat('**/*')
    WHERE is_file = 'true' AND size != '0'
)
SELECT 
    hash,
    size,
    array_agg(path) AS duplicate_files,
    count(*) AS duplicate_count
FROM file_hashes_sequential
WHERE hash IS NOT NULL
GROUP BY hash, size
HAVING count(*) > 1
ORDER BY duplicate_count DESC, size DESC;
```

### Binary File Analysis
```sql
-- Analyze file headers to identify file types
SELECT 
    path,
    blob_substr(read_blob(path), 1, 4) AS magic_bytes,
    CASE 
        WHEN blob_substr(read_blob(path), 1, 4) = '\x89PNG'::BLOB THEN 'PNG'
        WHEN blob_substr(read_blob(path), 1, 3) = 'PDF'::BLOB THEN 'PDF'
        WHEN blob_substr(read_blob(path), 1, 2) = '\xFF\xD8'::BLOB THEN 'JPEG'
        ELSE 'Unknown'
    END AS detected_type,
    path_parts(path).suffix AS extension
FROM glob_stat('files/*')
WHERE file_stat(path).is_file;
```