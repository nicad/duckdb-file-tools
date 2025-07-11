# File Tools Extension Functions

The File Tools extension provides functions for file system operations, metadata extraction, and path manipulation in DuckDB.

## Latest Changes

**v0.1.0 - Advanced Glob Options & Multi-Algorithm Compression**
- ✅ **Universal named parameters**: All `glob_stat*` functions now support `ignore_case`, `follow_symlinks`, and `exclude` parameters
- ✅ **Advanced glob options**: Added optional named parameters to `glob_stat()`, `glob_stat_sha256_parallel()`, and `glob_stat_sha256_jwalk()`
- ✅ **Exclude patterns**: Support for glob-style exclude patterns using array syntax (e.g., `exclude := ['*.tmp', '*.bak', 'node_modules/']`)
- ✅ **Symlink control**: Configure whether to follow symbolic links with `follow_symlinks` parameter
- ✅ **Case-insensitive glob matching**: Added `ignore_case` parameter for case-insensitive pattern matching
- ✅ **Compression functions**: Added `compress()`, `compress_zstd()`, `compress_lz4()` and `decompress()` functions
- ✅ **Runtime debug output**: Set `DUCKDB_FILE_TOOLS_DEBUG=1` to enable detailed performance instrumentation
- ✅ **New jwalk implementation**: Added `glob_stat_sha256_jwalk` as alternative parallel implementation
- ✅ **Pattern matching fixes**: Fixed file count discrepancies in jwalk implementation
- ✅ **Release builds available**: Both debug and optimized release builds with debug instrumentation
- ✅ **Performance improvements**: Release builds show ~31% performance improvement over debug builds

**Available Builds:**
- **Debug**: `./build/debug/file_tools.duckdb_extension` - Full debug symbols, detailed error messages
- **Release**: `./build/release/file_tools.duckdb_extension` - Optimized performance, production-ready

Both builds support runtime debug output via `DUCKDB_FILE_TOOLS_DEBUG=1` environment variable.

## Table Functions

### `glob_stat(pattern, ignore_case, follow_symlinks, exclude)`

Scans files matching a glob pattern and returns metadata for each file. Supports optional named parameters for advanced filtering and control.

**Syntax**
```sql
SELECT * FROM glob_stat(
    pattern,
    ignore_case := false,
    follow_symlinks := true,
    exclude := []
)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files (e.g., `'*.txt'`, `'data/**/*.csv'`)
- `ignore_case` (`BOOLEAN`, optional): Whether to perform case-insensitive pattern matching (default: `false`)
- `follow_symlinks` (`BOOLEAN`, optional): Whether to follow symbolic links (default: `true`) 
- `exclude` (`LIST(VARCHAR)`, optional): Array of glob patterns to exclude from results (default: `[]`)

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

**Examples**
```sql
-- Simple usage - list all CSV files with default settings
SELECT path, size, modified_time 
FROM glob_stat('*.csv');

-- Find large files recursively
SELECT path, size 
FROM glob_stat('**/*') 
WHERE CAST(size AS BIGINT) > 1000000;

-- Basic file listing with metadata
SELECT path, is_file, is_dir 
FROM glob_stat('data/**/*');

-- Case-insensitive matching (.txt, .TXT, .Txt, etc.)
SELECT path FROM glob_stat('**/*.txt', ignore_case := true);

-- Exclude temporary and backup files
SELECT path FROM glob_stat(
    '**/*', 
    exclude := ['*.tmp', '*.bak', '*.log', '.git/', 'node_modules/']
);

-- Don't follow symbolic links
SELECT path, is_symlink FROM glob_stat(
    '/data/**/*', 
    follow_symlinks := false
);

-- Complex example: case-insensitive, no symlinks, exclude patterns
SELECT path, size FROM glob_stat(
    'projects/**/*.{TXT,CSV}',
    ignore_case := true,
    follow_symlinks := false,
    exclude := ['temp/', '*.tmp', '.git/', 'node_modules/', '*.log']
) WHERE CAST(size AS BIGINT) > 1000;

