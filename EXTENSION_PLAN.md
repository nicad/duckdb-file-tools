# DuckDB File Tools Extension - Design Plan

## Project Overview

**Extension Name:** `duckdb-file-tools`  
**Language:** Rust  
**Primary Purpose:** Enhanced file system interaction with metadata and optional hashing capabilities

## Core Functionality

### Primary Features
1. **Enhanced File Globbing** - `glob_stat()` function that extends DuckDB's built-in `glob()` by including file metadata
2. **File Metadata Collection** - Retrieve stat() information (timestamps, size, inode, permissions)
3. **Optional Content Hashing** - SHA256 computation of file contents
4. **Other utilities**: blob_substr to extract specific bytes of a blob
4. **Future: Age Encryption Support** - Encryption/decryption using the age format

### Key Advantages Over Built-in glob()
- Returns comprehensive file metadata in addition to file paths
- Optional content hashing without separate file reads
- High-performance file traversal using jwalk library
- Structured output suitable for analysis and filtering

## API Design

### glob_stat() Function Signature
```sql
glob_stat(pattern TEXT)
→ TABLE(
    path TEXT,
    size BIGINT,
    modified_time TIMESTAMP,
    accessed_time TIMESTAMP,
    created_time TIMESTAMP,
    permissions TEXT,
    inode BIGINT,
    is_file BOOLEAN,
    is_dir BOOLEAN,
    is_symlink BOOLEAN
)
```

This is a table function.

### Parameters
- `pattern` (TEXT): File glob pattern (e.g., `*.txt`, `**/*.rs`)

### Usage Examples
```sql
-- Basic file listing with metadata
SELECT * FROM glob_stat('/home/user/**/*.log');

-- Include SHA256 hashes
SELECT path, size, hash FROM glob_stat('/data/*.csv');

-- Filter by file size and modification time
SELECT path, size, modified_time 
FROM glob_stat('/tmp/**/*') 
WHERE size > 1000000 AND modified_time > '2024-01-01';
```

### file_stat() Function signature
```sql
file_stat(filename TEXT)
→ STRUCT(
    size BIGINT,
    modified_time TIMESTAMP,
    accessed_time TIMESTAMP,
    created_time TIMESTAMP,
    permissions TEXT,
    inode BIGINT,
    is_file BOOLEAN,
    is_dir BOOLEAN,
    is_symlink BOOLEAN
)
```

This is a scalar function only.

### Parameters
- `filename` (TEXT): File path (e.g., `foo.txt`, `a/b/c/code.rs`)

### Usage Examples
```sql
-- most basic
SELECT file_stat(".gitignore");

-- Basic file listing with metadata
SELECT filename, file_stat(filename) FROM glob('/home/user/**/*.log');

 -- Access specific fields from glob results
SELECT filename, file_stat(filename).size FROM glob('*.txt');

-- Filter based on file metadata
SELECT filename FROM glob('*') WHERE file_stat(filename).size > 1000;

-- Order by file metadata
SELECT filename FROM glob('*') ORDER BY file_stat(filename).modified_time DESC;
```

### file_sha256() Function signature

Most important is to stream the file content by chunks to avoid reading all of it and keep memory usage good while using this function.

```sql
file_sha256(filename TEXT)
→ TEXT
```

This is a scalar function only.

### Parameters
- `filename` (TEXT): File path (e.g., `foo.txt`, `a/b/c/code.rs`)

### Usage Examples
```sql
-- most basic
SELECT file_sha256(".gitignore");

-- Basic file listing with metadata
SELECT filename, file_sha256(filename) FROM glob('/home/user/**/*.log');
```

### path_parts() Function signature

```sql
path_parts(path VARCHAR)
→ STRUCT(
    drive        VARCHAR,           -- “C:” on Windows, empty on POSIX
    root         VARCHAR,           -- "\" or "/" if present, else empty
    anchor       VARCHAR,           -- drive - plus - root  (convenience)
    parent       VARCHAR,           -- dirname(path) without trailing sep
    name         VARCHAR,           -- last component  ("archive.tar.gz")
    stem         VARCHAR,           -- name minus last suffix  ("archive.tar")
    suffix       VARCHAR,           -- last extension inc. dot  (".gz")
    suffixes     LIST<VARCHAR>,     -- all extensions  [".tar", ".gz"]
    parts        LIST<VARCHAR>,     -- path split on separators
    is_absolute  BOOLEAN            -- True when root is non-empty
);
```

