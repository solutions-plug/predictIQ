# PredictIQ Documentation

Welcome to the PredictIQ documentation! This directory contains comprehensive guides, references, and resources for developers, users, and contributors.

## 📚 Documentation Structure

```
docs/
├── README.md                    # This file
├── DISTRIBUTED_TRACING.md       # Distributed tracing setup and usage
└── (gas/ and security/ guides live in the repo root and services/)
```

## 🚀 Getting Started

### New to PredictIQ?

1. **[Project README](../README.md)** - Start here for project overview
2. **[API Specification](../API_SPEC.md)** - API reference and integration guide
3. **[Changelog](../CHANGELOG.md)** - Release history and notable changes

### Want to Contribute?

1. **[Contributing Guide](../CONTRIBUTING.md)** - Setup, branch naming, commit conventions, and PR process
2. **[API Specification](../API_SPEC.md)** - API reference and integration guide
3. **[Infrastructure README](../infrastructure/README.md)** - Infrastructure and deployment overview

## 📖 Documentation Categories

### Distributed Tracing

- **[Distributed Tracing Guide](./DISTRIBUTED_TRACING.md)** - OpenTelemetry setup and trace propagation

### Gas Optimization

Learn how to optimize gas usage in PredictIQ smart contracts:

- **[Gas Benchmarks README](../contracts/predict-iq/.gas-benchmarks/README.md)** - Gas benchmark results and methodology

### Infrastructure

- **[Infrastructure README](../infrastructure/README.md)** - Terraform modules, deployment, and rollback
- **[Rollback Guide](../infrastructure/ROLLBACK.md)** - Emergency rollback procedures

### Performance

- **[SLO Guide](../performance/SLO_GUIDE.md)** - Service level objectives and error budgets

### API Service

- **[Database Schema](../services/api/DATABASE.md)** - PostgreSQL schema and migration guide
- **[Graceful Shutdown](../services/api/GRACEFUL_SHUTDOWN.md)** - Shutdown behaviour and configuration
- **[Tracing](../services/api/TRACING.md)** - API service tracing configuration

## 🔍 Finding Documentation

### By Role

**Developers:**
- [API Specification](../API_SPEC.md)
- [Database Schema](../services/api/DATABASE.md)
- [Gas Benchmarks](../contracts/predict-iq/.gas-benchmarks/README.md)
- [Distributed Tracing](./DISTRIBUTED_TRACING.md)

**Operators / DevOps:**
- [Infrastructure README](../infrastructure/README.md)
- [Rollback Guide](../infrastructure/ROLLBACK.md)
- [SLO Guide](../performance/SLO_GUIDE.md)

**Users:**
- [Project README](../README.md)
- [API Specification](../API_SPEC.md)

### By Topic

**Smart Contracts:**
- [Gas Benchmarks](../contracts/predict-iq/.gas-benchmarks/README.md)

**Observability:**
- [Distributed Tracing](./DISTRIBUTED_TRACING.md)
- [API Tracing](../services/api/TRACING.md)

**Integration:**
- [API Specification](../API_SPEC.md)
- [Database Schema](../services/api/DATABASE.md)

## 🤝 Contributing to Documentation

Found an error or want to improve the documentation?

1. Documentation follows the same PR process as code
2. Use clear, concise language
3. Include code examples where applicable
4. Keep documentation up to date with code changes
5. Verify all links resolve before submitting

### Documentation Standards

- Use Markdown format
- Include table of contents for long documents
- Add code examples with syntax highlighting
- Link to related documentation
- Keep line length reasonable (80-100 characters)
- Use proper heading hierarchy

## 📝 Documentation Checklist

When creating or updating documentation:

- [ ] Clear and concise writing
- [ ] Code examples tested and working
- [ ] Links verified
- [ ] Spelling and grammar checked
- [ ] Follows project style
- [ ] Includes table of contents (if long)
- [ ] Cross-references added
- [ ] Updated in CHANGELOG (if significant)

## 🔗 External Resources

- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Pyth Network](https://pyth.network/)

## 📮 Feedback

Have suggestions for improving our documentation?

- Open an issue on [GitHub](https://github.com/solutions-plug/predictIQ/issues)
- Submit a PR with improvements

---

**Last Updated:** 2026-04-26  
**Maintained By:** PredictIQ Team
