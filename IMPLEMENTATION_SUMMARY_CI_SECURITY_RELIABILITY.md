# Implementation Summary: CI/CD Security & Reliability Fixes

## Overview

This implementation addresses four CI/CD reliability and security issues discovered in the
GitHub Actions workflows and Terraform configuration:

- **Issue 1** (🟠 High / Security): API container image scan silently no-ops because it targets a
  Dockerfile path that doesn't exist
- **Issue 2** (🟠 High / Reliability): `terraform plan` never converges because `default_tags`
  includes a constantly-changing `timestamp()` value
- **Issue 3** (🟡 Medium / Reliability): Rust dependency audit only fails CI on critical
  advisories, letting high-severity CVEs through
- **Issue 4** (🟡 Medium / Security): Workflows depend on archived/unmaintained third-party
  GitHub Actions (`actions-rs/toolchain`, `actions/create-release`)

All changes are in a single branch: `fix/ci-security-reliability-issues`

## Changes Made

### 1. Issue 1: API Container Image Scan Silently No-Ops

**Files Modified**: `.github/workflows/test.yml`

**Problem**:
The `container-scanning` job's build step ran:
```yaml
docker build -t predictiq-api:${{ github.sha }} -f services/api/Dockerfile .
```
`services/api/Dockerfile` does not exist anywhere in the repo — only the root `Dockerfile` and
`services/tts/Dockerfile` do. The build step was also marked `continue-on-error: true`, and the
following Trivy scan step was gated on `if: success()`. The net effect: the build failed on
every run, the failure was swallowed, the Trivy scan for the API image never executed, and the
job still reported green.

**Fix**:
- Build now points at the real Dockerfile: `-f Dockerfile` (repo root context, where the actual
  API Dockerfile lives).
- Removed `continue-on-error: true` from the build step, so a genuine build failure fails the
  job instead of being swallowed.
- Removed the `if: success()` gate on the Trivy step — with `continue-on-error` gone, a failed
  build now naturally halts the job before Trivy runs, and a successful build always reaches the
  scan.

**Before**:
```yaml
- name: Build API container image
  run: docker build -t predictiq-api:${{ github.sha }} -f services/api/Dockerfile .
  continue-on-error: true

- name: Scan API container with Trivy
  uses: aquasecurity/trivy-action@314ff8b43182423b84c50b1670b0e10f858f2d98 # master
  if: success()
  with:
    image-ref: "predictiq-api:${{ github.sha }}"
    ...
```

**After**:
```yaml
- name: Build API container image
  run: docker build -t predictiq-api:${{ github.sha }} -f Dockerfile .

- name: Scan API container with Trivy
  uses: aquasecurity/trivy-action@314ff8b43182423b84c50b1670b0e10f858f2d98 # master
  with:
    image-ref: "predictiq-api:${{ github.sha }}"
    ...
```

**Validation** (to run before merge):
```bash
docker build -t predictiq-api:test -f Dockerfile .
```
Re-run the `container-scanning` job in `test.yml` and confirm the Trivy step actually executes
and reports findings.

---

### 2. Issue 2: Terraform `default_tags` Uses `timestamp()`

**Files Modified**: `infrastructure/terraform/main.tf`

**Problem**:
```hcl
provider "aws" {
  default_tags {
    tags = {
      Environment = var.environment
      Project     = "predictiq"
      ManagedBy   = "terraform"
      CreatedAt   = timestamp()
    }
  }
}
```
`default_tags` applies to every taggable resource created by the AWS provider. Because
`timestamp()` re-evaluates on every `terraform plan`/`apply` invocation, every taggable resource
showed a spurious in-place tag update on every single run — breaking the ability to get a clean
`terraform plan`, which `terraform.yml` uses as a CI gate, and risking unnecessary resource churn
on every deploy.

**Fix**:
Removed `CreatedAt = timestamp()` from `default_tags` entirely. No replacement mechanism was
added (e.g. `ignore_changes` or a per-resource tag set once at creation) since no downstream
consumer of a `CreatedAt` tag was found in the codebase — if one is needed later, it should be
set via a separate mechanism that doesn't recompute on every plan.

**Before**:
```hcl
default_tags {
  tags = {
    Environment = var.environment
    Project     = "predictiq"
    ManagedBy   = "terraform"
    CreatedAt   = timestamp()
  }
}
```