This is a scalar function only.

### Parameters
- `path` (TEXT): A path to a file or a directory (e.g., `foo.txt`, `a/b/c/code.rs`, `mydir/subdir`)

### Usage Examples
```sql
-- most basic
SELECT path_parts(".gitignore");

-- Basic file listing with metadata
SELECT filename, path_parts(filename) FROM glob('/home/user/**/*.log');

SELECT filename FROM glob('/home/user/**/*.log') WHERE path_parts(filename).suffix = '.csv';
```

### glob_stat_sha256() Function Signature

Similar to glob_stat but adds a sha256.

```sql
glob_stat(pattern TEXT)
→ TABLE(
    path TEXT,
    size BIGINT,
    modified_time TIMESTAMP,
    accessed_time TIMESTAMP,
    created_time TIMESTAMP,
    permissions TEXT,
    inode BIGINT,
    is_file BOOLEAN,
    is_dir BOOLEAN,
    is_symlink BOOLEAN,
    hash VARCHAR
)
```

This is a table function.

### Parameters
- `pattern` (TEXT): File glob pattern (e.g., `*.txt`, `**/*.rs`)

### Usage Examples
```sql
-- Basic file listing with metadata
SELECT * FROM glob_stat('/home/user/**/*.log');

-- Include SHA256 hashes
SELECT path, size, hash FROM glob_stat('/data/*.csv');

-- Filter by file size and modification time
SELECT path, size, modified_time
FROM glob_stat('/tmp/**/*')
WHERE size > 1000000 AND modified_time > '2024-01-01';
```

### compress() and decompress() Function Signature

```sql
compress(data BLOB)
→ BLOB

decompress(data BLOB)
→ BLOB
```

They are scalar functions.

### Parameters
- `data` (BLOB): a literal or column to compress

### Usage Examples
```sql
SELECT compress('hello world');

SELECT
    decompress(data)
FROM (
    SELECT compress(text_file(filename)) as data
    FROM glob('/data/*.log')
);
```

### Implementation considerations

TODO: what compression method to use ? should it be pluggable ? what is a good rust library to include that doesn't add too much bloat ?

## Technical Architecture

### Core Dependencies
- **jwalk** - High-performance parallel file traversal
- use duckdb internal sha256 function but if that's not possible consider **sha2** - SHA256 hashing implementation
- use duckdb internal timestamp handling code but if that's not possible consider **chrono** - Timestamp handling and conversion
- **rage** (future) - Age encryption/decryption
- **duckdb-rs** or equivalent - DuckDB Rust bindings

### Implementation Phases

#### Phase 1: Core glob_stat() Implementation
1. Basic file traversal using jwalk
2. Stat metadata collection and conversion
3. DuckDB table function registration
4. Error handling and edge cases

#### Phase 3: Hashing Support
1. implement file_stat

#### Phase 3: Hashing Support
1. SHA256 implementation for file contents
2. Streaming hash computation for large files
3. Performance optimization for hash operations

#### Phase 4: Add extra arguments and other similar utility functions
1. exclude pattern: skip files matching a glob, for example `'.git/**'` would skip all git repos
2. ignore_case true/false for file name matching, for example `'*.csv'` would match .csv or .CSV
3. `read_file_text(pathname VARCHAR) VARCHAR` and `read_file_binary(pathname VARCHAR) BLOB` scalar functions that given a name returns its content
4. `permission_errors` optional argument can be 'ignore', 'print', 'fail' (default is 'ignore') - note: scalar functions like read_file_text/read_file_binary should error on permission issues
5. `symlink` optional argument can be 'follow', 'skip' (default is 'skip') - follow will include loop detection when implemented

#### Phase 5: Age Encryption Integration (Current Focus)

