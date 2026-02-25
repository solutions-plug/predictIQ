# Contributing to PredictIQ

Thank you for your interest in contributing to PredictIQ! This guide will help you get started with contributing to our decentralized prediction market platform.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Code Review Checklist](#code-review-checklist)
- [Getting Help](#getting-help)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please:

- Be respectful and considerate
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Respect differing viewpoints and experiences
- Accept responsibility and apologize for mistakes

## Getting Started

### Prerequisites

Before contributing, ensure you have:

1. Read the [DEVELOPMENT.md](./DEVELOPMENT.md) guide
2. Set up your local development environment
3. Familiarized yourself with the [ARCHITECTURE.md](./ARCHITECTURE.md)
4. Reviewed existing issues and pull requests

### Finding Work

- Check [GitHub Issues](https://github.com/your-org/predict-iq/issues) for open tasks
- Look for issues labeled `good-first-issue` or `help-wanted`
- Comment on an issue to express interest before starting work
- Ask questions if requirements are unclear

## Development Workflow

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/predict-iq.git
cd predict-iq

# Add upstream remote
git remote add upstream https://github.com/your-org/predict-iq.git
```

### 2. Create a Branch

```bash
# Update your main branch
git checkout main
git pull upstream main

# Create a feature branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### Branch Naming Convention

- `feature/` - New features (e.g., `feature/add-market-filters`)
- `fix/` - Bug fixes (e.g., `fix/123-resolve-bet-calculation`)
- `docs/` - Documentation updates (e.g., `docs/update-api-guide`)
- `refactor/` - Code refactoring (e.g., `refactor/optimize-oracle-queries`)
- `test/` - Test additions/fixes (e.g., `test/add-market-integration-tests`)
- `chore/` - Maintenance tasks (e.g., `chore/update-dependencies`)

### 3. Make Changes

Follow our [coding standards](#coding-standards) and ensure:

- Code is well-documented
- Tests are included
- No linting errors
- Commits follow our guidelines

### 4. Test Your Changes

```bash
# Run all tests
make test

# Run specific test suites
cargo test --lib                    # Unit tests
cargo test --test integration_test  # Integration tests

# Check formatting
cargo fmt --all -- --check

# Run linter
cargo clippy -- -D warnings
```

### 5. Commit Your Changes

Follow our [commit guidelines](#commit-guidelines):

```bash
git add .
git commit -m "feat: add market filtering by category"
```

### 6. Push and Create PR

```bash
# Push to your fork
git push origin feature/your-feature-name

# Create a Pull Request on GitHub
```

## Coding Standards

### Rust Code Style

We follow the official Rust style guide with some additions:

#### Formatting

```bash
# Format all code before committing
cargo fmt --all
```

#### Naming Conventions

```rust
// Constants: SCREAMING_SNAKE_CASE
const MAX_OUTCOMES: u32 = 100;

// Functions and variables: snake_case
fn create_market(market_id: u64) -> Result<(), ErrorCode> { }

// Types and traits: PascalCase
struct MarketData { }
trait OracleProvider { }

// Modules: snake_case
mod circuit_breaker;
```

#### Documentation

All public functions must have documentation:

```rust
/// Creates a new prediction market with the specified parameters.
///
/// # Arguments
///
/// * `creator` - Address of the market creator
/// * `title` - Market title/question
/// * `outcomes` - Vector of possible outcomes
/// * `deadline` - Market resolution deadline
///
/// # Returns
///
/// Returns the market ID on success, or an error code on failure.
///
/// # Errors
///
/// * `NotAuthorized` - Caller lacks permission
/// * `InvalidOutcome` - Outcome count exceeds maximum
///
/// # Examples
///
/// ```
/// let market_id = create_market(
///     creator_addr,
///     "Will BTC reach $100k?",
///     vec!["Yes", "No"],
///     deadline
/// )?;
/// ```
pub fn create_market(
    creator: Address,
    title: String,
    outcomes: Vec<String>,
    deadline: u64
) -> Result<u64, ErrorCode> {
    // Implementation
}
```

#### Error Handling

```rust
// Use Result types for fallible operations
fn get_market(id: u64) -> Result<Market, ErrorCode> {
    // Implementation
}

// Use descriptive error codes
return Err(ErrorCode::MarketNotFound);

// Avoid unwrap() in production code
// Use ? operator or proper error handling
let market = get_market(id)?;
```

#### Code Organization

```rust
// Order within modules:
// 1. Imports
// 2. Constants
// 3. Type definitions
// 4. Public functions
// 5. Private functions
// 6. Tests

use soroban_sdk::{contract, contractimpl, Address};

const MAX_MARKETS: u32 = 1000;

pub struct Market {
    // fields
}

#[contractimpl]
impl Contract {
    pub fn public_function() { }
    
    fn private_helper() { }
}

#[cfg(test)]
mod tests {
    // tests
}
```

### TypeScript/JavaScript (Frontend/Backend)

```typescript
// Use TypeScript strict mode
// Use functional components with hooks
// Use async/await over promises
// Use descriptive variable names

// Good
const fetchMarketData = async (marketId: string): Promise<Market> => {
  const response = await api.get(`/markets/${marketId}`);
  return response.data;
};

// Avoid
const getData = (id) => {
  return api.get('/markets/' + id).then(r => r.data);
};
```

## Testing Requirements

### Test Coverage

- All new features must include tests
- Aim for >80% code coverage
- Include both positive and negative test cases
- Test edge cases and error conditions

### Test Types

```rust
// Unit tests - test individual functions
#[test]
fn test_calculate_odds() {
    let odds = calculate_odds(100, 200);
    assert_eq!(odds, 50);
}

// Integration tests - test module interactions
#[test]
fn test_market_lifecycle() {
    let env = Env::default();
    let contract = create_contract(&env);
    
    // Create market
    let market_id = contract.create_market(/* ... */);
    
    // Place bet
    contract.place_bet(market_id, /* ... */);
    
    // Resolve market
    contract.resolve_market(market_id, /* ... */);
    
    // Verify results
    let market = contract.get_market(market_id);
    assert_eq!(market.status, MarketStatus::Resolved);
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_market_lifecycle

# Run tests with coverage
cargo tarpaulin --out Html
```

## Commit Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `perf`: Performance improvements

### Examples

```bash
# Feature
git commit -m "feat(markets): add category filtering"

# Bug fix
git commit -m "fix(bets): correct odds calculation for edge case"

# Documentation
git commit -m "docs(api): update market creation examples"

# With body and footer
git commit -m "feat(oracles): integrate Pyth Network oracle

Add support for Pyth Network price feeds with fallback
to Reflector oracle for redundancy.

Closes #123"
```

### Commit Best Practices

- Keep commits atomic (one logical change per commit)
- Write clear, descriptive commit messages
- Reference issue numbers when applicable
- Avoid commits like "fix typo" or "update" - be specific

## Pull Request Process

### Before Submitting

- [ ] Code follows style guidelines
- [ ] All tests pass locally
- [ ] New tests added for new features
- [ ] Documentation updated
- [ ] No linting errors
- [ ] Commit messages follow guidelines
- [ ] Branch is up to date with main

### PR Title Format

Use the same format as commit messages:

```
feat(markets): add real-time market updates
fix(voting): resolve duplicate vote prevention
```

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issues
Closes #123

## Changes Made
- Added market filtering by category
- Updated API documentation
- Added integration tests

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if applicable)
[Add screenshots for UI changes]

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests added
- [ ] All tests pass
```

### Review Process

1. **Automated Checks**: CI/CD runs tests and linting
2. **Code Review**: At least one maintainer reviews
3. **Feedback**: Address review comments
4. **Approval**: Maintainer approves PR
5. **Merge**: Maintainer merges to main

### Addressing Review Feedback

```bash
# Make requested changes
git add .
git commit -m "refactor: address review feedback"
git push origin feature/your-feature-name

# If you need to update your branch with main
git fetch upstream
git rebase upstream/main
git push origin feature/your-feature-name --force-with-lease
```

## Code Review Checklist

### For Authors

Before requesting review:

- [ ] Code is self-documenting with clear variable names
- [ ] Complex logic has explanatory comments
- [ ] No commented-out code
- [ ] No debug logs or console.log statements
- [ ] Error handling is comprehensive
- [ ] Edge cases are handled
- [ ] Performance implications considered
- [ ] Security implications considered
- [ ] Tests cover new functionality
- [ ] Documentation is updated

### For Reviewers

When reviewing code:

- [ ] Code solves the stated problem
- [ ] Logic is correct and efficient
- [ ] Code follows style guidelines
- [ ] Tests are adequate
- [ ] Error handling is appropriate
- [ ] Security vulnerabilities checked
- [ ] Performance impact assessed
- [ ] Documentation is clear
- [ ] Breaking changes are noted
- [ ] Backward compatibility maintained

### Review Feedback Guidelines

**Good Feedback:**
```
Consider using a HashMap here for O(1) lookups instead of 
iterating through the vector. This will improve performance 
when the market count is large.
```

**Avoid:**
```
This is wrong.
```

## Getting Help

### Resources

- **Documentation**: Check [docs/](./docs/) directory
- **API Reference**: See [API_SPEC.md](./API_SPEC.md)
- **Architecture**: Read [ARCHITECTURE.md](./ARCHITECTURE.md)
- **Development Guide**: See [DEVELOPMENT.md](./DEVELOPMENT.md)

### Communication Channels

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Discord**: [Join our Discord](https://discord.gg/predictiq) for real-time chat
- **Email**: dev@predictiq.io for private inquiries

### Common Questions

**Q: How do I run tests for a specific module?**
```bash
cargo test --package predict-iq --lib modules::markets
```

**Q: How do I update dependencies?**
```bash
cargo update
# Test thoroughly after updating
cargo test
```

**Q: How do I debug contract execution?**
```rust
// Use env.logs() in tests
#[test]
fn test_with_logs() {
    let env = Env::default();
    env.budget().reset_unlimited();
    // ... test code ...
    println!("{}", env.logs().all().join("\n"));
}
```

**Q: My PR has merge conflicts, what do I do?**
```bash
git fetch upstream
git rebase upstream/main
# Resolve conflicts
git add .
git rebase --continue
git push origin feature/your-branch --force-with-lease
```

## Recognition

Contributors will be:

- Listed in our [CONTRIBUTORS.md](./CONTRIBUTORS.md) file
- Mentioned in release notes for significant contributions
- Eligible for contributor rewards (for major contributions)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to PredictIQ! 🚀