**After**:
```hcl
default_tags {
  tags = {
    Environment = var.environment
    Project     = "predictiq"
    ManagedBy   = "terraform"
  }
}
```

**Note**: `infrastructure/terraform/modules/rds/main.tf` also calls `timestamp()`, but only
inside a conditional `final_snapshot_identifier` string for prod, which is not part of
`default_tags` and does not cause plan drift on every run — left unchanged.

**Validation** (to run before merge):
```bash
terraform plan -var-file=environments/dev.tfvars
terraform plan -var-file=environments/dev.tfvars   # run twice
```
The second run should report `No changes.`, and no resource tag diff should include a
`CreatedAt`/timestamp value.

---

### 3. Issue 3: Rust Dependency Audit Only Fails on Critical

**Files Modified**: `.github/workflows/dependency-scan.yml`

**Problem**:
The `scan-rust` job ran `cargo audit --deny warnings` with `continue-on-error: true`, then a
follow-up step parsed the JSON output and only exited non-zero when
`.advisory.severity == "critical"` was found. High-severity Rust CVEs were logged to the console
but never failed the build. This was inconsistent with:
- The `npm audit` step in the same file, which fails on both `critical` and `high`.
- `test.yml`'s `contracts-crate` audit, which runs `cargo audit --deny warnings` with no
  `continue-on-error` at all.

**Fix**:
- Removed `continue-on-error: true` from both initial `cargo audit --deny warnings` steps
  (`contracts/predict-iq` and `services/api`), so any warning-level finding fails the job
  immediately, matching the behavior already used in `test.yml`.
- Extended both follow-up JSON-based severity checks to count and fail on `high` severity in
  addition to `critical`, so the Rust audit gate matches the npm audit gate's severity threshold.

**Before**:
```yaml
- name: Audit contracts/predict-iq dependencies
  run: cargo audit --deny warnings
  working-directory: contracts/predict-iq
  continue-on-error: true

- name: Audit contracts/predict-iq (fail on critical)
  run: |
    output=$(cargo audit --json)
    critical=$(echo "$output" | jq '[.vulnerabilities[] | select(.advisory.severity == "critical")] | length')
    if [ "$critical" -gt 0 ]; then
      echo "❌ Found $critical critical vulnerabilities in contracts/predict-iq"
      exit 1
    fi
  working-directory: contracts/predict-iq
```

**After**:
```yaml
- name: Audit contracts/predict-iq dependencies
  run: cargo audit --deny warnings
  working-directory: contracts/predict-iq

- name: Audit contracts/predict-iq (fail on critical or high)
  run: |
    output=$(cargo audit --json)
    critical=$(echo "$output" | jq '[.vulnerabilities[] | select(.advisory.severity == "critical")] | length')
    high=$(echo "$output" | jq '[.vulnerabilities[] | select(.advisory.severity == "high")] | length')
    if [ "$critical" -gt 0 ] || [ "$high" -gt 0 ]; then
      echo "❌ Found $critical critical and $high high vulnerabilities in contracts/predict-iq"
      exit 1
    fi
  working-directory: contracts/predict-iq
```

The same change was applied symmetrically to the `services/api` audit steps.

**Validation** (to run before merge):
```bash
cd services/api && cargo audit --deny warnings
```
against a dependency with a known high-severity advisory, and confirm it exits non-zero. Re-run
`dependency-scan.yml` and confirm the job fails (not just logs a warning) on a high-severity
finding.

---

### 4. Issue 4: Archived/Unmaintained Third-Party GitHub Actions

**Files Modified**: `.github/workflows/contract-deployment.yml`, `.github/workflows/release.yml`,
`.github/workflows/test.yml`, `.github/workflows/performance.yml`

**Problem**:
- `actions-rs/toolchain@v1` was used in five places (`contract-deployment.yml:35`,
  `release.yml:88`, `test.yml:497`, `performance.yml:47` and `230`). The `actions-rs` GitHub org
  has been archived since 2021 and receives no further updates or security patches.
- `actions/create-release@v1` was used in `contract-deployment.yml:239`. This action was
  deprecated by GitHub in favor of `softprops/action-gh-release`, which `release.yml` already
  uses twice in the same repo (lines 65 and 132).
- Both are inconsistent with the actively-maintained `dtolnay/rust-toolchain` action already used
  in `dependency-scan.yml`.

