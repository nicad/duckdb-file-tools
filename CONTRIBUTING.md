# Contributing to DuckDB File Tools

Thank you for your interest in contributing to the DuckDB File Tools extension! This document provides guidelines for contributing to the project.

## Getting Started

### Prerequisites

- **Rust** 1.70 or later
- **Python 3** with venv support
- **Make**
- **Git**

### Development Setup

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/yourusername/duckdb-file-tools.git
   cd duckdb-file-tools
   ```

2. Configure the development environment:
   ```bash
   make configure
   ```

3. Build and test:
   ```bash
   make debug
   make test_debug
   ```

## Development Guidelines

### Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Use `cargo clippy` to catch common issues
- Add documentation for public APIs
- Write meaningful commit messages

### Testing

- Add tests for new functionality
- Ensure all tests pass: `make test_debug` and `make test_release`
- Test on multiple platforms if possible
- Include edge cases and error conditions

### Performance

- Profile new functions with large datasets
- Use the debug instrumentation: `DUCKDB_FILE_TOOLS_DEBUG=1`
- Consider memory usage for streaming operations
- Benchmark against existing implementations

## Types of Contributions

### Bug Reports

When reporting bugs, please include:

- DuckDB version
- Operating system and architecture
- Extension version
- Minimal reproduction steps
- Expected vs. actual behavior
- Any error messages or logs

Use this template:
```markdown
## Bug Description
Brief description of the issue

## Steps to Reproduce
1. Load extension: `LOAD './file_tools.duckdb_extension';`
2. Run query: `SELECT ...`
3. See error

## Expected Behavior
What should happen

## Actual Behavior  
What actually happens

## Environment
- OS: macOS 14.1
- DuckDB: 1.3.1
- Extension: 0.1.0
```

### Feature Requests

For new features, please:

- Check existing issues and planned features
- Describe the use case and benefits
- Consider implementation complexity
- Provide examples of how it would be used

### Code Contributions

1. **Create an issue** first to discuss the change
2. **Fork the repository** and create a feature branch
3. **Implement the feature** following our guidelines
4. **Add tests** for the new functionality
5. **Update documentation** as needed
6. **Submit a pull request**

## Pull Request Process

### Before Submitting

- [ ] Code follows project style guidelines
- [ ] All tests pass (`make test_debug` and `make test_release`)
- [ ] New functionality includes tests
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if applicable)

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change)
- [ ] New feature (non-breaking change)  
- [ ] Breaking change (fix or feature that changes existing behavior)
- [ ] Documentation update

## Testing
- [ ] Tests added/updated
- [ ] Manual testing completed
- [ ] Performance impact assessed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
```

## Code Structure

### Adding New Functions

1. **Scalar Functions**: Add to `src/lib.rs` following existing patterns
2. **Table Functions**: Create new module in `src/` if complex
3. **Tests**: Add to `test/sql/` directory using SQLLogicTest format
4. **Documentation**: Update `FUNCTIONS.md` with examples

### Example Function Implementation

```rust
// Scalar function example
struct MyFunction;

impl VScalar for MyFunction {
    fn call(&self, params: &[&LogicalType], args: &[DataChunk]) -> Result<Vector> {
        // Implementation here
    }
}

// Register in main extension function
#[duckdb_entrypoint]
pub fn extension_entrypoint(con: Connection) -> Result<(), Box<dyn Error>> {
    con.register_scalar_function::<MyFunction>("my_function")?;
    Ok(())
}
```

## Architecture Notes

### Performance Considerations

- Use `rayon` for parallel processing
- Implement streaming for large files
- Consider memory usage patterns
- Profile with real-world datasets

### Error Handling

- Use Result types consistently
- Provide meaningful error messages
- Handle edge cases gracefully
- Don't panic in library code

### Platform Compatibility

- Test on Windows, macOS, and Linux
- Handle path separators correctly
- Consider different file system behaviors
- Use cross-platform libraries

## Documentation

### Function Documentation

Each function should have:

- Clear description of purpose
- Parameter types and meanings
- Return value description
- Usage examples
- Performance characteristics
- Error conditions

### Code Comments

- Explain **why**, not **what**
- Document complex algorithms
- Note performance implications
- Explain non-obvious code

## Release Process

### Versioning

We use [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

- [ ] All tests pass on all platforms
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated
- [ ] Version numbers are bumped
- [ ] Git tag is created
- [ ] Release notes are written

## Getting Help

- **Documentation**: Check README.md and FUNCTIONS.md
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions
- **Email**: Contact maintainers for security issues

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and inclusive in all interactions.

## Recognition

Contributors will be recognized in:

- CHANGELOG.md for significant contributions
- README.md acknowledgments section
- Git commit history

Thank you for contributing! 🎉