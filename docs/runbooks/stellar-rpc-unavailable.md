# Stellar RPC Unavailable Runbook

## Alert

**Name:** `StellarRPCUnavailable`
**Severity:** critical
**Detection:** `stellar_rpc_up == 0` for 2 minutes, or
`stellar_rpc_error_rate > 0.5` for 5 minutes.
**Dashboard:** Grafana → *PredictIQ Services* → *Blockchain*

## Impact

- The blockchain indexer cannot ingest new events (bet placements, resolutions,
  payouts) from the Stellar network.
- Market resolution triggered by oracle callbacks will queue but not execute.
- The API returns stale data for on-chain state until connectivity is restored.
- New transactions (contract invocations) cannot be submitted.

## Immediate Mitigation (< 5 minutes)

1. Test connectivity to the configured RPC endpoint:
   ```bash
   curl -s "$STELLAR_RPC_URL/health" | jq .status
   # Expected: "healthy"
   ```
2. If unhealthy, switch to the fallback RPC endpoint:
   ```bash
   # Update the STELLAR_RPC_URL environment variable in ECS task definition
   aws ecs describe-task-definition --task-definition predictiq-indexer \
     --query 'taskDefinition.containerDefinitions[0].environment'
   # Then update and force redeploy with the fallback URL:
   # STELLAR_RPC_URL_FALLBACK is stored in AWS Secrets Manager
   aws ecs update-service \
     --cluster predictiq-prod \
     --service predictiq-indexer \
     --force-new-deployment
   ```
3. Check [Stellar Status](https://status.stellar.org) for network-wide
   incidents.

## Investigation Steps

1. **Determine the scope:** Is this our RPC provider (e.g., QuickNode, Blockdaemon)
   or the Stellar network itself?
   - Check the provider's status page.
   - Run `curl -s "https://horizon.stellar.org/fee_stats"` to test the public
     Horizon endpoint.
2. **Check the indexer error logs:**
   ```bash
   aws logs tail /ecs/predictiq-indexer --follow --since 10m | grep -i "rpc\|stellar\|timeout\|connect"
   ```
3. **Check the ledger sequence lag** — how far behind are we?
   ```bash
   # Current ledger from Horizon:
   curl -s https://horizon.stellar.org/ | jq .core_latest_ledger
   # Last ledger processed by our indexer (from the DB):
   psql $DATABASE_URL -c "SELECT max(ledger_sequence) FROM indexer_state"
   ```
4. **Inspect queued transactions** that failed to submit while the RPC was down;
   they will need to be replayed once connectivity is restored.

## Escalation

- **< 5 min:** On-call engineer switches to fallback RPC.
- **5–15 min:** If no fallback works and the Stellar network is operational,
  contact the RPC provider's support.
- **> 15 min, Stellar network issue:** Post a status update on the PredictIQ
  status page; no on-chain operations can proceed until the network recovers.

## Post-Incident Steps

1. Replay any missed ledgers once connectivity is restored; the indexer should
   auto-catchup but verify there are no gaps:
   ```bash
   psql $DATABASE_URL -c "SELECT count(*) FROM indexer_state WHERE processed = false"
   ```
2. Verify market resolutions and payout events that were queued during the
   outage processed correctly.
3. Evaluate adding a second RPC provider for automatic failover.
4. Update this runbook with new findings.
