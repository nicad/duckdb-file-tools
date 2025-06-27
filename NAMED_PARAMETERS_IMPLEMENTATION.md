# DuckDB Named Parameters Implementation Guide

This document explains how to implement true optional named parameters for the `glob_stat` table function in DuckDB using the C API from Rust.

## Current State

The current implementation supports:
- `glob_stat(pattern)` - uses default `ignore_case=false`
- `glob_stat(pattern, ignore_case_bool)` - positional parameter

## Goal

We want to support:
- `glob_stat(pattern)` - uses default `ignore_case=false`
- `glob_stat(pattern, ignore_case=true)` - named parameter syntax

## Implementation Approaches

### Approach 1: Enhanced VTab with named_parameters() (Implemented)

This approach uses the `named_parameters()` method in the VTab trait:

```rust
impl VTab for GlobStatVTab {
    // ... other methods ...

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // pattern (required)
        ])
    }

    fn named_parameters() -> Option<Vec<(String, LogicalTypeHandle)>> {
        Some(vec![
            (
                "ignore_case".to_string(),
                LogicalTypeHandle::from(LogicalTypeId::Boolean),
            ),
        ])
    }
}
```

**Status**: ✅ Implemented but limited by current duckdb-rs bindings
**Limitation**: The `named_parameters()` method exists but the `bind()` method doesn't have access to named parameter values through the high-level API.

### Approach 2: Direct C API Registration

This approach bypasses the VTab trait and uses the C API directly:

```rust
pub unsafe fn register_glob_stat_with_named_params(
    conn_ptr: ffi::duckdb_connection
) -> Result<(), Box<dyn std::error::Error>> {
    let table_func = ffi::duckdb_create_table_function();
    
    // Set function name
    let func_name = CString::new("glob_stat")?;
    ffi::duckdb_table_function_set_name(table_func, func_name.as_ptr());
    
    // Add required positional parameter
    let varchar_type = ffi::duckdb_create_logical_type(ffi::duckdb_type_DUCKDB_TYPE_VARCHAR);
    ffi::duckdb_table_function_add_parameter(table_func, varchar_type);
    
    // Add named parameter
    let bool_type = ffi::duckdb_create_logical_type(ffi::duckdb_type_DUCKDB_TYPE_BOOLEAN);
    let ignore_case_name = CString::new("ignore_case")?;
    ffi::duckdb_table_function_add_named_parameter(
        table_func, 
        ignore_case_name.as_ptr(), 
        bool_type
    );
    
    // Set callbacks and register
    // ... implementation details in separate file
}
```

**Status**: 🔄 Partially implemented (callbacks need completion)
**Advantage**: Full control over C API features
**Disadvantage**: Complex, requires extensive unsafe code

### Approach 3: Multiple Function Variants

Register multiple versions of the function:

```rust
// glob_stat(pattern) -> uses default ignore_case=false
// glob_stat(pattern, ignore_case) -> uses provided value
```

**Status**: ⚠️ Not recommended - creates confusion for users

## Current Working Solution

The implemented solution:

1. **Modified `parameters()`** to only require the pattern parameter
2. **Added `named_parameters()`** to declare the optional `ignore_case` parameter
3. **Enhanced parameter parsing** in the `bind()` method with fallback logic

### Usage Examples

```sql
-- These should work:
SELECT * FROM glob_stat('*.txt');                    -- Uses default ignore_case=false
SELECT * FROM glob_stat('*.txt', true);              -- Positional parameter
SELECT * FROM glob_stat('*.txt', ignore_case=true);  -- Named parameter (if supported)
```

## Limitations and Workarounds

### Current Limitations

1. **duckdb-rs VTab trait limitations**: The high-level Rust API doesn't expose named parameter access in the `bind()` method
2. **C API complexity**: Direct C API usage requires extensive unsafe code and callback implementations
3. **Type system challenges**: Converting between Rust and DuckDB types manually

### Recommended Workarounds

1. **Use positional parameters** with clear documentation
2. **Implement parameter validation** with helpful error messages
3. **Consider function overloading** for different use cases

## Testing

Test the implementation with:

```bash
# Build the extension
cargo build --release

# Load in DuckDB and test
duckdb -c "
.load target/release/libduckdb_file_tools.so;
SELECT * FROM glob_stat('*.txt') LIMIT 5;
SELECT * FROM glob_stat('*.TXT', true) LIMIT 5;
"
```

## Future Improvements

1. **Contribute to duckdb-rs**: Add named parameter support to the VTab trait
2. **Enhance error messages**: Provide better feedback for parameter issues  
3. **Add parameter validation**: Validate parameter types and values
4. **Documentation**: Add comprehensive usage examples

## References

- [DuckDB C API Table Functions](https://duckdb.org/docs/api/c/table_functions)
- [DuckDB C API Named Parameters](https://duckdb.org/docs/api/c/table_functions)
- [duckdb-rs VTab Documentation](https://docs.rs/duckdb/latest/duckdb/vtab/)
- [libduckdb-sys Bindings](https://docs.rs/libduckdb-sys/latest/libduckdb_sys/)

## Implementation Files

- `src/lib.rs` - Main implementation with enhanced VTab
- `src/glob_stat_optional.rs` - Alternative implementations and examples
- `test_optional_params.sql` - Test cases for the functionality