-- Compare with and without excludes
SELECT 'all files' as category, count(*) as file_count
FROM glob_stat('**/*')
UNION ALL
SELECT 'filtered files' as category, count(*) as file_count  
FROM glob_stat('**/*', exclude := ['*.tmp', '.git/', 'node_modules/']);
```

### `glob_stat_legacy(pattern)`

A simpler version of `glob_stat()` without optional parameters, useful for basic file listing and testing.

**Syntax**
```sql
SELECT * FROM glob_stat_legacy(pattern)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files

**Returns**
Same columns as `glob_stat()` with default behavior (case-sensitive, follows symlinks, no excludes).

**Examples**
```sql
-- Simple glob_stat_legacy for testing without options
SELECT path, size FROM glob_stat_legacy('*.csv');

-- Legacy function without advanced features
SELECT COUNT(*) FROM glob_stat_legacy('**/*.txt');
```

**Exclude Pattern Features**
- **File extensions**: `'*.tmp'`, `'*.log'`, `'*.{jpg,png,gif}'`
- **Directories**: `'temp/'`, `'.git/'`, `'node_modules/'` (trailing slash recommended)
- **Path patterns**: `'**/temp/*'`, `'build/**'`, `'target/'`
- **Common excludes**: `['.git/', '.svn/', 'node_modules/', 'target/', '*.tmp', '*.log', '*.bak']`

**Symlink Behavior**
- `follow_symlinks := true` (default): Follows symlinks and reports target file metadata
- `follow_symlinks := false`: Excludes symlinks from results entirely

**Performance Notes**
- Exclude patterns use compiled glob matching for efficiency
- Directory-level excludes (e.g., `'temp/'`) skip entire subtrees for better performance
- Case-insensitive matching may be slower on large datasets


### `glob_stat_sha256_parallel(pattern, ignore_case, follow_symlinks, exclude)`

**High-performance parallel version** of file scanning with SHA256 hash computation. Uses multi-threading to dramatically improve performance on large directories. Supports the same optional named parameters as `glob_stat()`.

**Syntax**
```sql
SELECT * FROM glob_stat_sha256_parallel(
    pattern,
    ignore_case := false,
    follow_symlinks := true,
    exclude := []
)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files
- `ignore_case` (`BOOLEAN`, optional): Whether to perform case-insensitive pattern matching (default: `false`)
- `follow_symlinks` (`BOOLEAN`, optional): Whether to follow symbolic links (default: `true`) 
- `exclude` (`LIST(VARCHAR)`, optional): Array of glob patterns to exclude from results (default: `[]`)

**Returns**
Returns the following columns:
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

**Examples**
```sql
-- Fast hash computation for large directories
SELECT path, hash 
FROM glob_stat_sha256_parallel('large_dataset/**/*');

-- Case-insensitive hash computation with excludes
SELECT path, hash
FROM glob_stat_sha256_parallel(
    '**/*.{TXT,CSV}',
    ignore_case := true,
    exclude := ['*.tmp', '.git/', 'node_modules/']
);

-- Performance comparison with different implementations
SELECT COUNT(*) as file_count, 'parallel' as method
FROM glob_stat_sha256_parallel('**/*.log')
UNION ALL
SELECT COUNT(*) as file_count, 'jwalk' as method  
FROM glob_stat_sha256_jwalk('**/*.log');

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

### `glob_stat_sha256_jwalk(pattern, ignore_case, follow_symlinks, exclude)`

**Alternative parallel implementation** using the `jwalk` crate for directory traversal. Provides identical results to `glob_stat_sha256_parallel` but with different internal implementation for comparison and testing. Supports the same optional named parameters as other glob_stat functions.

**Syntax**
```sql
SELECT * FROM glob_stat_sha256_jwalk(
    pattern,
    ignore_case := false,
    follow_symlinks := true,
    exclude := []
)
```

**Parameters**
- `pattern` (`VARCHAR`): A glob pattern to match files
- `ignore_case` (`BOOLEAN`, optional): Whether to perform case-insensitive pattern matching (default: `false`)
- `follow_symlinks` (`BOOLEAN`, optional): Whether to follow symbolic links (default: `true`) 
- `exclude` (`LIST(VARCHAR)`, optional): Array of glob patterns to exclude from results (default: `[]`)

