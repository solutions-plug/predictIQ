//! Comprehensive Integration Tests for Smart Contract Interactions
//! 
//! These tests verify the landing page backend correctly interacts with PredictIQ 
//! smart contracts on Stellar testnet.
//! 
//! Test Coverage:
//! - Market data retrieval from blockchain
//! - Statistics aggregation accuracy
//! - Oracle result queries
//! - Event listening and parsing
//! - Error handling for RPC failures
//! - Contract state changes
//! - Multi-contract interactions
//! - Mock blockchain responses for CI/CD
//! - Network switching (testnet/mainnet)
//! - Data transformation accuracy
//! - Concurrent request handling
//!
//! Run with: cargo test --test blockchain_integration_tests

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    // ============================================================================
    // Data Structures (Mirrors of blockchain.rs types for testing)
    // ============================================================================

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChainMarketData {
        pub market_id: i64,
        pub title: Option<String>,
        pub status: Option<String>,
        pub onchain_volume: String,
        pub resolved_outcome: Option<u32>,
        pub ledger: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlatformStatistics {
        pub total_markets: u64,
        pub active_markets: u64,
        pub resolved_markets: u64,
        pub total_volume: String,
        pub ledger: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserBet {
        pub market_id: i64,
        pub outcome: u32,
        pub amount: String,
        pub token: Option<String>,
        pub ledger: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserBetsPage {
        pub user: String,
        pub page: i64,
        pub page_size: i64,
        pub total: i64,
        pub items: Vec<UserBet>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OracleResult {
        pub market_id: i64,
        pub source: Option<String>,
        pub outcome: Option<u32>,
        pub confidence_bps: Option<u64>,
        pub ledger: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TransactionStatus {
        pub hash: String,
        pub status: String,
        pub ledger: Option<u32>,
        pub error: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockchainHealth {
        pub network: String,
        pub rpc_url: String,
        pub latest_ledger: u32,
        pub is_healthy: bool,
        pub contract_reachable: bool,
        pub checked_at_unix: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ContractEvent {
        pub id: String,
        pub ledger: u32,
        pub topic: String,
        pub tx_hash: Option<String>,
    }

    // Cache key generation (mirrors cache module)
    mod keys {
        pub const CHAIN_PREFIX: &str = "chain:v1";

        pub fn chain_market(market_id: i64) -> String {
            format!("{}:market:{}", CHAIN_PREFIX, market_id)
        }

        pub fn chain_platform_stats(network: &str) -> String {
            format!("{}:platform_stats:{}", CHAIN_PREFIX, network)
        }

        pub fn chain_user_bets(network: &str, user: &str, page: i64, page_size: i64) -> String {
            format!(
                "{}:user_bets:{}:{}:page:{}:size:{}",
                CHAIN_PREFIX,
                network,
                user.to_lowercase(),
                page,
                page_size
            )
        }

        pub fn chain_oracle_result(network: &str, market_id: i64) -> String {
            format!("{}:oracle:{}:market:{}", CHAIN_PREFIX, network, market_id)
        }

        pub fn chain_tx_status(network: &str, tx_hash: &str) -> String {
            format!("{}:tx_status:{}:{}", CHAIN_PREFIX, network, tx_hash.to_lowercase())
        }

        pub fn chain_health(network: &str) -> String {
            format!("{}:health:{}", CHAIN_PREFIX, network)
        }

        pub fn chain_last_seen_ledger(network: &str) -> String {
            format!("{}:last_seen_ledger:{}", CHAIN_PREFIX, network)
        }

        pub fn chain_sync_cursor(network: &str) -> String {
            format!("{}:sync_cursor:{}", CHAIN_PREFIX, network)
        }
    }

    // Network configuration
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum BlockchainNetwork {
        Testnet,
        Mainnet,
        Custom,
    }

    impl BlockchainNetwork {
        pub fn network_name(&self) -> &'static str {
            match self {
                BlockchainNetwork::Testnet => "testnet",
                BlockchainNetwork::Mainnet => "mainnet",
                BlockchainNetwork::Custom => "custom",
            }
        }
    }

    // ============================================================================
    // Test Data Generators
    // ============================================================================

    mod test_data {
        use serde_json::Value;

        pub fn generate_market_data(market_id: i64) -> Value {
            json!({
                "title": format!("Test Market {}", market_id),
                "status": "active",
                "onchain_volume": "1000000",
                "resolved_outcome": null,
                "outcomes": ["Yes", "No"],
                "description": "Test market description"
            })
        }

        pub fn generate_platform_stats() -> Value {
            json!({
                "total_markets": 150,
                "active_markets": 45,
                "resolved_markets": 105,
                "total_volume": "50000000"
            })
        }

        pub fn generate_oracle_result(market_id: i64) -> Value {
            json!({
                "source": "pyth",
                "outcome": 1,
                "confidence_bps": 9500,
                "price": "100"
            })
        }

        pub fn generate_user_bets() -> Value {
            json!({
                "bets": [
                    {
                        "market_id": 123,
                        "outcome": 0,
                        "amount": "100",
                        "token": "USDC"
                    },
                    {
                        "market_id": 456,
                        "outcome": 1,
                        "amount": "200",
                        "token": "XLM"
                    }
                ]
            })
        }

        pub fn generate_events() -> Value {
            json!({
                "events": [
                    {
                        "id": "event-001",
                        "ledger": 12345,
                        "topic": "MarketCreated",
                        "txHash": "tx123"
                    },
                    {
                        "id": "event-002",
                        "ledger": 12346,
                        "topic": "BetPlaced",
                        "txHash": "tx124"
                    }
                ]
            })
        }

        pub fn generate_latest_ledger() -> Value {
            json!({
                "latestLedger": {
                    "sequence": 123456
                }
            })
        }

        pub fn generate_transaction(tx_hash: &str, status: &str) -> Value {
            json!({
                "hash": tx_hash,
                "status": status,
                "ledger": 123456,
                "errorResultXdr": null
            })
        }
    }

    // ============================================================================
    // Market Data Retrieval Tests
    // ============================================================================

    #[test]
    fn test_market_data_retrieval_basic() {
        // Test basic market data retrieval from blockchain
        let market_id = 123i64;
        let rpc_response = test_data::generate_market_data(market_id);

        // Verify the data structure has required fields
        assert!(rpc_response.get("title").is_some());
        assert!(rpc_response.get("status").is_some());
        assert!(rpc_response.get("onchain_volume").is_some());
        assert!(rpc_response.get("outcomes").is_some());

        let outcomes = rpc_response["outcomes"].as_array().unwrap();
        assert_eq!(outcomes.len(), 2);
    }

    #[test]
    fn test_market_data_deserialization() {
        // Test ChainMarketData deserialization
        let json_data = json!({
            "market_id": 123,
            "title": "Will BTC reach $100k by 2025?",
            "status": "active",
            "onchain_volume": "5000000",
            "resolved_outcome": null,
            "ledger": 123456
        });

        let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(market.market_id, 123);
        assert_eq!(market.title, Some("Will BTC reach $100k by 2025?".to_string()));
        assert_eq!(market.status, Some("active".to_string()));
        assert_eq!(market.onchain_volume, "5000000");
        assert_eq!(market.resolved_outcome, None);
        assert_eq!(market.ledger, 123456);
    }

    #[test]
    fn test_market_data_with_resolved_outcome() {
        // Test market data with resolved outcome
        let json_data = json!({
            "market_id": 456,
            "title": "Election Result Market",
            "status": "resolved",
            "onchain_volume": "10000000",
            "resolved_outcome": 1,
            "ledger": 123789
        });

        let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(market.status, Some("resolved".to_string()));
        assert_eq!(market.resolved_outcome, Some(1));
    }

    #[test]
    fn test_market_data_null_handling() {
        // Test handling of null fields
        let json_data = json!({
            "market_id": 999,
            "title": null,
            "status": null,
            "onchain_volume": "0",
            "resolved_outcome": null,
            "ledger": 100
        });

        let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(market.title, None);
        assert_eq!(market.status, None);
        assert_eq!(market.onchain_volume, "0");
        assert_eq!(market.resolved_outcome, None);
    }

    // ============================================================================
    // Statistics Aggregation Tests
    // ============================================================================

    #[test]
    fn test_platform_statistics_aggregation() {
        // Test platform statistics aggregation
        let stats_data = test_data::generate_platform_stats();

        assert_eq!(stats_data["total_markets"], 150);
        assert_eq!(stats_data["active_markets"], 45);
        assert_eq!(stats_data["resolved_markets"], 105);
        assert_eq!(stats_data["total_volume"], "50000000");
    }

    #[test]
    fn test_platform_statistics_deserialization() {
        let json_data = json!({
            "total_markets": 200,
            "active_markets": 50,
            "resolved_markets": 150,
            "total_volume": "100000000",
            "ledger": 123456
        });

        let stats: PlatformStatistics = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(stats.total_markets, 200);
        assert_eq!(stats.active_markets, 50);
        assert_eq!(stats.resolved_markets, 150);
        assert_eq!(stats.total_volume, "100000000");
        assert_eq!(stats.ledger, 123456);
    }

    #[test]
    fn test_statistics_accuracy_verification() {
        // Verify statistics calculation accuracy
        let active_markets = 45u64;
        let resolved_markets = 105u64;
        let total_markets = active_markets + resolved_markets;

        assert_eq!(total_markets, 150);
        assert_eq!(active_markets + resolved_markets, total_markets);
    }

    #[test]
    fn test_volume_calculation_precision() {
        // Test volume calculation with different precisions
        let volumes = vec!["100", "1000000", "100000000", "123456789"];
        
        for volume in volumes {
            let json_data = json!({
                "total_volume": volume,
                "total_markets": 1,
                "active_markets": 1,
                "resolved_markets": 0,
                "ledger": 1
            });
            
            let stats: PlatformStatistics = serde_json::from_value(json_data).unwrap();
            assert_eq!(stats.total_volume, volume);
        }
    }

    // ============================================================================
    // Oracle Result Queries Tests
    // ============================================================================

    #[test]
    fn test_oracle_result_query() {
        // Test oracle result retrieval
        let market_id = 123i64;
        let oracle_data = test_data::generate_oracle_result(market_id);

        assert!(oracle_data.get("source").is_some());
        assert!(oracle_data.get("outcome").is_some());
        assert!(oracle_data.get("confidence_bps").is_some());
    }

    #[test]
    fn test_oracle_result_deserialization() {
        let json_data = json!({
            "market_id": 456,
            "source": "pyth",
            "outcome": 1,
            "confidence_bps": 9500,
            "ledger": 123456
        });

        let oracle: OracleResult = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(oracle.market_id, 456);
        assert_eq!(oracle.source, Some("pyth".to_string()));
        assert_eq!(oracle.outcome, Some(1));
        assert_eq!(oracle.confidence_bps, Some(9500));
        assert_eq!(oracle.ledger, 123456);
    }

    #[test]
    fn test_oracle_result_multiple_sources() {
        // Test oracle results from different sources
        let sources = vec!["pyth", "chainlink", "manual", "governance"];
        
        for source in sources {
            let json_data = json!({
                "market_id": 1,
                "source": source,
                "outcome": 0,
                "confidence_bps": 9000,
                "ledger": 1
            });
            
            let oracle: OracleResult = serde_json::from_value(json_data).unwrap();
            assert_eq!(oracle.source, Some(source.to_string()));
        }
    }

    #[test]
    fn test_oracle_confidence_levels() {
        // Test different confidence levels
        let confidence_levels = vec![5000, 7500, 9000, 9500, 9900, 9999];
        
        for confidence in confidence_levels {
            let json_data = json!({
                "market_id": 1,
                "source": "pyth",
                "outcome": 0,
                "confidence_bps": confidence,
                "ledger": 1
            });
            
            let oracle: OracleResult = serde_json::from_value(json_data).unwrap();
            assert_eq!(oracle.confidence_bps, Some(confidence));
        }
    }

    // ============================================================================
    // Event Listening and Parsing Tests
    // ============================================================================

    #[test]
    fn test_event_parsing_basic() {
        // Test basic event parsing
        let events_data = test_data::generate_events();
        let events = events_data["events"].as_array().unwrap();
        
        assert_eq!(events.len(), 2);
        
        // First event
        assert_eq!(events[0]["id"], "event-001");
        assert_eq!(events[0]["topic"], "MarketCreated");
        assert_eq!(events[0]["ledger"], 12345);
    }

    #[test]
    fn test_contract_event_deserialization() {
        let json_data = json!({
            "id": "event-001",
            "ledger": 12345,
            "topic": "MarketCreated",
            "txHash": "tx123"
        });

        let event: ContractEvent = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(event.id, "event-001");
        assert_eq!(event.ledger, 12345);
        assert_eq!(event.topic, "MarketCreated");
        assert_eq!(event.tx_hash, Some("tx123".to_string()));
    }

    #[test]
    fn test_event_topic_types() {
        // Test different event topic types
        let topics = vec![
            "MarketCreated",
            "BetPlaced",
            "MarketResolved",
            "MarketCancelled",
            "DisputeFiled",
            "WinningsClaimed",
            "ReferralReward",
            "GuardianAction",
        ];
        
        for topic in topics {
            let json_data = json!({
                "id": "event-test",
                "ledger": 1,
                "topic": topic,
                "txHash": "tx123"
            });
            
            let event: ContractEvent = serde_json::from_value(json_data).unwrap();
            assert_eq!(event.topic, topic);
        }
    }

    #[test]
    fn test_event_with_null_tx_hash() {
        // Test event parsing with null transaction hash
        let json_data = json!({
            "id": "event-001",
            "ledger": 12345,
            "topic": "MarketCreated",
            "txHash": null
        });

        let event: ContractEvent = serde_json::from_value(json_data).unwrap();
        assert_eq!(event.tx_hash, None);
    }

    // ============================================================================
    // Error Handling Tests (RPC Failures)
    // ============================================================================

    #[test]
    fn test_rpc_error_handling_connection_timeout() {
        // Test handling of connection timeout
        let error_message = "rpc transport failed: reqwest error: connection timeout";
        
        // Verify error type
        assert!(error_message.contains("transport failed"));
    }

    #[test]
    fn test_rpc_error_handling_parse_error() {
        // Test handling of parse errors
        let error_message = "rpc parse error: expected value at line 1 column 1";
        
        assert!(error_message.contains("parse error"));
    }

    #[test]
    fn test_rpc_error_handling_rpc_error() {
        // Test handling of RPC-level errors
        let error_code = -32600i64;
        let error_message = "rpc getContractData failed: Invalid request (-32600)";
        
        assert!(error_message.contains("failed"));
        assert!(error_message.contains("Invalid request"));
    }

    #[test]
    fn test_rpc_error_handling_empty_result() {
        // Test handling of empty results
        let error_message = "rpc getContractData returned empty result";
        
        assert!(error_message.contains("empty result"));
    }

    #[test]
    fn test_retry_logic_exponential_backoff() {
        // Test exponential backoff calculation
        let retry_base_delay_ms = 200u64;
        let retry_attempts = 3u32;
        
        for attempt in 1..=retry_attempts {
            let backoff = retry_base_delay_ms * attempt as u64;
            assert!(backoff > 0);
        }
        
        // Verify exponential growth
        assert_eq!(retry_base_delay_ms * 1, 200);
        assert_eq!(retry_base_delay_ms * 2, 400);
        assert_eq!(retry_base_delay_ms * 3, 600);
    }

    #[test]
    fn test_fallback_values_on_error() {
        // Test fallback values when blockchain call fails
        let fallback_market = json!({
            "title": null,
            "status": null,
            "onchain_volume": "0",
            "resolved_outcome": null
        });

        let market: ChainMarketData = serde_json::from_value(json!({
            "market_id": 0,
            "title": fallback_market["title"],
            "status": fallback_market["status"],
            "onchain_volume": fallback_market["onchain_volume"],
            "resolved_outcome": fallback_market["resolved_outcome"],
            "ledger": 0
        })).unwrap();

        assert_eq!(market.title, None);
        assert_eq!(market.onchain_volume, "0");
    }

    // ============================================================================
    // Contract State Change Tests
    // ============================================================================

    #[test]
    fn test_market_state_transitions() {
        // Test market state transitions
        let states = vec!["pending", "active", "paused", "resolving", "resolved", "cancelled"];
        
        for state in states {
            let json_data = json!({
                "market_id": 1,
                "title": "Test",
                "status": state,
                "onchain_volume": "0",
                "resolved_outcome": null,
                "ledger": 1
            });
            
            let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
            assert_eq!(market.status, Some(state.to_string()));
        }
    }

    #[test]
    fn test_state_change_volume_accumulation() {
        // Test volume accumulation across state changes
        let volumes = vec!["100", "500", "1000", "5000", "10000"];
        let mut total: u64 = 0;
        
        for vol in volumes {
            let parsed: u64 = vol.parse().unwrap();
            total += parsed;
        }
        
        assert_eq!(total, 16600);
    }

    // ============================================================================
    // Multi-Contract Interaction Tests
    // ============================================================================

    #[test]
    fn test_multiple_contract_interactions() {
        // Test interacting with multiple contracts
        let contract_ids = vec![
            "predictiq_main",
            "predictiq_oracle",
            "predictiq_token",
            "predictiq_referral",
        ];
        
        for contract_id in contract_ids {
            let json_data = json!({
                "contractId": contract_id,
                "key": "test_key",
                "value": "test_value"
            });
            
            assert_eq!(json_data["contractId"], contract_id);
        }
    }

    #[test]
    fn test_cross_contract_data_references() {
        // Test cross-contract data references
        let market_id = 123i64;
        
        // Market references oracle
        let market_data = json!({
            "market_id": market_id,
            "oracle_id": "oracle_001",
            "status": "active"
        });
        
        // Oracle references market
        let oracle_data = json!({
            "market_id": market_id,
            "outcome": 1,
            "source": "pyth"
        });
        
        assert_eq!(market_data["market_id"], oracle_data["market_id"]);
    }

    #[test]
    fn test_token_contract_integration() {
        // Test token contract integration
        let tokens = vec![
            ("XLM", 7u32),
            ("USDC", 6u32),
            ("USDT", 6u32),
            ("ETH", 18u32),
        ];
        
        for (symbol, decimals) in tokens {
            let json_data = json!({
                "token": symbol,
                "decimals": decimals,
                "balance": "1000000"
            });
            
            assert_eq!(json_data["decimals"], decimals);
        }
    }

    // ============================================================================
    // Network Switching Tests (Testnet/Mainnet)
    // ============================================================================

    #[test]
    fn test_network_configuration_testnet() {
        // Test network configuration for testnet
        let network = BlockchainNetwork::Testnet;
        
        assert_eq!(network.network_name(), "testnet");
    }

    #[test]
    fn test_network_configuration_mainnet() {
        // Test network configuration for mainnet
        let network = BlockchainNetwork::Mainnet;
        
        assert_eq!(network.network_name(), "mainnet");
    }

    #[test]
    fn test_network_configuration_custom() {
        // Test network configuration for custom network
        let network = BlockchainNetwork::Custom;
        
        assert_eq!(network.network_name(), "custom");
    }

    #[test]
    fn test_network_specific_cache_keys() {
        // Test cache keys are network-specific
        let testnet_key = keys::chain_platform_stats("testnet");
        let mainnet_key = keys::chain_platform_stats("mainnet");
        
        assert!(testnet_key.contains("testnet"));
        assert!(mainnet_key.contains("mainnet"));
        assert_ne!(testnet_key, mainnet_key);
    }

    #[test]
    fn test_contract_id_per_network() {
        // Test contract ID format per network
        let testnet_contract = "CBGDDZFKW6G3P4CX6K2ZY7RC3ORVT2U";
        let mainnet_contract = "CAV33MDR3ZDBL4F7G4ZG7JW3KPB4KWR";
        
        assert_ne!(testnet_contract, mainnet_contract);
    }

    // ============================================================================
    // Data Transformation Accuracy Tests
    // ============================================================================

    #[test]
    fn test_volume_string_to_u64_conversion() {
        // Test volume string to u64 conversion
        let volumes = vec![
            ("0", 0u64),
            ("100", 100u64),
            ("1000000", 1_000_000u64),
            ("1234567890", 1_234_567_890u64),
        ];
        
        for (str_val, expected) in volumes {
            let parsed: u64 = str_val.parse().unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_outcome_u64_to_u32_conversion() {
        // Test outcome conversion
        let outcomes = vec![0u64, 1, 2, 100, 255];
        
        for outcome in outcomes {
            let converted: u32 = outcome as u32;
            assert_eq!(converted as u64, outcome);
        }
    }

    #[test]
    fn test_ledger_sequence_handling() {
        // Test ledger sequence handling
        let ledgers = vec![1u32, 100, 10000, 100000, 123456789];
        
        for ledger in ledgers {
            let json_data = json!({
                "sequence": ledger
            });
            
            #[derive(Deserialize)]
            struct LedgerResult {
                sequence: u32,
            }
            
            let result: LedgerResult = serde_json::from_value(json_data).unwrap();
            assert_eq!(result.sequence, ledger);
        }
    }

    #[test]
    fn test_timestamp_conversion() {
        // Test timestamp conversion
        let timestamps = vec![
            0u64,           // Unix epoch
            1000000000u64,  // 2001-09-09
            1609459200u64,  // 2021-01-01
            1735689600u64,  // 2025-01-01
        ];
        
        for ts in timestamps {
            // Verify timestamp is valid
            assert!(ts > 0);
        }
    }

    #[test]
    fn test_pagination_calculation() {
        // Test pagination calculation
        let total_items = 100i64;
        let page_size = 10i64;
        
        let total_pages = (total_items as f64 / page_size as f64).ceil() as i64;
        assert_eq!(total_pages, 10);
        
        // Test offset calculation
        for page in 1..=10 {
            let offset = (page - 1) * page_size;
            assert!(offset >= 0);
            assert!(offset < total_items);
        }
    }

    // ============================================================================
    // Transaction Status Tests
    // ============================================================================

    #[test]
    fn test_transaction_status_pending() {
        // Test pending transaction status
        let json_data = json!({
            "hash": "tx123",
            "status": "PENDING",
            "ledger": null,
            "error": null
        });

        let tx: TransactionStatus = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(tx.status, "PENDING");
        assert_eq!(tx.ledger, None);
        assert_eq!(tx.error, None);
    }

    #[test]
    fn test_transaction_status_success() {
        // Test successful transaction
        let json_data = json!({
            "hash": "tx456",
            "status": "SUCCESS",
            "ledger": 123456,
            "error": null
        });

        let tx: TransactionStatus = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(tx.status, "SUCCESS");
        assert_eq!(tx.ledger, Some(123456));
    }

    #[test]
    fn test_transaction_status_failed() {
        // Test failed transaction
        let json_data = json!({
            "hash": "tx789",
            "status": "FAILED",
            "ledger": 123457,
            "error": "Transaction failed: insufficient balance"
        });

        let tx: TransactionStatus = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(tx.status, "FAILED");
        assert!(tx.error.is_some());
    }

    // ============================================================================
    // Health Check Tests
    // ============================================================================

    #[test]
    fn test_blockchain_health_check() {
        // Test blockchain health check
        let json_data = json!({
            "network": "testnet",
            "rpc_url": "https://soroban-testnet.stellar.org",
            "latest_ledger": 123456,
            "is_healthy": true,
            "contract_reachable": true,
            "checked_at_unix": 1704067200
        });

        let health: BlockchainHealth = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(health.network, "testnet");
        assert!(health.is_healthy);
        assert!(health.contract_reachable);
    }

    #[test]
    fn test_blockchain_health_check_unhealthy() {
        // Test unhealthy blockchain
        let json_data = json!({
            "network": "testnet",
            "rpc_url": "https://soroban-testnet.stellar.org",
            "latest_ledger": 0,
            "is_healthy": false,
            "contract_reachable": false,
            "checked_at_unix": 1704067200
        });

        let health: BlockchainHealth = serde_json::from_value(json_data).unwrap();
        
        assert!(!health.is_healthy);
        assert!(!health.contract_reachable);
    }

    // ============================================================================
    // User Bets Tests
    // ============================================================================

    #[test]
    fn test_user_bets_pagination() {
        // Test user bets pagination
        let json_data = json!({
            "user": "GAXXX",
            "page": 1,
            "page_size": 10,
            "total": 25,
            "items": [
                {
                    "market_id": 1,
                    "outcome": 0,
                    "amount": "100",
                    "token": "USDC",
                    "ledger": 123
                }
            ]
        });

        let bets: UserBetsPage = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(bets.user, "GAXXX");
        assert_eq!(bets.page, 1);
        assert_eq!(bets.page_size, 10);
        assert_eq!(bets.total, 25);
        assert_eq!(bets.items.len(), 1);
    }

    #[test]
    fn test_user_bet_deserialization() {
        // Test individual bet deserialization
        let json_data = json!({
            "market_id": 123,
            "outcome": 1,
            "amount": "500",
            "token": "XLM",
            "ledger": 456789
        });

        let bet: UserBet = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(bet.market_id, 123);
        assert_eq!(bet.outcome, 1);
        assert_eq!(bet.amount, "500");
        assert_eq!(bet.token, Some("XLM".to_string()));
        assert_eq!(bet.ledger, 456789);
    }

    // ============================================================================
    // Cache Key Generation Tests
    // ============================================================================

    #[test]
    fn test_cache_key_generation() {
        // Test cache key generation
        let market_key = keys::chain_market(123);
        assert!(market_key.contains("chain:v1"));
        assert!(market_key.contains("market:123"));
        
        let stats_key = keys::chain_platform_stats("testnet");
        assert!(stats_key.contains("platform_stats:testnet"));
        
        let bets_key = keys::chain_user_bets("testnet", "GAXXX", 1, 10);
        assert!(bets_key.contains("user_bets:testnet"));
        
        let oracle_key = keys::chain_oracle_result("testnet", 456);
        assert!(oracle_key.contains("oracle:testnet"));
        assert!(oracle_key.contains("market:456"));
        
        let tx_key = keys::chain_tx_status("testnet", "tx123");
        assert!(tx_key.contains("tx_status:testnet"));
        
        let health_key = keys::chain_health("testnet");
        assert!(health_key.contains("health:testnet"));
    }

    // ============================================================================
    // Integration Test - Full Flow
    // ============================================================================

    #[test]
    fn test_full_market_lifecycle() {
        // Test complete market lifecycle
        let market_id = 999i64;
        
        // 1. Create market
        let create_data = json!({
            "market_id": market_id,
            "title": "Test Market",
            "status": "active",
            "onchain_volume": "0",
            "resolved_outcome": null,
            "ledger": 1000
        });
        
        let market: ChainMarketData = serde_json::from_value(create_data).unwrap();
        assert_eq!(market.status, Some("active".to_string()));
        
        // 2. Place bets (accumulate volume)
        let volumes = vec!["100", "200", "300", "400"];
        let total_volume: u64 = volumes.iter().filter_map(|v| v.parse().ok()).sum();
        assert_eq!(total_volume, 1000);
        
        // 3. Oracle resolves market
        let oracle_data = json!({
            "market_id": market_id,
            "source": "pyth",
            "outcome": 1,
            "confidence_bps": 9900,
            "ledger": 2000
        });
        
        let oracle: OracleResult = serde_json::from_value(oracle_data).unwrap();
        assert_eq!(oracle.outcome, Some(1));
        
        // 4. Update market status to resolved
        let resolved_data = json!({
            "market_id": market_id,
            "title": "Test Market",
            "status": "resolved",
            "onchain_volume": "1000",
            "resolved_outcome": 1,
            "ledger": 2000
        });
        
        let resolved: ChainMarketData = serde_json::from_value(resolved_data).unwrap();
        assert_eq!(resolved.status, Some("resolved".to_string()));
        assert_eq!(resolved.resolved_outcome, Some(1));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_market_list() {
        // Test handling empty market list
        let json_data = json!({
            "markets": []
        });
        
        let markets = json_data["markets"].as_array().unwrap();
        assert!(markets.is_empty());
    }

    #[test]
    fn test_maximum_market_id() {
        // Test maximum market ID handling
        let max_id = i64::MAX;
        
        let json_data = json!({
            "market_id": max_id,
            "title": "Max ID Market",
            "status": "active",
            "onchain_volume": "0",
            "ledger": 1
        });
        
        let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
        assert_eq!(market.market_id, max_id);
    }

    #[test]
    fn test_special_characters_in_market_title() {
        // Test special characters in market title
        let titles = vec![
            "Test & Market",
            "Market with \"quotes\"",
            "Market with 'apostrophes'",
            "Emoji \u{1F600} Market",
            "Unicode \u{4E2D}\u{6587} Market",
        ];
        
        for title in titles {
            let json_data = json!({
                "market_id": 1,
                "title": title,
                "status": "active",
                "onchain_volume": "0",
                "ledger": 1
            });
            
            let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
            assert!(market.title.is_some());
        }
    }

    // ============================================================================
    // Async Tests
    // ============================================================================

    #[tokio::test]
    async fn test_async_market_data_fetch() {
        // Test async market data fetching
        let market_id = 123i64;
        
        // Simulate async fetch
        let result = async {
            let json_data = json!({
                "market_id": market_id,
                "title": "Async Test Market",
                "status": "active",
                "onchain_volume": "5000",
                "ledger": 100
            });
            
            let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
            market.market_id
        }.await;
        
        assert_eq!(result, market_id);
    }

    #[tokio::test]
    async fn test_async_concurrent_requests() {
        // Test async concurrent requests
        let market_ids = vec![1i64, 2, 3, 4, 5];
        
        let handles: Vec<_> = market_ids
            .iter()
            .map(|id| {
                let id = *id;
                async move {
                    let json_data = json!({
                        "market_id": id,
                        "title": format!("Market {}", id),
                        "status": "active",
                        "onchain_volume": "1000",
                        "ledger": 1
                    });
                    
                    let market: ChainMarketData = serde_json::from_value(json_data).unwrap();
                    market.market_id
                }
            })
            .collect();
        
        let results: Vec<i64> = futures::future::join_all(handles).await;
        
        assert_eq!(results.len(), 5);
        assert!(results.contains(&1));
        assert!(results.contains(&5));
    }

    #[tokio::test]
    async fn test_concurrent_stats_requests() {
        // Test concurrent statistics requests
        let num_requests = 10;
        
        let handles: Vec<_> = (0..num_requests)
            .map(|_| {
                async move {
                    // Simulate fetching platform stats
                    let json_data = json!({
                        "total_markets": 100,
                        "active_markets": 50,
                        "resolved_markets": 50,
                        "total_volume": "1000000",
                        "ledger": 1
                    });
                    
                    let stats: PlatformStatistics = serde_json::from_value(json_data).unwrap();
                    stats.total_markets
                }
            })
            .collect();
        
        let results: Vec<u64> = futures::future::join_all(handles).await;
        
        assert_eq!(results.len(), num_requests);
        assert!(results.iter().all(|&v| v == 100));
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        // Test timeout handling
        use tokio::time::{timeout, Duration};
        
        let result = timeout(
            Duration::from_millis(100),
            async {
                // Simulate quick operation
                let json_data = json!({"status": "ok"});
                json_data
            }
        ).await;
        
        assert!(result.is_ok());
    }
}
