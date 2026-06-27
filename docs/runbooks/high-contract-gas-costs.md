# High Contract Gas Costs Runbook

## Alert Meaning
Gas cost for market creation operations has exceeded 500,000 threshold.

## Impact
- Increased transaction costs
- Reduced profitability
- Potential user friction from high fees

## Investigation Steps

1. **Check gas usage trends**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=contract_gas_used{operation="create_market"}'
   ```

2. **Review recent contract changes**
   - Check for new features
   - Review optimization opportunities
   - Check for inefficient operations

3. **Analyze transaction patterns**
   - Check market complexity
   - Review data storage patterns

## Remediation

### Immediate Actions
1. Review recent contract deployments
2. Analyze gas usage by operation
3. Identify optimization opportunities

### Short-term
1. Optimize contract code
2. Reduce storage operations
3. Batch operations where possible
4. Implement gas-efficient patterns

### Long-term
1. Implement gas monitoring
2. Add gas benchmarks to CI/CD
3. Establish gas cost SLOs
4. Regular contract optimization