**Returns**
Same columns as the parallel implementation:
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

**Implementation Details**
- Uses `jwalk` for directory walking, then applies glob pattern matching
- Falls back to glob crate results for pattern matching accuracy
- Provides identical results to `glob_stat_sha256_parallel`
- Useful for performance testing and comparison with different directory traversal strategies

**When to Use**
- **Testing and comparison**: Compare performance against `glob_stat_sha256_parallel`
- **Alternative implementation**: When glob-based traversal has issues
- **Development**: For testing different directory walking approaches

**Examples**
```sql
-- Basic usage with pattern
SELECT path, hash
FROM glob_stat_sha256_jwalk('data/**/*.txt');

-- Advanced usage with named parameters
SELECT path, hash, size
FROM glob_stat_sha256_jwalk(
    '**/*.{JPG,PNG}',
    ignore_case := true,
    follow_symlinks := false,
    exclude := ['thumbnails/', '*.tmp']
);

-- Compare implementations on same directory
SELECT 'jwalk' as method, COUNT(*) as file_count, AVG(CAST(size AS BIGINT)) as avg_size
FROM glob_stat_sha256_jwalk('data/**/*')
WHERE is_file = 'true'
UNION ALL
SELECT 'parallel' as method, COUNT(*) as file_count, AVG(CAST(size AS BIGINT)) as avg_size  
FROM glob_stat_sha256_parallel('data/**/*')
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

### `file_read_text(filename)`

Reads the content of a text file and returns it as a VARCHAR string.

**Syntax**
```sql
file_read_text(filename)
```

**Parameters**
- `filename` (`VARCHAR`): Path to the file to read

**Returns**
- `VARCHAR`: The complete file content as text
- `NULL`: If the file doesn't exist or cannot be read

**Error Handling**
- Returns `NULL` for non-existent files (no error thrown)
- Returns `NULL` for permission errors (no error thrown)
- Returns `NULL` for binary files that cannot be decoded as UTF-8

**Comparison with DuckDB Built-ins**
- **DuckDB's `read_text(glob)`**: Table function that reads multiple files matching a glob pattern, returns `(filename, content)` rows
- **This `file_read_text(filename)`**: Scalar function that reads a single specific file, returns content directly

**When to Use**
- **Single file reading**: When you need content of one specific file
- **SELECT expressions**: Can be used in any SELECT expression or WHERE clause
- **Data processing**: When file content needs to be combined with other data

**Example**
```sql
-- Read a configuration file
SELECT file_read_text('config.yaml') AS config;

-- Read multiple specific files
SELECT 
    filename,
    file_read_text(filename) AS content
FROM (VALUES ('file1.txt'), ('file2.txt'), ('config.json')) AS t(filename);

-- Use file content in queries
SELECT 
    'large_file' AS category
WHERE length(file_read_text('data.txt')) > 1000000;

-- Process configuration with other data
SELECT 
    user_id,
    settings,
    file_read_text('default_config.json') AS default_config
FROM user_settings;

-- Handle missing files gracefully
SELECT 
    COALESCE(file_read_text('custom.conf'), file_read_text('default.conf'), '{}') AS config;
```

### `file_read_blob(filename)`

Reads the content of a file as binary data and returns it as a BLOB.

**Syntax**
```sql
file_read_blob(filename)
```

**Parameters**
- `filename` (`VARCHAR`): Path to the file to read

**Returns**
- `BLOB`: The complete file content as binary data
- `NULL`: If the file doesn't exist or cannot be read

**Error Handling**
- Returns `NULL` for non-existent files (no error thrown)
- Returns `NULL` for permission errors (no error thrown)
- Can read any file type (text, binary, images, etc.)

**Comparison with DuckDB Built-ins**
- **DuckDB's `read_blob(glob)`**: Table function that reads multiple files matching a glob pattern, returns `(filename, content)` rows
- **This `file_read_blob(filename)`**: Scalar function that reads a single specific file, returns content directly

**When to Use**
- **Binary file processing**: Images, executables, compressed files, etc.
- **Single file reading**: When you need content of one specific file
- **Data pipelines**: When file content needs processing with other functions
- **File size analysis**: Use with `octet_length()` for size calculations

**Example**
```sql
-- Read a binary file
SELECT file_read_blob('image.png') AS image_data;