**Fix**:
- Replaced every `actions-rs/toolchain@v1` step with `dtolnay/rust-toolchain@stable`, translating
  the old `profile: minimal` / `toolchain: stable` / `override: true` / `target: ...` inputs to
  the equivalent `targets:` / `components:` inputs `dtolnay/rust-toolchain` expects (dropping
  `profile` and `override`, which have no equivalent and aren't needed — `dtolnay/rust-toolchain`
  sets the installed toolchain as default automatically).
- Replaced `contract-deployment.yml`'s `actions/create-release@v1` step with
  `softprops/action-gh-release@v2`, translating `release_name` → `name` and keeping
  `tag_name` / `body` / `draft` / `prerelease` as-is, matching the pattern already used twice in
  `release.yml`.

**Before** (example from `contract-deployment.yml`):
```yaml
- name: Install Rust
  uses: actions-rs/toolchain@v1
  with:
    profile: minimal
    toolchain: stable
    override: true
    target: wasm32-unknown-unknown
    components: rustfmt, clippy
```

**After**:
```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown
    components: rustfmt, clippy
```

**Before** (`contract-deployment.yml` release creation):
```yaml
- name: Create GitHub Release
  uses: actions/create-release@v1
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  with:
    tag_name: contract-${{ needs.build-and-test.outputs.contract-hash }}
    release_name: Contract Deployment ${{ needs.build-and-test.outputs.contract-hash }}
    body: |
      Mainnet deployment of PredictIQ contract
      ...
    draft: false
    prerelease: false
```

**After**:
```yaml
- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    tag_name: contract-${{ needs.build-and-test.outputs.contract-hash }}
    name: Contract Deployment ${{ needs.build-and-test.outputs.contract-hash }}
    body: |
      Mainnet deployment of PredictIQ contract
      ...
    draft: false
    prerelease: false
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Validation** (to run before merge):
```bash
grep -rn "actions-rs/toolchain\|actions/create-release" .github/workflows
```
This was run locally after the change and returns zero matches. Trigger `release.yml` and
`contract-deployment.yml` and confirm both complete successfully end-to-end with the replacement
actions.

## Testing

None of the changes in this branch were built, run, or executed locally — per the task
constraints, only the workflow YAML and Terraform HCL files were edited. Before merge, the
Testing Requirements listed in each original issue should be run:

- `docker build -t predictiq-api:test -f Dockerfile .` from repo root
- `terraform plan -var-file=environments/dev.tfvars` twice in a row against unchanged config
- `cd services/api && cargo audit --deny warnings` against a known high-severity advisory
- `grep -rn "actions-rs/toolchain\|actions/create-release" .github/workflows` (zero matches
  confirmed already)
- `hadolint Dockerfile` and `actionlint` against the modified workflow files
- End-to-end trigger of `release.yml` and `contract-deployment.yml`

## Commits

1. **fix(ci): point API container scan at repo-root Dockerfile**
   Fixes the container-scanning job building a non-existent Dockerfile path and silently
   skipping the Trivy scan.

2. **fix(terraform): remove timestamp() from provider default_tags**
   Fixes `terraform plan` never converging due to a constantly-changing tag value.

3. **fix(ci): fail dependency-scan on high-severity Rust advisories**
   Fixes the Rust dependency audit only failing CI on critical (not high) severity findings.

4. **fix(ci): replace archived actions-rs/toolchain and actions/create-release**
   Replaces unmaintained third-party actions with actively-maintained equivalents already used
   elsewhere in the repo.

## Branch

All changes are in: `fix/ci-security-reliability-issues`

Ready for a PR that closes all four issues.

## Verification Checklist

- ✅ API container scan build path corrected, `continue-on-error`/`if: success()` gating removed
- ✅ Terraform `default_tags` no longer includes `timestamp()`
- ✅ Rust dependency audit fails on `critical` and `high` severity, consistently across
  `contracts/predict-iq` and `services/api`
- ✅ `actions-rs/toolchain` and `actions/create-release` fully removed from all workflows
  (confirmed via `grep`)
- ✅ Four issues, four isolated commits, single branch
- ⬜ `docker build`, `terraform plan`, `cargo audit`, `hadolint`, and `actionlint` validation
  runs — not executed as part of this change, still required before merge
