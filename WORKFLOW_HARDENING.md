# CI/CD Security & Reliability Hardening

Summary of the four fixes implemented on this branch. Each fix is its own
commit; this file documents what changed and why, since each individual
diff is small enough to need extra context.

## 1. Pin third-party GitHub Actions to commit SHAs

**Commit:** `security: pin third-party GitHub Actions to commit SHAs`

Only `aquasecurity/trivy-action` and `trufflesecurity/trufflehog` were
pinned to full commit SHAs. Every other third-party (non-`actions/`,
non-`github/`) action across all 14 workflows was referenced by a mutable
tag (`@v4`, `@v12`, etc.), which a compromised maintainer account could
silently repoint to malicious code with no visible diff in this repo.

Pinned to a commit SHA (resolved from the upstream tag/branch via
`git ls-remote`) with a trailing `# vX` comment for readability:

- `dtolnay/rust-toolchain` (`@stable`, `@master`)
- `hashicorp/setup-terraform` (`@v3`, `@v4`)
- `aws-actions/configure-aws-credentials` (`@v4`, `@v6`)
- `bridgecrewio/checkov-action` (`@v12`)
- `docker/setup-buildx-action`, `docker/login-action`,
  `docker/metadata-action`, `docker/build-push-action`
- `slackapi/slack-github-action`
- `actions-rs/toolchain`
- `softprops/action-gh-release`
- `orhun/git-cliff-action`
- `peter-evans/create-pull-request`
- `codecov/codecov-action`
- `returntocorp/semgrep-action`
- `gitleaks/gitleaks-action`

`actions/*` and `github/*` actions were intentionally left on major-version
tags per the issue's scope.

## 2. Open a PR for CHANGELOG updates instead of pushing to main

**Commit:** `security: open a PR for CHANGELOG updates instead of pushing to main`

`release.yml`'s `changelog` job committed and ran
`git push origin HEAD:main` directly using the default `GITHUB_TOKEN`
(`contents: write`), letting an automated bot bypass branch protection
and review on `main`.

- Replaced the direct push with `peter-evans/create-pull-request`, which
  opens a `changelog-update/<tag>` branch against `main` instead.
- Added `pull-requests: write` to the workflow's `permissions` block so
  the PR can be created.
- `protect-changelog.yml` blocks any PR that touches `CHANGELOG.md`, so
  it now carries a documented, narrowly-scoped exception: only a PR
  authored by `github-actions[bot]` from a `changelog-update/*` branch is
  allowed through. Every other PR touching `CHANGELOG.md` is still
  blocked exactly as before.

## 3. Fix ROLLBACK.md S3 bucket/key drift

**Commit:** `fix(docs): correct ROLLBACK.md S3 bucket/key and tfvars drift`

`infrastructure/ROLLBACK.md` documented `predictiq-terraform-state` /
`prod/terraform.tfstate` and `environments/prod.tfvars`, none of which
exist. The real backend config
(`infrastructure/terraform/environments/{production,staging}/backend.hcl`)
uses:

| Environment | Bucket | Key |
|---|---|---|
| Staging | `predictiq-terraform-state-staging` | `staging/terraform.tfstate` |
| Production | `predictiq-terraform-state-production` | `production/terraform.tfstate` |

Updated every `aws s3api` command, `terraform init -backend-config=...`,
the `-var-file` path (real file is
`environments/production/terraform.tfvars`), and the `prod` resource-id
references in the verification section, plus added a reference table so
the runbook stays copy-paste runnable against real infrastructure.

## 4. Add `timeout-minutes` to every CI job

**Commit:** `reliability: add timeout-minutes to every CI job`

None of the 14 workflows set a job-level timeout. A hung step (e.g.
`cargo install cargo-audit` / `soroban-cli`, or a stuck `terraform apply`)
would run until GitHub's 360-minute default — for `deploy.yml`'s
Terraform apply job, that also holds a DynamoDB state lock and blocks
every other environment behind it in the `max-parallel: 1` matrix.

- Added `timeout-minutes: 20` to every job across all 14 workflow files.
- Tightened `deploy.yml`'s `terraform-apply` job and the `cargo-audit`
  install jobs in `dependency-scan.yml` / `test.yml` to `timeout-minutes: 15`,
  since those are the jobs most likely to hang and most costly to block on.

**Known pre-existing issue (out of scope):** `test.yml`'s `clippy` and
`oracle-quality-gate` jobs have a `with:` key missing before
`components: clippy` on the `dtolnay/rust-toolchain` step (lines ~525,
~539 on `main` before this branch). This predates this branch and isn't
part of any of the four issues addressed here, so it was left as-is.
