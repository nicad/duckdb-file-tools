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

#### Phase 5: Age Encryption Integration - **IMPLEMENTED**

**Age Encryption/Decryption API Implementation**

**Implementation Status:** All Age encryption functions have been implemented and tested.

##### Core Encryption/Decryption Functions - IMPLEMENTED

1. **`age_encrypt(data BLOB, recipients VARCHAR) → BLOB`** - IMPLEMENTED
   - Encrypts BLOB data for comma-separated list of X25519 public key recipients  
   - `data`: Binary data to encrypt
   - `recipients`: Comma-separated age public keys (e.g., `'age1...,age1...'`)
   - Returns: Binary encrypted data in age format (any recipient can decrypt)
   - **Calls `age_encrypt_multi()` internally for consistency**

2. **`age_encrypt_multi(data BLOB, recipients VARCHAR[]) → BLOB`** - IMPLEMENTED
   - Encrypts BLOB data for array of X25519 public key recipients
   - `data`: Binary data to encrypt
   - `recipients`: Array of age public keys (e.g., `['age1...', 'age1...']`)
   - Returns: Binary encrypted data in age format

3. **`age_encrypt_passphrase(data BLOB, passphrase VARCHAR) → BLOB`** - IMPLEMENTED
   - Encrypts BLOB data using scrypt passphrase-based encryption
   - `data`: Binary data to encrypt  
   - `passphrase`: Password string for encryption
   - Returns: Binary encrypted data in age format

4. **`age_decrypt(encrypted_data BLOB, identities VARCHAR) → BLOB`** - IMPLEMENTED
   - Decrypts age-encrypted data trying comma-separated X25519 private keys
   - `encrypted_data`: Age-encrypted binary data
   - `identities`: Comma-separated age private keys (e.g., `'AGE-SECRET-KEY-1...,AGE-SECRET-KEY-1...'`)
   - Returns: Decrypted binary data (first matching identity is used)
   - **Calls `age_decrypt_multi()` internally for consistency**

5. **`age_decrypt_multi(encrypted_data BLOB, identities VARCHAR[]) → BLOB`** - IMPLEMENTED
   - Decrypts age-encrypted data using array of X25519 private keys
   - `encrypted_data`: Age-encrypted binary data
   - `identities`: Array of age private keys (e.g., `['AGE-SECRET-KEY-1...', ...]`)
   - Returns: Decrypted binary data

6. **`age_decrypt_passphrase(encrypted_data BLOB, passphrase VARCHAR) → BLOB`** - IMPLEMENTED
   - Decrypts age-encrypted data using passphrase
   - `encrypted_data`: Age-encrypted binary data  
   - `passphrase`: Password string used for encryption
   - Returns: Decrypted binary data

##### Key Generation Functions - IMPLEMENTED

7. **`age_keygen(dummy INTEGER) → STRUCT(public_key VARCHAR, private_key VARCHAR)`** - IMPLEMENTED
   - Generates a new X25519 key pair
   - `dummy`: Use 0 (required due to DuckDB scalar function limitations)
   - Returns: Struct with public and private keys in bech32 format
   - Example: `{public_key: "age1...", private_key: "AGE-SECRET-KEY-1..."}`

8. **`age_keygen_secret(name VARCHAR) → VARCHAR`** - IMPLEMENTED
   - Generates a new X25519 key pair and returns CREATE SECRET SQL
   - `name`: Name for the secret to be created
   - Returns: Complete CREATE SECRET SQL statement
   - Example: `CREATE SECRET my_keys (TYPE age, PUBLIC_KEY 'age1...', PRIVATE_KEY 'AGE-SECRET-KEY-1...');`

##### Implementation Status

**Phase 5.1: Core Dependencies** - COMPLETED
- Added `age = "0.11"` dependency to Cargo.toml
- Added `secrecy = "0.10"` for secure key handling
- Leveraged established patterns from compression functions

**Phase 5.2: X25519 Key-Based Encryption** - COMPLETED
- Implemented `age_encrypt()` and `age_encrypt_multi()` using `age::Encryptor::with_recipients()`
- Implemented `age_decrypt()` and `age_decrypt_multi()` using `age::Decryptor`
- Used `age::encrypt()` and `age::decrypt()` APIs for compatibility

