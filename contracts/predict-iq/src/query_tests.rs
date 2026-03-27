#![cfg(test)]

use crate::{PredictIQ, PredictIQClient};
use crate::types::{MarketStatus, OracleConfig, MarketTier};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_paginated_markets() {
    let e = Env::default();
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    client.initialize(&admin, &0);

    // Create 10 markets
    for i in 1..=10 {
        let options = Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]);
        let oracle_cfg = OracleConfig {
            oracle_address: Address::generate(&e),
            feed_id: String::from_str(&e, "test"),
            min_responses: None,
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        
        client.create_market(
            &creator,
            &String::from_str(&e, &format!("Market {}", i)),
            &options,
            &(100 + i as u64),
            &(200 + i as u64),
            &oracle_cfg,
            &MarketTier::Basic,
            &native_token,
            &0,
            &0
        );
    }

    // Test Offset/Limit Pagination
    let page1 = client.get_markets(&0, &5);
    assert_eq!(page1.len(), 5);
    assert_eq!(page1.get(0).unwrap().id, 1);
    assert_eq!(page1.get(4).unwrap().id, 5);

    let page2 = client.get_markets(&5, &5);
    assert_eq!(page2.len(), 5);
    assert_eq!(page2.get(0).unwrap().id, 6);
    assert_eq!(page2.get(4).unwrap().id, 10);

    let page3 = client.get_markets(&10, &5);
    assert_eq!(page3.len(), 0);

    // Test Out of Bounds
    let overflow = client.get_markets(&100, &5);
    assert_eq!(overflow.len(), 0);
}

#[test]
fn test_paginated_archived_markets() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    client.initialize(&admin, &0);

    // Create and Prune 5 markets
    let mut market_ids = Vec::new(&e);
    for _ in 0..5 {
        let options = Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]);
        let oracle_cfg = OracleConfig {
            oracle_address: Address::generate(&e),
            feed_id: String::from_str(&e, "test"),
            min_responses: None,
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        
        let id = client.create_market(
            &creator,
            &String::from_str(&e, "Archive Test"),
            &options,
            &100,
            &200,
            &oracle_cfg,
            &MarketTier::Basic,
            &native_token,
            &0,
            &0
        );
        market_ids.push_back(id);
    }

    // Manually resolve and prune
    for id in market_ids.iter() {
        client.set_oracle_result(&id, &0);
        client.resolve_market(&id, &0);
        
        // Jump forward in time to allow pruning (grace period is 30 days)
        e.ledger().set_timestamp(e.ledger().timestamp() + 2_592_001);
        client.prune_market(&id);
    }

    let archived_ids = client.get_archived_market_ids(&0, &10);
    assert_eq!(archived_ids.len(), 5);
    assert_eq!(archived_ids.get(0).unwrap(), 1);
    assert_eq!(archived_ids.get(4).unwrap(), 5);
}

#[test]
fn test_status_based_pagination() {
    let e = Env::default();
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    client.initialize(&admin, &0);

    // Create 3 active markets
    for _ in 0..3 {
        let options = Vec::from_array(&e, [String::from_str(&e, "A"), String::from_str(&e, "B")]);
        let oracle_cfg = OracleConfig {
            oracle_address: Address::generate(&e),
            feed_id: String::from_str(&e, "test"),
            min_responses: None,
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        client.create_market(&creator, &String::from_str(&e, "Active"), &options, &100, &200, &oracle_cfg, &MarketTier::Basic, &native_token, &0, &0);
    }

    let active_page = client.get_markets_by_status(&MarketStatus::Active, &0, &10);
    assert_eq!(active_page.len(), 3);
}
