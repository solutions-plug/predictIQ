# Contract Invariants

This document enumerates the financial and state-machine invariants that the
`predict-iq` Soroban contract must uphold at all times. Violating any of these
invariants constitutes a critical bug.

---

## 1. Stake Conservation

**Statement:** The sum of all per-outcome stakes for a market equals the
market's `total_staked` field at every observable state boundary (after each
bet, after each refund, after each payout claim).

```
∀ market m:  Σ outcome_stake(m, o) for o in 0..m.num_outcomes  ==  m.total_staked
```

**Why:** `total_staked` drives payout calculations; a discrepancy would allow
over-payment or under-payment.

**Enforced by:** `invariants_test.rs`, `property_invariants_test.rs`
(proptest Props 1 & 5).

---

## 2. Non-Negative Stakes

**Statement:** `total_staked` and every `outcome_stake` are always `≥ 0`.

**Why:** Negative values would indicate funds were created from nothing.

**Enforced by:** `property_invariants_test.rs` (Prop 4).

---

## 3. State Machine Irreversibility

The market status follows a directed acyclic graph (DAG). Backwards transitions
are forbidden.

```
Active
  ├─► PendingResolution ──► Resolved   (terminal)
  ├─► Disputed          ──► Resolved   (terminal)
  └─► Cancelled                        (terminal)
```

**Rules:**
- `Resolved` and `Cancelled` are terminal — no further status changes are
  allowed once a market reaches either state.
- A market in `PendingResolution` or `Disputed` cannot return to `Active`.
- Only `Active` markets accept new bets.

**Enforced by:** `test_resolution_state_machine.rs`,
`property_invariants_test.rs` (Props 2 & 3).

---

## 4. Bet Acceptance Window

Bets are only accepted when:
- Market status is `Active`, **and**
- Current ledger timestamp `< market.deadline`, **and**
- Current ledger timestamp `< market.resolution_deadline`

**Enforced by:** `bets_fuzz_test.rs` (Props 4 & 5),
`property_invariants_test.rs` (Prop 3).

---

## 5. Payout Upper Bound

After resolution, the total amount distributed to winners must not exceed
`total_staked`. The platform fee creates a small shortfall (funds go to the
protocol treasury) so the bound is:

```
total_payouts ≤ total_staked
```

**Why:** Any excess would require the contract to create tokens, which is
impossible; the real failure mode is a logic error that directs more funds to
a single winner than were contributed.

---

## 6. Fee Integrity

The platform fee is collected once per bet at placement time. No fee is applied
at withdrawal or payout. The net stake recorded is:

```
net_stake = amount - floor(amount * fee_bps / 10_000)
```

The fee amount is transferred to the protocol treasury address at bet time.

---

## 7. Refund Idempotency

Calling `withdraw_refund` more than once for the same `(bettor, market_id,
outcome)` must not yield additional funds. The first call drains the stake
record to zero; subsequent calls are no-ops (or return an error).

---

## Formal Verification Notes

The most critical invariant for formal treatment is **Stake Conservation**
(§1) because it ties together every mutation path (place_bet, cancel,
resolve, claim_payout, withdraw_refund).

### Certora Prover

Certora's Prover can verify Soroban contracts compiled to WebAssembly using
its EVM-agnostic bytecode backend (in preview as of 2025). Suggested specs:

```certora
rule stakeConservation(method f) {
    env e;
    uint64 mid;
    mathint before = sumOutcomeStakes(mid);
    calldataarg args;
    f(e, args);
    mathint after = sumOutcomeStakes(mid);
    assert after == to_mathint(getMarket(mid).total_staked);
}
```

Track Certora's Soroban support at: https://docs.certora.com

### KEVM

KEVM can formally verify WASM semantics. For the payout logic the recommended
approach is:

1. Extract the `claim_payout` and `withdraw_refund` functions as standalone
   WASM modules.
2. Write K specifications asserting the stake cell decreases by exactly the
   computed payout, with no overflow.
3. Run with `kprove` against the WASM semantics module.

KEVM repository: https://github.com/runtimeverification/wasm-semantics

### Priority

Given the WASM toolchain maturity timeline, the recommended order is:
1. Expand proptest coverage (done — `property_invariants_test.rs`)
2. Add cargo-fuzz targets (done — `fuzz/` directory)
3. Engage Certora when Soroban WASM backend reaches GA