**Phase 5.3: Passphrase-Based Encryption** - COMPLETED
- Implemented `age_encrypt_passphrase()` using `age::scrypt::Recipient`
- Implemented `age_decrypt_passphrase()` using `age::scrypt::Identity`
- Used secure scrypt parameters for key derivation

**Phase 5.4: Key Generation** - COMPLETED
- Implemented `age_keygen()` using `age::x25519::Identity::generate()`
- Implemented `age_keygen_secret()` for DuckDB secrets integration
- Return both public and private keys as structured data

**Phase 5.5: Error Handling** - COMPLETED
- Map age encryption errors to appropriate DuckDB errors
- Handle invalid keys, wrong passphrases, corrupted data gracefully
- Follow existing error handling patterns (return NULL on errors)

**Phase 5.6: Multi-Recipient Support** - COMPLETED
- Dual API design: comma-separated strings AND array inputs
- Both regular and _multi functions call shared underlying logic
- Multi-recipient encryption implemented

**Phase 5.7: Testing and Validation** - COMPLETED
- Basic functionality tested with real key generation
- Round-trip encryption/decryption implemented
- Multi-recipient encryption working
- Passphrase encryption functional
- Error cases handled

##### Current Limitation: Secret Type Registration

**⚠️ Known Issue:** DuckDB secret type registration requires C++ extension code that is not available through Rust FFI bindings.

**Status:**
- `age_keygen_secret()` generates CREATE SECRET SQL
- ❌ `CREATE SECRET ... (TYPE age, ...)` fails with "Secret type 'age' not found"
- All Age functions work with raw keys
- Secret name detection implemented (ready for future FFI support)

**Workaround:** Use raw keys directly or store in database tables. See FUNCTIONS.md for detailed examples.

##### Usage Scenarios

Age encryption can be useful in the following scenarios:

**Scenario 1: Transit Encryption**
* Store private key securely (in database tables or external key management)
* Distribute public key to data sources
* Data sources encrypt content with public key before transmission
* Database can decrypt received data using private key
* Ensures data confidentiality during transit

**Scenario 2: At-Rest Encryption**
* Store public key in database (tables or external systems)
* Private key managed outside database for security
* Encrypt sensitive data before storage using public key
* Database cannot read encrypted data without private key
* Authorized clients with private key can decrypt when needed

**Scenario 3: Multi-Recipient Workflows**
* Encrypt data for multiple recipients simultaneously
* Any recipient can decrypt with their private key
* Ideal for team collaboration and backup scenarios

##### Integration Status with DuckDB Secrets Manager

**Current Implementation:**
- `age_keygen_secret(name)` generates CREATE SECRET SQL with proper syntax
- Secret name detection in encryption/decryption functions (ready for future)
- Functions check key format to distinguish between raw keys and secret names
- ❌ Actual secret type registration requires C++ extension code (FFI limitation)

**Designed Secret Schema:**
```sql
CREATE SECRET key_name (
    TYPE age,
    PUBLIC_KEY 'age1...',
    PRIVATE_KEY 'AGE-SECRET-KEY-1...'  -- optional
);
```

**Current Workarounds:**
1. **Use raw keys directly**
2. **Store keys in database tables** 
3. **Generate CREATE SECRET SQL for future use**

See FUNCTIONS.md for complete implementation details and C++ code required for secret registration.

##### Usage Examples

