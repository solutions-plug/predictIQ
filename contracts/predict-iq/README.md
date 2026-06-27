# predict-iq Contract

Soroban smart contract for the PredictIQ prediction market platform.

## Fuzzing

The `fuzz/` directory contains [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
targets for the three primary entry points. Fuzzing requires a nightly toolchain
and the `cargo-fuzz` binary.

### Setup

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

### Running a target

```bash
# From contracts/predict-iq/
cargo +nightly fuzz run fuzz_place_bet
cargo +nightly fuzz run fuzz_resolve_market
cargo +nightly fuzz run fuzz_withdraw
```

Run with a time limit (CI uses 60 s):

```bash
cargo +nightly fuzz run fuzz_place_bet -- -max_total_time=60
```

### Targets

| Target | Entry point | What it fuzzes |
|--------|-------------|----------------|
| `fuzz_place_bet` | `place_bet` | Arbitrary outcome, amount, timestamp |
| `fuzz_resolve_market` | `resolve_market` | Arbitrary market ID and winning outcome |
| `fuzz_withdraw` | `withdraw_refund` | Arbitrary market ID on a cancelled market |

### Corpus and crashes

Corpora are stored in `fuzz/corpus/<target>/` (gitignored). Crash-inducing
inputs found during a run are written to `fuzz/artifacts/<target>/` and must be
added as regression tests under `src/modules/` before the crash is considered
fixed.

### CI

The `contract-fuzz` CI job (`.github/workflows/test.yml`) runs each target for
**60 seconds** using libFuzzer on every push to `main` / `develop`. Crashes
upload to the `fuzz-crashes` GitHub Actions artifact.

## WASM Size Limit

The contract enforces a **64 KB (65,536 bytes)** WASM size limit. This is an internal budget target stricter than Soroban's actual limit, ensuring the contract remains performant and deployable across all networks. The limit is configured in `.github/workflows/test.yml` as `WASM_SIZE_LIMIT_BYTES` and checked during the build-optimized job.

If the contract exceeds this limit, optimize by:
- Removing unused dependencies
- Refactoring large functions into separate modules
- Using feature flags to conditionally compile code
- Consulting the runbook: `docs/runbooks/high-contract-gas-costs.md`

## Event Schema

All events emitted by this contract include a `version` field as the first element of the data payload. Indexers must check this field before decoding the rest of the payload to handle schema changes across contract upgrades.

**Current version: `1`**

### Topic Layout

| Position | Type | Description |
|----------|------|-------------|
| 0 | Symbol | Event name (max 9 chars) |
| 1 | u64 | `market_id` — primary identifier for indexers (0 for contract-level events) |
| 2 | Address | Triggering address |

### Data Layout

| Position | Type | Description |
|----------|------|-------------|
| 0 | u32 | `version` — schema version |
| 1+ | varies | Event-specific payload (see table below) |

### Event Reference

| Symbol | Description | Data (after version) |
|--------|-------------|----------------------|
| `mkt_creat` | Market created | `(description: String, num_outcomes: u32, deadline: u64)` |
| `bet_place` | Bet placed | `(outcome: u32, amount: i128)` |
| `disp_file` | Dispute filed | `(new_deadline: u64)` |
| `resolv_fx` | Resolution finalized | `(winning_outcome: u32, total_payout: i128)` |
| `reward_fx` | Rewards claimed | `(amount: i128, token: Address, is_refund: bool)` |
| `vote_cast` | Vote cast | `(outcome: u32, weight: i128)` |
| `cb_state` | Circuit breaker state changed | `(state: String)` |
| `oracle_ok` | Oracle result set | `(oracle_id: u32, outcome: u32)` |
| `orcl_res` | Oracle resolved | `(outcome: u32)` |
| `mkt_final` | Market finalized | `(winning_outcome: u32)` |
| `disp_res` | Dispute resolved | `(winning_outcome: u32)` |
| `mkt_cncl` | Market cancelled (admin) | _(none)_ |
| `mk_cn_vt` | Market cancelled (vote) | _(none)_ |
| `ref_rwrd` | Referral reward | `(amount: i128)` |
| `ref_claim` | Referral claimed | `(amount: i128)` |
| `ref_dist` | Referral distribution | _(none)_ |
| `cb_auto` | Circuit breaker auto-triggered | `(error_count: u32)` |
| `fee_colct` | Fee collected | `(amount: i128)` |
| `adm_fbk` | Admin fallback resolution | `(winning_outcome: u32)` |
| `rep_set` | Creator reputation set | `(old_score: u32, new_score: u32)` |
| `dep_set` | Creation deposit set | `(old_amount: i128, new_amount: i128)` |
| `mon_reset` | Monitoring state reset | `(previous_error_count: u32, previous_last_observation: u64)` |
| `mkt_prune` | Market pruned | `(pruned_at: u64)` |
| `upg_init` | Upgrade initiated | `(wasm_hash: BytesN<32>)` |
| `upg_vote` | Upgrade voted | `(vote_for: bool)` |
| `upg_exec` | Upgrade executed | `(wasm_hash: BytesN<32>)` |
| `upg_rej` | Upgrade rejected | `(wasm_hash: BytesN<32>)` |
| `mkt_state` | Market state changed | `(old_status: String, new_status: String, timestamp: u64)` |

### Version History

| Version | Changes |
|---------|---------|
| 1 | Initial versioned schema — `version` field added to all events |

> **Note for indexers:** When `version` is incremented, the payload structure for affected events may change. Always decode `version` first and branch on its value.