-- Check file sizes
SELECT 
    filename,
    octet_length(file_read_blob(filename)) AS size_bytes
FROM (VALUES ('file1.dat'), ('file2.bin')) AS t(filename);

-- Process binary data
SELECT 
    filename,
    substring(file_read_blob(filename), 1, 4) AS file_header
FROM (VALUES ('data.pdf'), ('archive.zip')) AS t(filename);

-- Combine with compression functions
SELECT 
    filename,
    compress(file_read_blob(filename)) AS compressed_data,
    octet_length(file_read_blob(filename)) AS original_size,
    octet_length(compress(file_read_blob(filename))) AS compressed_size
FROM (VALUES ('document.txt'), ('data.csv')) AS t(filename);

-- Copy files using SQL
CREATE TABLE file_backup AS
SELECT 
    'backup_' || filename AS backup_name,
    file_read_blob(filename) AS file_data
FROM (VALUES ('important.dat'), ('config.bin')) AS t(filename);
```

**Key Differences from DuckDB Built-ins**

| Feature | DuckDB Built-in | File Tools Extension |
|---------|----------------|---------------------|
| **Function Type** | Table function | Scalar function |
| **Input** | Glob pattern | Single filename |
| **Output** | Table with (filename, content) | Content directly |
| **Usage** | `FROM read_text('*.txt')` | `SELECT file_read_text('file.txt')` |
| **Multiple files** | Yes (via glob) | No (one file per call) |
| **WHERE clause** | Cannot use in WHERE | Can use in WHERE |
| **SELECT expressions** | Limited | Full expression support |
| **Error handling** | May throw errors | Returns NULL |

**Migration Examples**

```sql
-- DuckDB built-in approach
SELECT filename, content 
FROM read_text('config/*.txt');

-- Equivalent with file_read_text (if you know the filenames)
SELECT 
    filename,
    file_read_text(filename) AS content
FROM (VALUES ('config/app.txt'), ('config/db.txt')) AS t(filename);

-- Or discover files first, then read
SELECT 
    path AS filename,
    file_read_text(path) AS content
FROM glob_stat('config/*.txt')
WHERE is_file = true;
```

### `file_exists(filename)`

Checks if a path exists and is specifically a file (not a directory or other type).

**Syntax**
```sql
file_exists(filename)
```

**Parameters**
- `filename` (`VARCHAR`): Path to check

**Returns**
- `TRUE`: If the path exists and is a regular file
- `FALSE`: If the path doesn't exist
- `NULL`: If the path exists but is not a file (e.g., directory, symlink, device)

**Example**
```sql
-- Check if specific files exist
SELECT 
    filename,
    file_exists(filename) AS is_file