**Age Encryption/Decryption API Implementation**

Based on the rage library analysis, implement the following DuckDB scalar functions:

##### Core Encryption/Decryption Functions

1. **`age_encrypt(data BLOB, recipients VARCHAR[]) → BLOB`**
   - Encrypts BLOB data to multiple X25519 public key recipients  
   - `data`: Binary data to encrypt
   - `recipients`: Array of age public keys in bech32 format (e.g., `['age1...', 'age1...']`)
   - Returns: Binary encrypted data in age format (any recipient can decrypt)

2. **`age_encrypt(data BLOB, recipient VARCHAR) → BLOB`** *(overload)*
   - Encrypts BLOB data to single X25519 public key recipient
   - `data`: Binary data to encrypt
   - `recipient`: Single age public key in bech32 format (e.g., `age1...`)
   - Returns: Binary encrypted data in age format

3. **`age_encrypt_passphrase(data BLOB, passphrase VARCHAR) → BLOB`**
   - Encrypts BLOB data using scrypt passphrase-based encryption
   - `data`: Binary data to encrypt  
   - `passphrase`: Password string for encryption
   - Returns: Binary encrypted data in age format

4. **`age_decrypt(encrypted_data BLOB, identities VARCHAR[]) → BLOB`**
   - Decrypts age-encrypted data trying multiple X25519 private keys
   - `encrypted_data`: Age-encrypted binary data
   - `identities`: Array of age private keys in bech32 format (e.g., `['AGE-SECRET-KEY-1...', ...]`)
   - Returns: Decrypted binary data (first matching identity is used)

5. **`age_decrypt(encrypted_data BLOB, identity VARCHAR) → BLOB`** *(overload)*
   - Decrypts age-encrypted data using single X25519 private key
   - `encrypted_data`: Age-encrypted binary data
   - `identity`: Age private key in bech32 format (e.g., `AGE-SECRET-KEY-1...`)
   - Returns: Decrypted binary data

6. **`age_decrypt_passphrase(encrypted_data BLOB, passphrase VARCHAR) → BLOB`**
   - Decrypts age-encrypted data using passphrase
   - `encrypted_data`: Age-encrypted binary data  
   - `passphrase`: Password string used for encryption
   - Returns: Decrypted binary data

##### Key Generation Functions

7. **`age_keygen() → STRUCT(public_key VARCHAR, private_key VARCHAR)`**
   - Generates a new X25519 key pair
   - Returns: Struct with public and private keys in bech32 format
   - Example: `{public_key: "age1...", private_key: "AGE-SECRET-KEY-1..."}`

##### Implementation Plan

**Phase 5.1: Core Dependencies**
- Add `age = "0.11"` dependency to Cargo.toml
- Use the existing rage library from `docs/other-extensions/rage/age`
- Leverage established patterns from compression functions

**Phase 5.2: X25519 Key-Based Encryption**
- Implement `age_encrypt()` using `age::x25519::Recipient`
- Implement `age_decrypt()` using `age::x25519::Identity`
- Use `age::encrypt()` and `age::decrypt()` simple APIs for single recipients

**Phase 5.3: Passphrase-Based Encryption**
- Implement `age_encrypt_passphrase()` using `age::scrypt::Recipient`
- Implement `age_decrypt_passphrase()` using `age::scrypt::Identity`
- Use `age::Encryptor::with_user_passphrase()` for encryption

**Phase 5.4: Key Generation**
- Implement `age_keygen()` using `age::x25519::Identity::generate()`
- Return both public and private keys as structured data

**Phase 5.5: Error Handling**
- Map age encryption errors to appropriate DuckDB errors
- Handle invalid keys, wrong passphrases, corrupted data gracefully
- Follow existing error handling patterns from compression functions

##### Usage Examples

