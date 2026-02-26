/**
 * Frontend Mock Test for PredictIQ Error Code Parsing
 * 
 * This demonstrates how a frontend application can parse and handle
 * error codes 100-120 from the PredictIQ smart contract.
 */

// Error code mapping (100-120)
const ERROR_CODES = {
  100: { name: "AlreadyInitialized", message: "Contract has already been initialized" },
  101: { name: "NotAuthorized", message: "Caller lacks required authorization" },
  102: { name: "MarketNotFound", message: "Requested market does not exist" },
  103: { name: "MarketClosed", message: "Market is closed for betting" },
  104: { name: "MarketStillActive", message: "Market is still active (cannot resolve yet)" },
  105: { name: "InvalidOutcome", message: "Outcome index is out of bounds" },
  106: { name: "InvalidBetAmount", message: "Bet amount is invalid (zero or negative)" },
  107: { name: "InsufficientBalance", message: "User has insufficient balance" },
  108: { name: "OracleFailure", message: "Oracle failed to provide result" },
  109: { name: "CircuitBreakerOpen", message: "Circuit breaker is open (system paused)" },
  110: { name: "DisputeWindowClosed", message: "Dispute period has ended" },
  111: { name: "VotingNotStarted", message: "Voting period has not begun" },
  112: { name: "VotingEnded", message: "Voting period has ended" },
  113: { name: "AlreadyVoted", message: "User has already cast a vote" },
  114: { name: "FeeTooHigh", message: "Fee exceeds acceptable threshold" },
  115: { name: "MarketNotActive", message: "Market is not in active state" },
  116: { name: "DeadlinePassed", message: "Market deadline has passed" },
  117: { name: "CannotChangeOutcome", message: "Cannot change bet outcome after initial bet" },
  118: { name: "MarketNotDisputed", message: "Market is not in disputed state" },
  119: { name: "MarketNotPendingResolution", message: "Market is not pending resolution" },
  120: { name: "AdminNotSet", message: "Admin address has not been configured" },
};

/**
 * Parse error code from contract response
 */
function parseErrorCode(errorCode) {
  const error = ERROR_CODES[errorCode];
  if (!error) {
    return {
      code: errorCode,
      name: "UnknownError",
      message: `Unknown error code: ${errorCode}`,
    };
  }
  return {
    code: errorCode,
    ...error,
  };
}

/**
 * Parse event from contract
 */
function parseEvent(event) {
  const { topics, data } = event;
  
  // Standard format: (Topic, MarketID, SubjectAddr, Data)
  const eventType = topics[0];
  const marketId = topics[1] || null;
  const subjectAddr = topics[2] || null;
  
  return {
    type: eventType,
    marketId,
    subjectAddr,
    data,
  };
}

/**
 * Test error code parsing
 */
function testErrorCodeParsing() {
  console.log("=== Testing Error Code Parsing (100-120) ===\n");
  
  let passed = 0;
  let failed = 0;
  
  // Test all error codes 100-120
  for (let code = 100; code <= 120; code++) {
    const result = parseErrorCode(code);
    if (result.code === code && result.name && result.message) {
      console.log(`✓ Code ${code}: ${result.name} - ${result.message}`);
      passed++;
    } else {
      console.log(`✗ Code ${code}: Failed to parse`);
      failed++;
    }
  }
  
  // Test unknown error code
  const unknownResult = parseErrorCode(999);
  if (unknownResult.name === "UnknownError") {
    console.log(`✓ Unknown code handling works correctly`);
    passed++;
  } else {
    console.log(`✗ Unknown code handling failed`);
    failed++;
  }
  
  console.log(`\n=== Results ===`);
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);
  console.log(`Success Rate: ${((passed / (passed + failed)) * 100).toFixed(2)}%`);
  
  return failed === 0;
}

/**
 * Test event parsing
 */
function testEventParsing() {
  console.log("\n=== Testing Event Parsing ===\n");
  
  const testEvents = [
    {
      name: "bet_placed",
      event: {
        topics: ["bet_placed", 1n, "GADDRESS123"],
        data: 1000n,
      },
      expected: {
        type: "bet_placed",
        marketId: 1n,
        subjectAddr: "GADDRESS123",
        data: 1000n,
      },
    },
    {
      name: "market_created",
      event: {
        topics: ["market_created", 5n, "GCREATOR456"],
        data: null,
      },
      expected: {
        type: "market_created",
        marketId: 5n,
        subjectAddr: "GCREATOR456",
        data: null,
      },
    },
    {
      name: "circuit_breaker_updated",
      event: {
        topics: ["circuit_breaker_updated"],
        data: "Open",
      },
      expected: {
        type: "circuit_breaker_updated",
        marketId: null,
        subjectAddr: null,
        data: "Open",
      },
    },
  ];
  
  let passed = 0;
  let failed = 0;
  
  testEvents.forEach(({ name, event, expected }) => {
    const result = parseEvent(event);
    const isCorrect = 
      result.type === expected.type &&
      result.marketId === expected.marketId &&
      result.subjectAddr === expected.subjectAddr &&
      result.data === expected.data;
    
    if (isCorrect) {
      console.log(`✓ ${name}: Parsed correctly`);
      passed++;
    } else {
      console.log(`✗ ${name}: Failed to parse correctly`);
      console.log(`  Expected:`, expected);
      console.log(`  Got:`, result);
      failed++;
    }
  });
  
  console.log(`\n=== Results ===`);
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);
  
  return failed === 0;
}

/**
 * Run all tests
 */
function runTests() {
  console.log("PredictIQ Frontend Mock Test\n");
  console.log("Testing error code parsing (100-120) and event schema\n");
  console.log("=".repeat(60) + "\n");
  
  const errorTestPassed = testErrorCodeParsing();
  const eventTestPassed = testEventParsing();
  
  console.log("\n" + "=".repeat(60));
  console.log("\n=== Overall Test Results ===");
  
  if (errorTestPassed && eventTestPassed) {
    console.log("✓ All tests passed!");
    console.log("\nFrontend can successfully:");
    console.log("  - Parse all error codes 100-120");
    console.log("  - Handle unknown error codes gracefully");
    console.log("  - Parse standardized event format (Topic, MarketID, SubjectAddr, Data)");
    return true;
  } else {
    console.log("✗ Some tests failed");
    if (!errorTestPassed) console.log("  - Error code parsing failed");
    if (!eventTestPassed) console.log("  - Event parsing failed");
    return false;
  }
}

// Run tests if executed directly
if (typeof module !== 'undefined' && require.main === module) {
  const success = runTests();
  process.exit(success ? 0 : 1);
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    ERROR_CODES,
    parseErrorCode,
    parseEvent,
    runTests,
  };
}
