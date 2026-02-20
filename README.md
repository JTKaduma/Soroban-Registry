# Soroban Registry

> **A comprehensive platform for discovering, publishing, and verifying Soroban smart contracts on the Stellar network.**

Soroban Registry is the trusted package manager and contract registry for the Stellar ecosystem, similar to npm for JavaScript or crates.io for Rust. It provides developers with a centralized platform to share, discover, and verify smart contracts.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![TypeScript](https://img.shields.io/badge/typescript-5.0%2B-blue.svg)

## ✨ Features

- 🔍 **Contract Discovery** - Search and browse verified Soroban contracts
- ✅ **Source Verification** - Verify contract source code matches on-chain bytecode
- 📦 **Package Management** - Publish and manage contract versions
- 🌐 **Multi-Network Support** - Mainnet, Testnet, and Futurenet
- 🔐 **Publisher Profiles** - Track contract publishers and their deployments
- 📊 **Analytics** - Contract usage statistics and metrics
- 🎨 **Modern UI** - Beautiful, responsive web interface
- 🛠️ **CLI Tool** - Command-line interface for developers

## 🏗️ Architecture

```
soroban-registry/
├── backend/              # Rust backend services
│   ├── api/             # REST API server (Axum)
│   ├── indexer/         # Blockchain indexer
│   ├── verifier/        # Contract verification engine
│   └── shared/          # Shared types and utilities
├── frontend/            # Next.js web application
├── cli/                 # Rust CLI tool
├── database/            # PostgreSQL migrations
└── examples/            # Example contracts
```

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.75+ ([Install](https://rustup.rs/))
- **Node.js** 20+ ([Install](https://nodejs.org/))
- **PostgreSQL** 16+ ([Install](https://www.postgresql.org/download/))
- **Docker** (optional, for containerized setup)

### Database Seeding

Populate your development database with realistic test data:

```bash
# Seed with 50 contracts (default)
cargo run --bin seeder -- --count=50

# Seed with 100 contracts
cargo run --bin seeder -- --count=100

# Use a specific seed for reproducible data
cargo run --bin seeder -- --count=50 --seed=12345

# Use custom data file
cargo run --bin seeder -- --count=50 --data-file=./custom-data.json

# Specify database URL
cargo run --bin seeder -- --count=50 --database-url=postgresql://user:pass@localhost/dbname
```

**Features:**
- creates realistic contracts with names, descriptions, tags, and categories
- generates publishers with Stellar addresses
- creates contract versions and verification records
- Distributes contracts across all networks (mainnet, testnet, futurenet)
- safe to run multiple times
- fast - creates 100 contracts in <5 seconds
- reproducible with `--seed` flag

**Custom Data Format:**
```json
{
  "contract_names": ["CustomContract1", "CustomContract2"],
  "publisher_names": ["CustomPublisher1", "CustomPublisher2"]
}
```

### Option 1: Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/soroban-registry.git
cd soroban-registry

# Copy environment file
cp .env.example .env

# Start all services
docker-compose up -d

# The API will be available at http://localhost:3001
# The frontend will be available at http://localhost:3000
```

### Option 2: Manual Setup

#### 1. Database Setup

```bash
# Create database
createdb soroban_registry

# Set database URL
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
```

#### 2. Backend Setup

```bash
cd backend

# Install dependencies and build
cargo build --release

# Run migrations
sqlx migrate run --source ../database/migrations

# Start API server
cargo run --bin api
```

#### 3. Frontend Setup

```bash
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev
```

## 📖 Usage

### Web Interface

Visit `http://localhost:3000` to:
- Browse and search contracts
- View contract details and source code
- Publish new contracts
- Verify contract deployments

### CLI Tool

```bash
# Install CLI
cargo install --path cli

# Search for contracts
soroban-registry search "token"

# Get contract details
soroban-registry info <contract-id>

# Publish a contract
soroban-registry publish --contract-path ./my-contract

# Verify a contract
soroban-registry verify <contract-id> --source ./src
```

## 🔧 API Endpoints

### Contracts

- `GET /api/contracts` - List and search contracts
- `GET /api/contracts/:id` - Get contract details
- `POST /api/contracts` - Publish a new contract
- `GET /api/contracts/:id/versions` - Get contract versions
- `POST /api/contracts/verify` - Verify contract source

### Publishers

- `GET /api/publishers/:id` - Get publisher details
- `GET /api/publishers/:id/contracts` - Get publisher's contracts
- `POST /api/publishers` - Create publisher profile

### Statistics

- `GET /api/stats` - Get registry statistics
- `GET /health` - Health check

## 🗄️ Database Schema

The registry uses PostgreSQL with the following main tables:

- `contracts` - Contract metadata and deployment info
- `contract_versions` - Version history
- `verifications` - Verification records
- `publishers` - Publisher accounts
- `contract_interactions` - Usage statistics

See [`database/migrations/001_initial.sql`](database/migrations/001_initial.sql) for the complete schema.

## 🛠️ Development

### Running Tests

```bash
# Backend tests
cd backend
cargo test --all

# Frontend tests
cd frontend
npm test
```

### Code Formatting

```bash
# Rust
cargo fmt --all

# TypeScript
npm run lint
```

## 🌟 Example Contract

Here's how to publish a simple contract:

```rust
// examples/hello-world/src/lib.rs
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        symbol_short!("Hello")
    }
}
```

```bash
# Build the contract
cd examples/hello-world
soroban contract build

# Publish to registry
soroban-registry publish \
  --name "Hello World" \
  --description "A simple greeting contract" \
  --category "examples" \
  --network testnet
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Soroban SDK](https://github.com/stellar/rs-soroban-sdk)
- Inspired by [Hintents](https://github.com/dotandev/hintents) debugging tool
- Powered by the Stellar ecosystem

## 📞 Support

- **Documentation**: [Coming soon]
- **Issues**: [GitHub Issues](https://github.com/yourusername/soroban-registry/issues)
- **Discord**: [Stellar Discord](https://discord.gg/stellar)

---

**Built with ❤️ for the Stellar ecosystem**
