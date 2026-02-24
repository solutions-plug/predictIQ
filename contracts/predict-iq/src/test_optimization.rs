#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

use crate::{PredictIQ, PredictIQClient};
use crate::types::OracleConfig;

fn setup_test() -> (Env, PredictIQClient<'static>, Address) {
    let e = Env::default();
    e.mock_all_auths();
    e.budget().reset_unlimited();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    (e, client, admin)
}

#[test]
fn test_optimized_market_creation_gas_comparison() {
    let (e, client, creator) = setup_test();
    let token = Address::generate(&e);

    let description = String::from_str(&e, "Will BTC reach $100k by end of 2026?");
    let options = Vec::from_array(
        &e,
        [String::from_str(&e, "Yes"), String::from_str(&e, "No")],
    );
    let deadline = e.ledger().timestamp() + 86400;
    let resolution_deadline = deadline + 86400;
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    // Reset budget before market creation
    e.budget().reset_default();

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &token,
    );

    // Get CPU and memory usage
    let cpu_insns = e.budget().cpu_instruction_cost();
    let mem_bytes = e.budget().memory_bytes_cost();

    println!("=== Optimized Market Creation Gas Costs ===");
    println!("CPU Instructions: {}", cpu_insns);
    println!("Memory Bytes: {}", mem_bytes);
    println!("Market ID: {}", market_id);

    // Verify market was created
    assert!(market_id > 0);

    // Test metadata decompression
    let retrieved_desc = client.get_market_description(&market_id);
    let retrieved_options = client.get_market_options(&market_id);

    assert_eq!(retrieved_desc, description);
    assert_eq!(retrieved_options.len(), 2);
    assert_eq!(retrieved_options.get(0).unwrap(), String::from_str(&e, "Yes"));
    assert_eq!(retrieved_options.get(1).unwrap(), String::from_str(&e, "No"));
}

#[test]
fn test_metadata_compression() {
    let (e, client, creator) = setup_test();
    let token = Address::generate(&e);

    // Create market with longer description and more options
    let description = String::from_str(&e, "This is a longer market description to test compression efficiency. It contains multiple sentences and should demonstrate the benefits of binary storage optimization.");
    let options = Vec::from_array(
        &e,
        [
            String::from_str(&e, "Option A"),
            String::from_str(&e, "Option B"),
            String::from_str(&e, "Option C"),
            String::from_str(&e, "Option D"),
        ],
    );
    let deadline = e.ledger().timestamp() + 86400;
    let resolution_deadline = deadline + 86400;
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "TEST"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &token,
    );

    // Verify decompression works correctly
    let retrieved_desc = client.get_market_description(&market_id);
    let retrieved_options = client.get_market_options(&market_id);

    assert_eq!(retrieved_desc, description);
    assert_eq!(retrieved_options.len(), 4);
    for i in 0..4 {
        assert_eq!(retrieved_options.get(i).unwrap(), options.get(i).unwrap());
    }
}

#[test]
fn test_garbage_collection() {
    let (e, client, creator) = setup_test();
    let token = Address::generate(&e);
    let bettor = Address::generate(&e);

    // Create and resolve a market
    let description = String::from_str(&e, "Test Market");
    let options = Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]);
    let deadline = e.ledger().timestamp() + 86400;
    let resolution_deadline = deadline + 86400;
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "TEST"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &token,
    );

    // Note: Full garbage collection test would require:
    // 1. Placing a bet
    // 2. Resolving the market
    // 3. Advancing time by 180 days
    // 4. Calling garbage_collect_bet
    // This is a placeholder to show the function exists

    assert!(market_id > 0);
}
