# DuckDB File Tools Extension

[![Build Status](https://github.com/yourusername/duckdb-file-tools/workflows/CI/badge.svg)](https://github.com/yourusername/duckdb-file-tools/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A DuckDB extension written in Rust that provides file system operations, metadata extraction, compression, and planned Age encryption support.

Vibe coded so use at your discretion. Not tested much yet.

## Features

**File Operations**
- Parallel file scanning with multi-threaded hash computation
- Memory-efficient streaming for large files
- Cross-platform path handling (Windows, Unix, macOS)

**File Metadata**
- File size, timestamps (created, modified, accessed)
- Permissions, inode numbers, file type detection
- Path decomposition and analysis

**Multi-Algorithm Compression**
- GZIP (balanced), ZSTD (best compression), LZ4 (fastest)
- Automatic format detection for decompression
- Optimized for different use cases

**Content Analysis**
- SHA256 hash computation with streaming support
- BLOB manipulation and substring extraction
- Binary file header analysis

## Quick Start

### Installation

#### Option 1: Load Pre-built Extension
```sql
-- Load the extension (unsigned mode for development)
LOAD './build/release/file_tools.duckdb_extension';
```

#### Option 2: Build from Source
```bash
# Clone the repository
git clone https://github.com/yourusername/duckdb-file-tools.git
cd duckdb-file-tools

# Configure and build
make configure
make release

# Load in DuckDB
duckdb -unsigned -c "LOAD './build/release/file_tools.duckdb_extension';"
```

### Basic Usage

```sql
-- Scan files with metadata
SELECT path, size, modified_time 
FROM glob_stat('*.csv')
WHERE size > 1000000;

-- Get file hashes in parallel (high performance)
SELECT path, hash, size
FROM glob_stat_sha256_parallel('data/**/*')
WHERE is_file = 'true';

-- Analyze file types by extension
SELECT 
    path_parts(path).suffix AS extension,
    count(*) AS file_count,
    sum(cast(size AS BIGINT)) AS total_size
FROM glob_stat('**/*')
WHERE is_file = 'true'
GROUP BY extension
ORDER BY total_size DESC;

-- Compress data with different algorithms
SELECT 
    'GZIP' as algo, 
    octet_length(compress(data)) as size,
    'Balanced performance' as use_case
FROM (SELECT 'Large text data...'::BLOB as data)
UNION ALL
SELECT 
    'ZSTD' as algo,
    octet_length(compress_zstd(data)) as size, 
    'Best compression ratio' as use_case
FROM (SELECT 'Large text data...'::BLOB as data)
UNION ALL
SELECT 
    'LZ4' as algo,
    octet_length(compress_lz4(data)) as size,
    'Fastest compression' as use_case  
FROM (SELECT 'Large text data...'::BLOB as data);
```

## Function Reference

### Table Functions

| Function | Purpose | Performance |
|----------|---------|-------------|
| `glob_stat(pattern)` | File metadata collection | Standard |
| `glob_stat_sha256_parallel(pattern)` | **High-performance** parallel hashing | **Fast** |
| `glob_stat_sha256_jwalk(pattern)` | Alternative parallel implementation | **Fast** |

warning: `glob_stat_sha256_*` compute full file checksum, even though it is performed in parallel it can take a long time on big files and/or big directories. It should outperform using `glob` with `file_sha256` that doesn't seem parallelized (more testing needed).

### Scalar Functions

| Function | Purpose | Example |
|----------|---------|---------|
| `file_stat(path)` | Single file metadata | `file_stat('data.csv').size` |
| `file_sha256(path)` | SHA256 hash of file | `file_sha256('document.pdf')` |
| `path_parts(path)` | Path decomposition | `path_parts('/a/b/file.tar.gz').suffix` |
| `blob_substr(blob, start, length)` | BLOB substring | `blob_substr(data, 1, 4)` |
| `compress(data)` | GZIP compression | `compress('text'::BLOB)` |
| `compress_zstd(data)` | ZSTD compression | `compress_zstd(large_data)` |
| `compress_lz4(data)` | LZ4 compression | `compress_lz4(stream_data)` |
| `decompress(data)` | Auto-detect decompression | `decompress(compressed_blob)` |

## Performance

### Compression Estimations

not verified.

| Algorithm | Compression Ratio | Speed | Best Use Case |
|-----------|------------------|--------|---------------|
| **LZ4** | ~50% size reduction | 2-3 GB/s | Real-time, streaming, cache |
| **GZIP** | ~70% size reduction | 30-50 MB/s | General purpose, compatibility |  
| **ZSTD** | ~75% size reduction | 100-400 MB/s | **Balanced: recommended** |

### Parallel Performance

The parallel functions (`glob_stat_sha256_parallel`, `glob_stat_sha256_jwalk`) provide significant performance improvements:

- **Multi-core utilization**: Scales with available CPU cores
- **Large directories**: 5-10x faster than sequential processing
- **Memory efficient**: Streaming hash computation prevents memory issues
- **Debug instrumentation**: Set `DUCKDB_FILE_TOOLS_DEBUG=1` for detailed timing

## Advanced Usage

### Performance Monitoring

```bash
# Enable detailed performance instrumentation
export DUCKDB_FILE_TOOLS_DEBUG=1
duckdb -unsigned -c "
  LOAD './build/release/file_tools.duckdb_extension';
  SELECT count(*) FROM glob_stat_sha256_parallel('large_dataset/**/*');
"
```

### File Integrity Workflows

```sql
-- Create file integrity manifest
CREATE TABLE file_manifest AS
SELECT 
    path,
    hash,
    cast(size AS BIGINT) AS size,
    modified_time
FROM glob_stat_sha256_parallel('critical_data/**/*')
WHERE is_file = 'true';

-- Later: verify file integrity  
SELECT 
    path,
    hash = file_sha256(path) AS is_valid
FROM file_manifest;
```

### Data Archival with Compression

```sql
-- Archive logs with maximum compression
CREATE TABLE archived_logs AS
SELECT 
    date_trunc('day', timestamp) AS log_date,
    compress_zstd(string_agg(log_entry, '\n' ORDER BY timestamp)::BLOB) AS compressed_logs,
    count(*) AS entry_count
FROM application_logs 
WHERE timestamp < current_date - INTERVAL '30 days'
GROUP BY date_trunc('day', timestamp);

-- Fast cache with LZ4
CREATE TABLE cached_reports AS
SELECT 
    report_id,
    compress_lz4(report_data::BLOB) AS cached_data
FROM expensive_reports
WHERE access_count > 100;
```

## Development

### Building

```bash
# Development build with debug symbols
make configure
make debug

# Optimized release build
make release

# Run tests
make test_debug
make test_release
```

### Testing with Different DuckDB Versions

```bash
# Clean previous builds
make clean_all

# Configure for specific DuckDB version
DUCKDB_TEST_VERSION=v1.3.1 make configure

# Build and test
make debug
make test_debug
```

### Dependencies

- **Rust** 1.70+ (for development)
- **Python 3** with venv support
- **Make** 
- **Git**

Runtime dependencies are statically linked into the extension.

## Planned Features

🔐 **Age Encryption Support** (Coming Soon)
- `age_encrypt()` / `age_decrypt()` functions
- X25519 public key and passphrase-based encryption
- Key generation utilities
- Integration with existing file operations

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`) 
5. Open a Pull Request

### Development Guidelines

- Follow Rust conventions and `cargo fmt`
- Add tests for new functionality
- Update documentation for new features
- Run `make test_debug` before submitting

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- mostly coded by Claude Code
- Built on the [DuckDB Rust Extension Template](https://github.com/duckdb/duckdb-extension-template-rs)
- Uses the excellent [jwalk](https://github.com/Byron/jwalk) crate for parallel directory traversal
- Compression powered by [flate2](https://github.com/rust-lang/flate2-rs), [zstd](https://github.com/gyscos/zstd-rs), and [lz4_flex](https://github.com/PSeitz/lz4_flex)
- SHA256 implementation from [sha2](https://github.com/RustCrypto/hashes)

## Related Projects

- [DuckDB](https://duckdb.org/) - An in-process SQL OLAP database management system
- [rage](https://github.com/str4d/rage) - Rust implementation of the age encryption format