```sql
-- Generate a new key pair
SELECT age_keygen() AS keys;

-- Encrypt data with public key
SELECT age_encrypt('sensitive data'::BLOB, 'age1...') AS encrypted;

-- Decrypt with private key  
SELECT age_decrypt(encrypted_data, 'AGE-SECRET-KEY-1...') AS decrypted;

-- Encrypt with passphrase
SELECT age_encrypt_passphrase('secret message'::BLOB, 'my-password') AS encrypted;

-- Decrypt with passphrase
SELECT age_decrypt_passphrase(encrypted_data, 'my-password') AS decrypted;

-- File encryption workflow
CREATE TABLE encrypted_files AS
SELECT 
    path,
    age_encrypt(read_blob(path), 'age1...') AS encrypted_content
FROM glob_stat('sensitive_docs/*.txt')
WHERE is_file = 'true';
```

##### Technical Implementation Notes

**Rust Implementation Details:**
- Use `age::encrypt()` and `age::decrypt()` for single-recipient scenarios
- Handle key parsing using `str::parse()` on recipient/identity strings
- Stream data through age encryption to handle large BLOBs efficiently
- Follow existing scalar function patterns from compression implementation

**Security Considerations:**
- Keys are handled as strings in DuckDB - users responsible for secure storage
- No persistent key storage in extension (stateless design)
- Passphrase-based encryption uses scrypt with appropriate work factor
- Memory is zeroized appropriately by the age library

**Integration Approach:**
- Mirror compression function structure for consistency
- Use similar error handling and parameter validation
- Maintain same performance characteristics as existing functions
- Support both binary and text-based workflows

### Performance Considerations
- **Parallel Processing**: Leverage jwalk's parallel directory traversal
- **Memory Management**: Stream large files for hashing to avoid memory issues
- **Caching**: Consider caching stat results for repeated queries
- **Error Resilience**: Continue processing when individual files are inaccessible

## Project Structure
```
duckdb-file-tools/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main extension entry point
│   ├── glob_stat.rs        # glob_stat function implementation
│   ├── metadata.rs         # File metadata collection utilities
│   ├── hashing.rs          # File content hashing
│   └── encryption.rs       # Age encryption support (future)
├── tests/
│   ├── integration_tests.rs
│   └── test_data/
└── examples/
    └── usage_examples.sql
```

## Reference Extensions
- **duckdb-extension-template-rs** - Rust extension template and build system
- **duckdb-shellfs** - File system interaction patterns
- **duckdb-crypto** - Cryptographic function implementation
- **duckdb-zipfs** - Archive file handling

these extensions can be found locally at docs/other-extensions/

You can also find duckdb itself at docs/other-extensions/duckdb but because it's a big code base only read what is needed from it.

## Best practices
* avoid comments describing what the next line does
* compile code after making a change, if can't make the code compile after 2 tries just prompt me back telling me that the code doesn't compile (otherwise I can assume it does compile)

## Development Workflow
1. Set up Rust project based on duckdb-extension-template-rs
2. Implement basic glob_stat without hashing
3. Add comprehensive test suite
4. Implement SHA256 hashing support
5. Performance testing and optimization
6. Documentation and usage examples
7. Future: Age encryption integration

## Security Considerations
- File system access permissions
    - if no permissions to read skip without errors
- Path traversal prevention
- Secure handling of encryption keys
- Memory safety for large file processing

## Testing Strategy
- Unit tests for metadata extraction
- Integration tests with various file types
- Performance benchmarks vs. built-in glob()
- Edge case handling (permissions, symlinks, large files)
- Cross-platform compatibility testing

## Success Metrics
- Performance improvement over multiple glob() + stat() calls
- Accurate metadata collection across platforms
- Reliable hash computation for various file sizes
- Clean integration with DuckDB query patterns

## Development, debugging and troubleshooting

```bash
duckdb -unsigned -cmd " LOAD './build/debug/file_tools.duckdb_extension';"
DUCKDB_FILE_TOOLS_DEBUG=1 duckdb -unsigned -cmd "load './build/debug/file_tools.duckdb_extension';"
duckdb -unsigned -cmd "load './build/release/file_tools.duckdb_extension';"

select * from glob_stat_sha256_parallel('/Users/nicolas/Downloads/*.pdf');
select * from glob_stat_sha256_jwalk('/Users/nicolas/Downloads/*.pdf');
```