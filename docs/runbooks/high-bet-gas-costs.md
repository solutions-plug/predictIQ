# High Bet Gas Costs Runbook

## Alert Meaning
Gas cost for placing bet operations has exceeded 200,000 threshold.

## Impact
- Increased transaction costs for users
- Reduced user engagement
- Competitive disadvantage

## Investigation Steps

1. **Check gas usage**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=contract_gas_used{operation="place_bet"}'
   ```

2. **Review contract implementation**
   - Check for unnecessary operations
   - Review data validation logic
   - Check for inefficient storage patterns

3. **Analyze bet patterns**
   - Check bet complexity
   - Review market state updates

## Remediation

### Immediate Actions
1. Analyze recent contract changes
2. Identify gas optimization opportunities
3. Review bet validation logic

### Short-term
1. Optimize contract operations
2. Reduce validation overhead
3. Implement efficient data structures
4. Batch operations where possible

### Long-term
1. Implement gas monitoring
2. Add performance benchmarks
3. Establish gas cost targets
4. Regular optimization reviews