FROM (VALUES 
    ('config.json'),      -- returns TRUE if it's a file
    ('data/'),           -- returns NULL (it's a directory)
    ('missing.txt')      -- returns FALSE (doesn't exist)
) AS t(filename);

-- Only process existing files
SELECT 
    path,
    file_read_text(path) AS content
FROM glob_stat('*.conf')
WHERE file_exists(path) = TRUE;

-- Find missing required files
SELECT filename
FROM (VALUES ('config.json'), ('data.csv'), ('schema.sql')) AS required(filename)
WHERE file_exists(filename) = FALSE;
```

### `path_exists(pathname)`

Checks if a path exists, regardless of its type (file, directory, symlink, etc.).

**Syntax**
```sql
path_exists(pathname)
```

**Parameters**
- `pathname` (`VARCHAR`): Path to check

**Returns**
- `TRUE`: If the path exists (any type)
- `FALSE`: If the path doesn't exist

**Example**
```sql
-- Check if paths exist
SELECT 
    path,
    path_exists(path) AS exists,
    file_exists(path) AS is_file
FROM (VALUES 
    ('config.json'),     -- TRUE, TRUE (if it's a file)
    ('data/'),          -- TRUE, NULL (exists but is directory)
    ('missing.txt')     -- FALSE, FALSE
) AS t(path);

-- Ensure directory exists before reading files
SELECT 
    CASE 
        WHEN path_exists('data/') THEN 'Directory exists'
        ELSE 'Directory missing'
    END AS status;

-- Check multiple paths
SELECT 
    path,
    path_exists(path) AS exists,
    CASE 
        WHEN NOT path_exists(path) THEN 'Missing'
        WHEN file_exists(path) = TRUE THEN 'File'
        WHEN file_exists(path) IS NULL THEN 'Directory/Other'
    END AS type
FROM (VALUES 
    ('/etc/passwd'),
    ('/tmp/'),
    ('/nonexistent')
) AS t(path);
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

### `compress(data)`

Compresses BLOB data using GZIP compression algorithm, reducing storage size and bandwidth requirements.

**Syntax**
```sql
compress(data)
```

**Parameters**
- `data` (`BLOB`): The binary data to compress

**Returns**
- `BLOB`: Compressed data in GZIP format

**Features**
- **GZIP compression**: Uses the widely-supported GZIP format with good compression ratios
- **Memory efficient**: Streams data through compression to handle large BLOBs
- **Cross-platform**: Standard GZIP format compatible with all systems
- **Default compression level**: Balanced speed vs. compression ratio

**Example**
```sql
-- Basic compression
SELECT compress('hello world'::BLOB) AS compressed_data;

-- Measure compression efficiency
SELECT 
    octet_length(data) AS original_size,
    octet_length(compress(data)) AS compressed_size,
    round(100.0 * octet_length(compress(data)) / octet_length(data), 1) AS compression_ratio
FROM (SELECT 'The quick brown fox jumps over the lazy dog'::BLOB AS data);

-- Compress file contents
SELECT 
    filename,
    octet_length(content) AS original_size,
    octet_length(compress(content)) AS compressed_size
FROM (
    SELECT 
        'large_file.txt' AS filename,
        read_blob('large_file.txt') AS content
);

-- Store compressed data in table
CREATE TABLE compressed_files AS
SELECT 
    path,
    compress(read_blob(path)) AS compressed_content,
    octet_length(read_blob(path)) AS original_size,
    octet_length(compress(read_blob(path))) AS compressed_size
FROM glob_stat('data/*.txt')
WHERE is_file = 'true';
```

### `decompress(data)`

Decompresses BLOB data that was compressed with the `compress()` function, automatically detecting the compression format.

**Syntax**
```sql
decompress(data)
```

**Parameters**
- `data` (`BLOB`): The compressed binary data

**Returns**
- `BLOB`: Decompressed original data

**Features**
- **Format auto-detection**: Automatically detects GZIP compression format from data headers
- **Round-trip compatibility**: Perfect reconstruction of original data when used with `compress()`
- **Error handling**: Graceful handling of invalid or corrupted compressed data
- **Memory streaming**: Efficient decompression of large compressed BLOBs

**Example**
```sql
-- Basic round-trip compression/decompression
SELECT decompress(compress('hello world'::BLOB)) = 'hello world'::BLOB AS roundtrip_works;

-- Decompress stored data
SELECT 
    path,
    original_size,
    compressed_size,
    decompress(compressed_content) AS restored_content
FROM compressed_files;

-- Verify data integrity after compression/decompression
WITH test_data AS (
    SELECT repeat('Test data with repetitive patterns. ', 100)::BLOB AS original
)
SELECT 
    original = decompress(compress(original)) AS data_integrity_check,
    octet_length(original) AS original_size,
    octet_length(compress(original)) AS compressed_size,
    round(100.0 * octet_length(compress(original)) / octet_length(original), 1) AS compression_ratio
FROM test_data;

-- Process compressed files
SELECT 
    filename,
    length(decompress(compressed_data)::VARCHAR) AS text_length,
    substr(decompress(compressed_data)::VARCHAR, 1, 50) AS preview
FROM compressed_text_files;
```

### `compress_zstd(data)`

High-performance compression using the ZSTD algorithm, providing excellent compression ratios with fast decompression speeds.

**Syntax**
```sql
compress_zstd(data)
```

**Parameters**
- `data` (`BLOB`): The binary data to compress

**Returns**
- `BLOB`: Compressed data in ZSTD format

**Features**
- **Superior compression**: Best compression ratios among all algorithms
- **Fast decompression**: Optimized for quick data retrieval
- **Modern algorithm**: Facebook's ZSTD, designed for real-time applications
- **Wide compatibility**: Standard format supported across platforms

**Example**
```sql
-- ZSTD compression
SELECT compress_zstd('Large dataset content'::BLOB) AS zstd_compressed;

-- Compare compression efficiency
WITH algorithms AS (
    SELECT 'GZIP' AS algo, compress(data) AS compressed FROM (SELECT repeat('data', 1000)::BLOB AS data)
    UNION ALL
    SELECT 'ZSTD' AS algo, compress_zstd(data) AS compressed FROM (SELECT repeat('data', 1000)::BLOB AS data)
    UNION ALL  
    SELECT 'LZ4' AS algo, compress_lz4(data) AS compressed FROM (SELECT repeat('data', 1000)::BLOB AS data)
)
SELECT 
    algo,
    octet_length(compressed) AS compressed_size,
    round(100.0 * octet_length(compressed) / 4000, 1) AS compression_ratio
FROM algorithms
ORDER BY compressed_size;
```

### `compress_lz4(data)`

Ultra-fast compression using the LZ4 algorithm, prioritizing speed over compression ratio for real-time applications.

**Syntax**
```sql
compress_lz4(data)
```

**Parameters**
- `data` (`BLOB`): The binary data to compress

**Returns**
- `BLOB`: Compressed data in LZ4 format with size-prepended header

**Features**
- **Extreme speed**: Fastest compression and decompression
- **Low CPU usage**: Minimal processing overhead
- **Real-time suitable**: Perfect for high-throughput scenarios
- **Size-prepended format**: Includes original size for efficient decompression

**Example**
```sql
-- LZ4 compression for speed-critical applications
SELECT compress_lz4(large_blob_data) AS lz4_compressed
FROM high_volume_table;

-- Benchmark compression speed vs ratio
WITH test_data AS (
    SELECT 
        'logs' AS data_type,
        string_agg(log_entry, '\n')::BLOB AS data
    FROM application_logs
    WHERE timestamp > current_timestamp - INTERVAL '1 hour'
)
SELECT 
    'GZIP' AS algorithm,
    octet_length(compress(data)) AS size,
    'Standard compression' AS use_case
FROM test_data
UNION ALL
SELECT 
    'ZSTD' AS algorithm, 
    octet_length(compress_zstd(data)) AS size,
    'Best compression' AS use_case
FROM test_data
UNION ALL
SELECT 
    'LZ4' AS algorithm,
    octet_length(compress_lz4(data)) AS size, 
    'Fastest compression' AS use_case
FROM test_data;
```

### Algorithm Comparison

| Algorithm | Compression Ratio | Speed | CPU Usage | Best Use Case |
|-----------|------------------|-------|-----------|---------------|
| **LZ4** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Real-time, cache, streaming |
| **GZIP** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | General purpose, compatibility |
| **ZSTD** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | **Balanced: best overall** |

**Performance Characteristics:**
- **LZ4**: 2-3GB/s compression, 3-4GB/s decompression, ~50% size reduction
- **GZIP**: 30-50MB/s compression, 300-400MB/s decompression, ~70% size reduction  
- **ZSTD**: 100-400MB/s compression, 800-1200MB/s decompression, ~75% size reduction

### Compression Use Cases

**Data Storage Optimization**
```sql
-- Archive old logs with maximum compression (ZSTD)
CREATE TABLE archived_logs AS
SELECT 
    date_trunc('day', timestamp) AS log_date,
    compress_zstd(string_agg(log_entry, '\n' ORDER BY timestamp)::BLOB) AS compressed_logs,
    count(*) AS entry_count
FROM application_logs 
WHERE timestamp < current_date - INTERVAL '30 days'
GROUP BY date_trunc('day', timestamp);

-- Cache frequently accessed data with fast compression (LZ4)
CREATE TABLE cached_reports AS
SELECT 
    report_id,
    compress_lz4(report_data::BLOB) AS cached_data,
    last_accessed
FROM expensive_reports
WHERE access_count > 100;
```

**Algorithm Selection Strategy**
```sql
-- Choose algorithm based on data characteristics and usage pattern
WITH data_classification AS (
    SELECT 
        table_name,
        avg_size,
        access_frequency,
        CASE 
            WHEN access_frequency = 'high' AND avg_size < 1048576 THEN 'LZ4'     -- Fast for small, frequently accessed
            WHEN access_frequency = 'low' AND avg_size > 10485760 THEN 'ZSTD'    -- Maximum compression for large archives
            ELSE 'GZIP'                                                           -- Standard for everything else
        END AS recommended_algorithm
    FROM table_stats
)
SELECT 
    table_name,
    recommended_algorithm,
    CASE recommended_algorithm
        WHEN 'LZ4' THEN 'Real-time access, fast decompression'
        WHEN 'ZSTD' THEN 'Maximum space savings, good speed'
        WHEN 'GZIP' THEN 'Universal compatibility, balanced'
    END AS rationale
FROM data_classification;
```

**Multi-Algorithm Performance Comparison**
```sql
-- Comprehensive comparison across different data types
WITH test_data AS (
    SELECT 'json_config' AS data_type, config_json::BLOB AS data FROM system_settings
    UNION ALL
    SELECT 'log_entries' AS data_type, string_agg(message, '\n')::BLOB AS data FROM app_logs LIMIT 1000
    UNION ALL
    SELECT 'binary_data' AS data_type, file_content AS data FROM uploaded_files WHERE file_type = 'pdf' LIMIT 1
),
compression_results AS (
    SELECT 
        data_type,
        octet_length(data) AS original_size,
        octet_length(compress(data)) AS gzip_size,
        octet_length(compress_zstd(data)) AS zstd_size,
        octet_length(compress_lz4(data)) AS lz4_size
    FROM test_data
)
SELECT 
    data_type,
    original_size,
    gzip_size,
    zstd_size, 
    lz4_size,
    round(100.0 * gzip_size / original_size, 1) AS gzip_ratio,
    round(100.0 * zstd_size / original_size, 1) AS zstd_ratio,
    round(100.0 * lz4_size / original_size, 1) AS lz4_ratio,
    CASE 
        WHEN zstd_size < gzip_size AND zstd_size < lz4_size THEN 'ZSTD'
        WHEN lz4_size < gzip_size * 1.2 THEN 'LZ4'  -- LZ4 if within 20% of best
        ELSE 'GZIP'
    END AS recommended
FROM compression_results
ORDER BY original_size DESC;
```

## Debug and Performance Monitoring

The extension includes runtime debug output to help analyze performance and troubleshoot issues with the parallel functions `glob_stat_sha256_parallel` and `glob_stat_sha256_jwalk`.

### Enabling Debug Output

Set the `DUCKDB_FILE_TOOLS_DEBUG` environment variable to `1` to enable detailed instrumentation:

```bash
# Enable debug output
export DUCKDB_FILE_TOOLS_DEBUG=1
duckdb -unsigned -c "LOAD './file_tools.duckdb_extension'; SELECT count(*) FROM glob_stat_sha256_parallel('data/**');"

# Or enable for a single command
DUCKDB_FILE_TOOLS_DEBUG=1 duckdb -unsigned -c "LOAD './file_tools.duckdb_extension'; SELECT count(*) FROM glob_stat_sha256_parallel('data/**');"
```

### Debug Output Information

When enabled, debug output provides detailed timing and performance information:

**For `glob_stat_sha256_parallel`:**
```
[PERF] Starting parallel collection for pattern: data/**
[PERF] Normalized pattern: data/** -> data/**/*
[PERF] Glob expansion took: 140.41ms, found 30818 paths
[PERF] Quick metadata scan took: 37.35ms
[PERF] Found 26787 files, 4020 directories, 11 errors
[PERF] Starting parallel processing with 16 threads
[PERF] Hash: large_file.dat (2310212 bytes) took 70.38ms (open: 18.75µs, hash: 70.36ms) 2 reads, 32.8 MB/s
[PERF] Slow item: large_file.dat took 70.40ms (metadata: 2.0µs, hash: 70.38ms)
[PERF] Parallel processing took: 2.15s
[PERF] Total operation took: 2.33s
[PERF] Processed 30807 items, returned 30807 results
[PERF] Average time per item: 69.8µs
```

**For `glob_stat_sha256_jwalk`:**
```
[JWALK] Starting jwalk collection for pattern: data/**
[JWALK] Using normalized pattern: data/** -> data/**/*
[JWALK] Base directory: data, will filter with glob pattern: data/**/*
[JWALK] Directory walk found 7016 total paths
[JWALK] Comparing with glob crate results...
[JWALK] jwalk found: 7015 paths
[JWALK] glob crate found: 30818 paths
[JWALK] Files only found by glob (23803):
[JWALK]   - data/subdir/file1.txt
[JWALK]   - data/subdir/file2.txt
[JWALK]   ... and 23798 more
[JWALK] Parallel directory walk took: 475.54ms, found 30818 matching paths
[JWALK] Metadata count took: 52.59ms
[JWALK] Found 26787 files, 4020 directories, 11 errors
[JWALK] Starting parallel processing with 16 threads
[JWALK] Parallel processing took: 2.08s
[JWALK] Total operation took: 2.61s
[JWALK] Processed 30807 items, returned 30807 results
[JWALK] Average time per item: 67.5µs
```

### Performance Analysis

The debug output helps identify performance bottlenecks:

1. **Glob expansion time**: How long it takes to find matching files
2. **Thread utilization**: Number of threads used for parallel processing
3. **Individual file timing**: Slow files that may need attention
4. **Hash computation performance**: File reading and hashing speeds
5. **Pattern matching accuracy**: Comparison between different implementations

### Debug Output in Production

- **Default behavior**: Debug output is **disabled by default** - no performance impact
- **Runtime control**: Enable only when needed for troubleshooting
- **Clean output**: When disabled, functions run silently with no debug overhead
- **Available in both builds**: Debug instrumentation available in both debug and release builds

### Performance Comparison Example

```sql
-- Use debug output to compare implementations
-- Run with DUCKDB_FILE_TOOLS_DEBUG=1 to see detailed timing

-- Test glob-based parallel implementation
SELECT 'parallel' as method, COUNT(*) as files, SUM(CAST(size AS BIGINT)) as total_size
FROM glob_stat_sha256_parallel('large_dataset/**/*')
WHERE is_file = 'true';

-- Test jwalk-based implementation  
SELECT 'jwalk' as method, COUNT(*) as files, SUM(CAST(size AS BIGINT)) as total_size
FROM glob_stat_sha256_jwalk('large_dataset/**/*') 
WHERE is_file = 'true';
```

The debug output will show timing differences, helping you choose the best implementation for your use case.

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

-- Alternative: Use individual file functions for targeted analysis
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

-- Alternative: Use individual file hashing for targeted analysis
WITH specific_files AS (
    SELECT path FROM glob_stat('important_docs/**/*') WHERE is_file = 'true'
),
file_hashes AS (
    SELECT 
        path,
        file_sha256(path) AS hash,
        file_stat(path).size AS size
    FROM specific_files
    WHERE file_sha256(path) IS NOT NULL
)
SELECT 
    hash,
    size,
    array_agg(path) AS duplicate_files,
    count(*) AS duplicate_count
FROM file_hashes
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

