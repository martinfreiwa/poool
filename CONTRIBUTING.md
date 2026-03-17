# Contributing to POOOL

First off, thank you for contributing to POOOL! It is our goal to build the most secure and transparent RWA platform in the world.

## Engineering Standards

### 1. Rust Best Practices
- **Format first**: Run `cargo fmt` before every commit.
- **Linting**: No warnings allowed. Run `cargo clippy` and address all suggestions.
- **Documentation**: All public functions and structs must have a `///` doc-string.
- **Error Handling**: Use the `anyhow` or `thiserror` crates; never use `unwrap()` in production code.

### 2. Commit Message Convention
We follow **Conventional Commits**:
- `feat: ...` for new features.
- `fix: ...` for bug fixes.
- `docs: ...` for documentation changes.
- `refactor: ...` for code changes that neither fix a bug nor add a feature.

Example: `feat(api): add withdrawal verification endpoint`

### 3. Branching Strategy
- `main`: The stable production-ready branch.
- `feat/feature-name`: For new features.
- `fix/issue-description`: For bug fixes.

## Pull Request Process
1. Create a branch from `main`.
2. Ensure all tests pass: `cargo test` and `python3 -m pytest tests/`.
3. Update relevant documentation (ADRs, `docs/`, or doc-strings).
4. Tag a maintainer for review.

## Architecture
Before proposing a major architectural change, please open a discussion or draft a new **ADR** (Architecture Decision Record) in `/docs/adr/`.
