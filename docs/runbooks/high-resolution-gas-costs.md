# High Resolution Gas Costs Runbook

## Alert Meaning
Gas cost for market resolution operations has exceeded 300,000 threshold.

## Impact
- Increased operational costs
- Delayed market resolution
- Reduced profitability

## Investigation Steps

1. **Check gas usage**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=contract_gas_used{operation="resolve_market"}'
   ```

2. **Review resolution logic**
   - Check for unnecessary state updates
   - Review settlement calculations
   - Check for inefficient data access

3. **Analyze market complexity**
   - Check number of participants
   - Review outcome determination logic

## Remediation

### Immediate Actions
1. Analyze resolution operations
2. Identify optimization opportunities
3. Review settlement logic

### Short-term
1. Optimize resolution code
2. Reduce state updates
3. Implement efficient calculations
4. Batch settlement operations

### Long-term
1. Implement gas monitoring
2. Add resolution benchmarks
3. Establish gas cost targets
4. Regular optimization reviews
