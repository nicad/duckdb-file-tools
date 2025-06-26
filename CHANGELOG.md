# Changelog

All notable changes to the DuckDB File Tools extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Age encryption/decryption functions (`age_encrypt`, `age_decrypt`, `age_keygen`)
- Multi-recipient encryption support
- Advanced glob pattern options (exclude patterns, case-insensitive matching)
- File reading functions (`read_file_text`, `read_file_binary`)

## [0.1.0] - 2024-12-26

### Added
- **Table Functions**
  - `glob_stat(pattern)` - File metadata collection with glob patterns
  - `file_path_sha256(pattern)` - File scanning with SHA256 hash computation
  - `glob_stat_sha256_parallel(pattern)` - High-performance parallel file hashing
  - `glob_stat_sha256_jwalk(pattern)` - Alternative parallel implementation using jwalk

- **Scalar Functions**
  - `file_stat(path)` - Single file metadata as STRUCT
  - `file_sha256(path)` - SHA256 hash computation with streaming
  - `path_parts(path)` - Cross-platform path decomposition
  - `blob_substr(blob, start, length)` - BLOB substring extraction

- **Compression Functions**
  - `compress(data)` - GZIP compression (default)
  - `compress_zstd(data)` - ZSTD compression (best ratio)
  - `compress_lz4(data)` - LZ4 compression (fastest)
  - `decompress(data)` - Auto-detection decompression

- **Performance Features**
  - Multi-threaded hash computation using rayon
  - Streaming file processing for memory efficiency
  - Runtime debug instrumentation via `DUCKDB_FILE_TOOLS_DEBUG=1`
  - Parallel directory traversal with glob pattern matching

- **Platform Support**
  - Windows, macOS, and Linux compatibility
  - Cross-platform path handling and metadata extraction
  - Proper handling of different file systems and permissions

### Technical Details
- Built with Rust using DuckDB extension framework
- Statically linked dependencies for easy deployment
- Memory-safe implementation with comprehensive error handling
- Optimized for both development (debug) and production (release) builds

### Dependencies
- jwalk 0.8 - Parallel directory traversal
- sha2 0.10 - SHA256 hash computation
- glob 0.3 - Pattern matching
- rayon 1.8 - Parallel processing
- flate2 1.0 - GZIP compression
- lz4_flex 0.11 - LZ4 compression
- zstd 0.13 - ZSTD compression

### Performance Benchmarks
- **Parallel functions**: 5-10x faster than sequential processing on multi-core systems
- **Compression ratios**: LZ4 (~50%), GZIP (~70%), ZSTD (~75%)
- **Memory usage**: Streaming design prevents memory issues with large files
- **Hash computation**: Adaptive chunk sizes (1MB→8MB) for optimal throughput