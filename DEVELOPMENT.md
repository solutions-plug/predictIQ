# PredictIQ Development Guide

Complete guide for setting up and developing PredictIQ locally.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Project Structure](#project-structure)
- [Local Development Setup](#local-development-setup)
- [Running the Project](#running-the-project)
- [Testing](#testing)
- [Common Tasks](#common-tasks)
- [Debugging](#debugging)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.70+ | Smart contract development |
| Soroban CLI | 20.0.0+ | Contract deployment and testing |
| Node.js | 18+ | Frontend/backend development |
| Git | 2.30+ | Version control |
| Docker | 20.10+ | Local blockchain (optional) |

### Installation

#### 1. Install Rust

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add wasm target
rustup target add wasm32-unknown-unknown

# Verify installation
rustc --version
cargo --version
```

#### 2. Install Soroban CLI

```bash
# Install Soroban CLI
cargo install --locked --version 20.0.0 soroban-cli

# Verify installation
soroban --version
```

#### 3. Install Node.js

```bash
# Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Verify installation
node --version
npm --version
```

#### 4. Install Docker (Optional)

```bash
# macOS
brew install docker

# Linux
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Verify installation
docker --version
```

## Quick Start

Get up and running in under 5 minutes:

```bash
# 1. Clone the repository
git clone https://github.com/your-org/predict-iq.git
cd predict-iq

# 2. Build smart contracts
cd contracts/predict-iq
cargo build --target wasm32-unknown-unknown --release

# 3. Run tests
cargo test

# 4. Success! You're ready to develop
```

## Project Structure

```
predict-iq/
├── contracts/
│   └── predict-iq/              # Main smart contract
│       ├── src/
│       │   ├── lib.rs           # Contract entry point
│       │   ├── types.rs         # Data structures
│       │   ├── errors.rs        # Error definitions
│       │   ├── modules/         # Feature modules
│       │   │   ├── admin.rs     # Admin functions
│       │   │   ├── markets.rs   # Market management
│       │   │   ├── bets.rs      # Betting logic
│       │   │   ├── oracles.rs   # Oracle integration
│       │   │   ├── voting.rs    # Voting system
│       │   │   ├── disputes.rs  # Dispute resolution
│       │   │   └── ...
│       │   └── test.rs          # Test utilities
│       ├── Cargo.toml           # Dependencies
│       └── Makefile             # Build commands
├── docs/                        # Documentation
│   ├── api/                     # API documentation
│   ├── contracts/               # Contract docs
│   ├── security/                # Security guides
│   └── gas/                     # Gas optimization
├── .github/                     # CI/CD workflows
├── README.md                    # Project overview
├── CONTRIBUTING.md              # Contribution guide
├── DEVELOPMENT.md               # This file
└── ARCHITECTURE.md              # Architecture docs
```

### Key Directories

- **`contracts/predict-iq/src/`**: All smart contract code
- **`contracts/predict-iq/src/modules/`**: Feature-specific modules
- **`docs/`**: Comprehensive documentation
- **`.github/workflows/`**: CI/CD pipelines

## Local Development Setup

### 1. Clone and Navigate

```bash
git clone https://github.com/your-org/predict-iq.git
cd predict-iq
```

### 2. Configure Stellar Networks

```bash
# Add testnet
soroban config network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Add futurenet
soroban config network add futurenet \
  --rpc-url https://rpc-futurenet.stellar.org:443 \
  --network-passphrase "Test SDF Future Network ; October 2022"

# Set default network
soroban config network use testnet
```

### 3. Create Test Identity

```bash
# Generate a new identity
soroban config identity generate alice

# Fund the account (testnet only)
soroban config identity fund alice --network testnet

# Get the address
soroban config identity address alice
```

### 4. Environment Configuration

Create `.env` file in project root:

```bash
# .env
NETWORK=testnet
DEPLOYER_SECRET_KEY="S..."  # Your secret key
ADMIN_ADDRESS="G..."        # Admin address
RPC_URL="https://soroban-testnet.stellar.org:443"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

**⚠️ Important**: Add `.env` to `.gitignore` - never commit secrets!

### 5. Build the Contract

```bash
cd contracts/predict-iq

# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release

# Or use Makefile
make build
```

Build output: `target/wasm32-unknown-unknown/release/predict_iq.wasm`

## Running the Project

### Build Commands

```bash
# From contracts/predict-iq directory

# Standard build
cargo build --target wasm32-unknown-unknown --release

# Development build (faster, larger)
cargo build --target wasm32-unknown-unknown

# Clean build
cargo clean
make build

# Check without building
cargo check
```

### Deploy to Testnet

```bash
# Deploy contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predict_iq.wasm \
  --network testnet \
  --source alice

# Save the contract ID
CONTRACT_ID="<contract_id_from_output>"

# Initialize contract
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source alice \
  -- initialize \
  --admin $(soroban config identity address alice) \
  --base_fee 100
```

### Interact with Contract

```bash
# Create a market
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source alice \
  -- create_market \
  --creator $(soroban config identity address alice) \
  --title "Will BTC reach $100k by 2024?" \
  --outcomes '["Yes","No"]' \
  --deadline 1735689600

# Get market data
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- get_market \
  --id 0
```

## Testing

### Run All Tests

```bash
cd contracts/predict-iq

# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run with detailed output
cargo test -- --nocapture --test-threads=1
```

### Test Categories

```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration_test

# Specific test
cargo test test_market_lifecycle

# Tests matching pattern
cargo test market
```

### Test with Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Open report
open coverage/index.html
```

### Gas Benchmarking

```bash
# Run gas benchmarks
cd contracts/predict-iq/benches
./gas_benchmark.sh

# Or use cargo
cargo test --release -- --nocapture bench_
```

## Common Tasks

### Task 1: Adding a New Module

```bash
# 1. Create new module file
touch contracts/predict-iq/src/modules/my_feature.rs

# 2. Add module content
cat > contracts/predict-iq/src/modules/my_feature.rs << 'EOF'
use soroban_sdk::{contract, contractimpl, Address, Env};
use crate::errors::ErrorCode;

pub fn my_function(env: &Env, param: u64) -> Result<(), ErrorCode> {
    // Implementation
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_function() {
        let env = Env::default();
        let result = my_function(&env, 42);
        assert!(result.is_ok());
    }
}
EOF

# 3. Register in mod.rs
echo "pub mod my_feature;" >> contracts/predict-iq/src/modules/mod.rs

# 4. Build and test
cargo build
cargo test
```

### Task 2: Adding a New Contract Function

```rust
// In contracts/predict-iq/src/lib.rs

#[contractimpl]
impl PredictIQContract {
    /// Your new public function
    pub fn my_new_function(
        env: Env,
        param1: Address,
        param2: u64
    ) -> Result<String, ErrorCode> {
        // Verify authorization
        param1.require_auth();
        
        // Your logic here
        let result = format!("Processed: {}", param2);
        
        // Emit event
        env.events().publish(
            (symbol_short!("my_event"), param1.clone()),
            param2
        );
        
        Ok(result)
    }
}

// Add tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_new_function() {
        let env = Env::default();
        let contract = create_contract(&env);
        let user = Address::generate(&env);
        
        let result = contract.my_new_function(user, 42);
        assert!(result.is_ok());
    }
}
```

### Task 3: Adding a New Error Code

```rust
// In contracts/predict-iq/src/errors.rs

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    // ... existing errors ...
    
    /// New error: Description of when this occurs
    MyNewError = 121,
}

// Use in code
return Err(ErrorCode::MyNewError);
```

### Task 4: Updating Dependencies

```bash
# Update all dependencies
cargo update

# Update specific dependency
cargo update -p soroban-sdk

# Check for outdated dependencies
cargo outdated

# Run tests after updating
cargo test
```

### Task 5: Running Linter

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix automatically fixable issues
cargo clippy --fix

# Check formatting
cargo fmt --all -- --check

# Apply formatting
cargo fmt --all
```

## Debugging

### Debug Logging in Tests

```rust
#[test]
fn test_with_debug() {
    let env = Env::default();
    env.budget().reset_unlimited();
    
    let contract = create_contract(&env);
    
    // Your test code
    contract.create_market(/* ... */);
    
    // Print all logs
    println!("{}", env.logs().all().join("\n"));
}
```

### Inspect Contract Events

```rust
#[test]
fn test_events() {
    let env = Env::default();
    let contract = create_contract(&env);
    
    contract.create_market(/* ... */);
    
    // Get events
    let events = env.events().all();
    println!("Events: {:?}", events);
}
```

### Debug Contract State

```rust
#[test]
fn test_state() {
    let env = Env::default();
    let contract = create_contract(&env);
    
    // Check storage
    let storage = env.storage();
    println!("Storage: {:?}", storage);
}
```

### Using Rust Debugger

```bash
# Install rust-lldb (macOS) or rust-gdb (Linux)
rustup component add lldb-preview  # macOS
rustup component add gdb           # Linux

# Debug a test
rust-lldb target/debug/deps/predict_iq-<hash>
# or
rust-gdb target/debug/deps/predict_iq-<hash>
```

### Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --test integration_test

# Open flamegraph.svg
```

## Troubleshooting

### Common Issues

#### Issue: "error: linker `cc` not found"

**Solution:**
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora
sudo dnf install gcc
```

#### Issue: "target 'wasm32-unknown-unknown' not found"

**Solution:**
```bash
rustup target add wasm32-unknown-unknown
```

#### Issue: "soroban: command not found"

**Solution:**
```bash
# Reinstall Soroban CLI
cargo install --locked --version 20.0.0 soroban-cli --force

# Add to PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Issue: Tests fail with "insufficient balance"

**Solution:**
```bash
# Fund your testnet account
soroban config identity fund alice --network testnet

# Or use unlimited budget in tests
env.budget().reset_unlimited();
```

#### Issue: "RPC error: transaction failed"

**Solution:**
```bash
# Check network status
curl https://soroban-testnet.stellar.org:443/health

# Try different RPC endpoint
soroban config network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

#### Issue: Build is very slow

**Solution:**
```bash
# Use development build for faster iteration
cargo build --target wasm32-unknown-unknown

# Enable parallel compilation
export CARGO_BUILD_JOBS=8

# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
```

#### Issue: "error: could not compile `predict-iq`"

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build --target wasm32-unknown-unknown --release

# Check for syntax errors
cargo check

# Update dependencies
cargo update
```

### Getting More Help

1. **Check Documentation**: Review [docs/](./docs/) directory
2. **Search Issues**: Look for similar issues on GitHub
3. **Ask Community**: Join our [Discord](https://discord.gg/predictiq)
4. **Create Issue**: Open a GitHub issue with details

### Useful Commands Reference

```bash
# Build
cargo build --target wasm32-unknown-unknown --release

# Test
cargo test
cargo test -- --nocapture
cargo test test_name

# Format
cargo fmt --all

# Lint
cargo clippy -- -D warnings

# Deploy
soroban contract deploy --wasm <path> --network testnet --source alice

# Invoke
soroban contract invoke --id <id> --network testnet -- function_name --arg value

# Check network
soroban config network ls

# Check identity
soroban config identity ls
soroban config identity address alice

# Fund account
soroban config identity fund alice --network testnet
```

## Next Steps

- Read [ARCHITECTURE.md](./ARCHITECTURE.md) to understand system design
- Review [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guidelines
- Check [API_SPEC.md](./API_SPEC.md) for API reference
- Explore [docs/](./docs/) for detailed documentation

---

Happy coding! 🚀
