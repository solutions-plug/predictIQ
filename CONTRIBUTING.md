# Contributing to PredictIQ

Thank you for your interest in contributing! This guide covers everything you need to get started.

## Table of Contents

- [Development Setup](#development-setup)
- [Branch Naming](#branch-naming)
- [Commit Conventions](#commit-conventions)
- [Pull Request Process](#pull-request-process)
- [Running Tests](#running-tests)
- [Code Style](#code-style)

---

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+
- [Docker](https://docs.docker.com/get-docker/) and Docker Compose
- [PostgreSQL](https://www.postgresql.org/) 15+ (or use the provided Docker Compose stack)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/solutions-plug/predictIQ.git
cd predictIQ

# Start backing services (Postgres, Redis, etc.)
docker compose up -d

# API service
cd services/api
cp .env.example .env          # fill in required values
cargo build

# Frontend
cd frontend
cp .env.example .env.local    # fill in required values
npm install
npm run dev

# TTS service
cd services/tts
npm install
npm run dev
```

---

## Branch Naming

Use the following prefixes:

| Prefix | Purpose |
|--------|---------|
| `feat/` | New feature |
| `fix/` | Bug fix |
| `chore/` | Maintenance, dependency updates |
| `docs/` | Documentation only |
| `refactor/` | Code refactoring without behaviour change |
| `perf/` | Performance improvement |
| `ci/` | CI/CD changes |

Examples: `feat/market-resolution`, `fix/rate-limit-header`, `docs/contributing`

---

## Commit Conventions

This project uses **[Conventional Commits](https://www.conventionalcommits.org/)**.  
The CHANGELOG is **auto-generated** from commit messages via [git-cliff](https://git-cliff.org/) — do not edit `CHANGELOG.md` manually.

### Format

```
<type>(<scope>): <short description>

[optional body]

[optional footer(s)]
```

### Types

| Type | When to use |
|------|-------------|
| `feat` | A new feature (triggers a minor version bump) |
| `fix` | A bug fix (triggers a patch version bump) |
| `docs` | Documentation changes only |
| `chore` | Build process, dependency updates, tooling |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `ci` | CI/CD configuration changes |

Append `!` after the type/scope for **breaking changes** (triggers a major version bump):

```
feat(api)!: remove deprecated /v0 endpoints
```

### Examples

```
feat(markets): add oracle result caching
fix(newsletter): handle duplicate subscription gracefully
docs(api): document rate-limit response headers
chore(deps): bump axum to 0.7.5
```

---

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. Ensure all tests pass locally (see [Running Tests](#running-tests)).
3. Keep commits focused — one logical change per commit.
4. Open a PR against `main` with a clear title following the commit convention.
5. Fill in the PR description:
   - **What** changed and **why**
   - How to test the change
   - Any breaking changes or migration steps
6. Link related issues using `Closes #<issue>` in the PR description.
7. At least one approval is required before merging.
8. Squash-merge is preferred to keep the history clean.

### PR Checklist

- [ ] Branch is up to date with `main`
- [ ] Commit messages follow Conventional Commits
- [ ] Tests added or updated for the change
- [ ] Documentation updated if behaviour changed
- [ ] No secrets or credentials committed
- [ ] `CHANGELOG.md` **not** manually edited

---

## Running Tests

### API (Rust)

```bash
cd services/api
cargo test
```

### Frontend (Next.js)

```bash
cd frontend
npm test              # unit tests (Jest)
npm run test:e2e      # end-to-end tests (Playwright)
```

### Visual Regression Tests

Visual regression tests use Playwright snapshots to detect unintended UI changes. Baseline screenshots are stored in Git LFS to keep the repository size manageable.

#### Baseline Screenshot Storage

Baseline screenshot files are configured in `.gitattributes` to be tracked by Git LFS:

```
frontend/e2e/**/__snapshots__/*.png filter=lfs diff=lfs merge=lfs -text
```

Before running visual regression tests locally, ensure Git LFS is installed:

```bash
# Install Git LFS (macOS)
brew install git-lfs
git lfs install

# Or on Linux
sudo apt-get install git-lfs
git lfs install
```

#### Updating Baselines Locally

When UI changes are intentional and tests fail due to new screenshots, update baselines:

```bash
cd frontend
npx playwright test --update-snapshots
```

This command captures new baseline screenshots. Commit the updated baselines via Git LFS:

```bash
git add frontend/e2e/**/__snapshots__/
git commit -m "test: update visual regression baselines"
```

#### Visual Diff Threshold

The visual regression tests use a configurable diff threshold (default: 0.1%) to prevent flakiness from minor pixel differences. Configure the threshold via the `VISUAL_DIFF_THRESHOLD` environment variable:

```bash
# Run with custom threshold (e.g., 0.2%)
VISUAL_DIFF_THRESHOLD=0.2 npm run test:e2e
```

In CI, the threshold is enforced automatically. Tests fail if the pixel diff exceeds the configured threshold.

### TTS Service

```bash
cd services/tts
npm test
```

### Smart Contracts

```bash
cd contracts/predict-iq
make test
```

---

## Code Style

### Rust

- Follow `rustfmt` defaults — run `cargo fmt` before committing.
- Lint with `cargo clippy -- -D warnings`.

### TypeScript / JavaScript

- ESLint and Prettier are configured in the `frontend/` directory.
- Run `npm run lint` and `npm run format` before committing.

### YAML / Markdown

- Keep line length reasonable (80–100 characters).
- Use 2-space indentation for YAML.

---

## Branch Protection Rules

The `main` branch is protected. The following rules are enforced:

- **Pull request required** — direct pushes to `main` are not allowed; all changes must go through a PR.
- **CI must pass** — all status checks in `.github/workflows/` must succeed before a PR can be merged.
- **At least 1 approval required** — a PR must receive at least one approving review from a team member.
- **No force pushes** — `git push --force` to `main` is disabled.
- **No branch deletion** — `main` cannot be deleted.

These rules are configured in the repository settings under **Settings → Branches → Branch protection rules**.

## Code Ownership

Sensitive paths have designated reviewers defined in [`.github/CODEOWNERS`](.github/CODEOWNERS). GitHub automatically requests a review from the relevant owner when a PR touches those paths.

## Secrets and Environment Variables

- Never commit real secrets or credentials.
- Copy `services/api/.env.example` to `services/api/.env` and fill in real values locally. The `.env` file is gitignored.
- All placeholder values in `.env.example` are intentionally empty or clearly fake.
- Gitleaks runs on every push to detect accidental secret commits (see `.gitleaks.toml`).

---

## Questions?

Open an issue or start a discussion on [GitHub](https://github.com/solutions-plug/predictIQ/issues).