```sql
-- Generate new key pairs
SELECT age_keygen(0) AS keys;

-- Multi-recipient encryption
SELECT age_encrypt('sensitive data'::BLOB, 'age1recipient1,age1recipient2') AS encrypted;

-- Array-based multi-recipient
SELECT age_encrypt_multi('data'::BLOB, ['age1key1', 'age1key2']) AS encrypted;

-- Passphrase encryption
SELECT age_encrypt_passphrase('secret'::BLOB, 'strong-password') AS encrypted;

-- Complete round-trip example
WITH keys AS (SELECT age_keygen(0) AS k),
     encrypted AS (SELECT k, age_encrypt('test message'::BLOB, k.public_key) AS enc FROM keys)
SELECT age_decrypt(enc, k.private_key)::VARCHAR AS decrypted FROM encrypted;

-- File encryption workflow
CREATE TABLE encrypted_files AS
SELECT 
    path,
    age_encrypt(read_blob(path), 'age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p') AS encrypted_content,
    file_stat(path).size AS original_size
FROM glob_stat('sensitive_docs/*.txt')
WHERE is_file = 'true';

-- Key management with tables
CREATE TABLE age_keys (
    key_name VARCHAR PRIMARY KEY,
    public_key VARCHAR,
    private_key VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO age_keys 
SELECT 'backup_key', (keys).public_key, (keys).private_key, CURRENT_TIMESTAMP
FROM (SELECT age_keygen(0) AS keys);
```

##### Technical Implementation Notes

**Rust Implementation Details:**
- Uses `age::Encryptor::with_recipients()` for multi-recipient encryption
- Uses `age::Decryptor` with identity iteration for decryption  
- Handles key parsing using `str::parse()` on recipient/identity strings
- Streams data through age encryption to handle large BLOBs efficiently
- Follows existing scalar function patterns from compression implementation
- Dual API: both comma-separated strings and array inputs supported

**Security Implementation:**
- Keys handled as strings in DuckDB - users responsible for secure storage
- Stateless extension design (no persistent key storage)
- Passphrase-based encryption uses secure scrypt parameters
- Memory zeroized appropriately by the age library via `secrecy` crate
- Private keys redacted in debug output and error messages

**Integration Approach:**
- Mirrors compression function structure for consistency
- Uses similar error handling (NULL return on failures)
- Maintains same performance characteristics as existing functions
- Supports both binary and text-based workflows
- Secret name detection ready for future FFI implementation

### Performance Considerations
- **Parallel Processing**: Leverage jwalk's parallel directory traversal
- **Memory Management**: Stream large files for hashing to avoid memory issues
- **Caching**: Consider caching stat results for repeated queries
- **Error Resilience**: Continue processing when individual files are inaccessible

## Project Structure
```
duckdb-file-tools/
├── Cargo.toml              # Complete with all dependencies (age, secrecy, etc.)
├── src/
│   ├── lib.rs              # Main extension entry point with all functions
│   └── glob_stat.rs        # Additional glob implementation
├── tests/                  # Test suite
│   ├── integration_tests.rs
│   └── test_data/
├── docs/                   # Documentation
│   ├── FUNCTIONS.md        # Function documentation
│   └── other-extensions/   # Reference implementations
├── EXTENSION_PLAN.md       # This file - implementation roadmap
├── README.md               # Project overview and setup
├── Makefile                # Build system integration
└── build/                  # Generated extension binaries
    ├── debug/
    └── release/

**Note:** All functionality is implemented in `src/lib.rs` following DuckDB extension patterns.
Age encryption functions are integrated alongside file system operations.
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

## Development Workflow - COMPLETED
1. Set up Rust project based on duckdb-extension-template-rs
2. Implement basic glob_stat without hashing
3. Add test suite
4. Implement SHA256 hashing support with multiple parallel implementations
5. Performance testing and optimization (debug output, jwalk alternatives)
6. Add compression functions (GZIP, LZ4, ZSTD)
7. Implement Age encryption integration
8. Documentation and usage examples for all functions
9. Debug and release builds

## Current Status

The extension includes:
- File system operations (glob_stat, file_stat, file_sha256, path_parts)
- High-performance parallel implementations with debug instrumentation  
- Multi-algorithm compression support (GZIP, LZ4, ZSTD)
- Age encryption implementation with multi-recipient support
- Documentation and examples
- Error handling and performance optimization

**Known Limitations:**
- Age secret type registration requires C++ extension code (FFI limitation)
- Limited testing in production environments
- Performance characteristics may vary across different systems

**Potential Future Enhancements:**
- C++ secret type registration for full DuckDB secrets integration
- Additional file system operations based on user feedback
- Performance optimizations based on real-world usage patterns
- More comprehensive testing and validation

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