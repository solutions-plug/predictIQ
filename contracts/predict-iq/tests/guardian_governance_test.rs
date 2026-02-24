#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Vec,
};

mod test_guardian_governance {
    use super::*;
    use predict_iq::{PredictIQ, PredictIQClient};

    fn setup_test() -> (Env, PredictIQClient<'static>, Address, Vec<Address>) {
        let e = Env::default();
        e.mock_all_auths();

        let admin = Address::generate(&e);
        let contract_id = e.register(PredictIQ, ());
        let client = PredictIQClient::new(&e, &contract_id);

        client.initialize(&admin, &1000);

        // Create 5 guardians
        let guardians = Vec::from_array(
            &e,
            [
                Address::generate(&e),
                Address::generate(&e),
                Address::generate(&e),
                Address::generate(&e),
                Address::generate(&e),
            ],
        );

        client.set_guardians(&guardians);

        (e, client, admin, guardians)
    }

    #[test]
    fn test_two_of_five_guardians_admin_unchanged() {
        let (e, client, admin, guardians) = setup_test();

        let new_admin = Address::generate(&e);

        // 2 guardians sign
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(1).unwrap(), &new_admin);

        // Admin should remain unchanged
        assert_eq!(client.get_admin(), Some(admin));

        // Recovery should not be active (needs 3/5)
        assert_eq!(client.is_recovery_active(), false);
    }

    #[test]
    fn test_three_of_five_guardians_recovery_active() {
        let (e, client, admin, guardians) = setup_test();

        let new_admin = Address::generate(&e);

        // 3 guardians sign
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(1).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(2).unwrap(), &new_admin);

        // Recovery should be active
        assert_eq!(client.is_recovery_active(), true);

        // Admin should still be old
        assert_eq!(client.get_admin(), Some(admin));

        // Recovery state should exist
        let recovery = client.get_recovery_state();
        assert!(recovery.is_some());
        assert_eq!(recovery.unwrap().new_admin, new_admin);
    }

    #[test]
    fn test_finalize_recovery_fails_before_72_hours() {
        let (e, client, _admin, guardians) = setup_test();

        let new_admin = Address::generate(&e);

        // 3 guardians sign
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(1).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(2).unwrap(), &new_admin);

        // Try to finalize after 24 hours (86400 seconds)
        e.ledger().set(LedgerInfo {
            timestamp: e.ledger().timestamp() + 86400,
            protocol_version: 22,
            sequence_number: e.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 3110400,
        });

        // Should fail - timelock not expired
        let result = client.try_finalize_recovery();
        assert!(result.is_err());
    }

    #[test]
    fn test_finalize_recovery_succeeds_after_72_hours() {
        let (e, client, old_admin, guardians) = setup_test();

        let new_admin = Address::generate(&e);

        // 3 guardians sign
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(1).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(2).unwrap(), &new_admin);

        // Verify old admin is still active
        assert_eq!(client.get_admin(), Some(old_admin));

        // Advance time by 72 hours (259200 seconds)
        e.ledger().set(LedgerInfo {
            timestamp: e.ledger().timestamp() + 259200,
            protocol_version: 22,
            sequence_number: e.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 3110400,
        });

        // Should succeed
        let new_admin_result = client.try_finalize_recovery();
        assert!(new_admin_result.is_ok());

        // Admin should be updated
        assert_eq!(client.get_admin(), Some(new_admin));

        // Recovery state should be cleared
        assert_eq!(client.get_recovery_state(), None);
        assert_eq!(client.is_recovery_active(), false);
    }

    #[test]
    fn test_all_five_guardians_sign() {
        let (e, client, _admin, guardians) = setup_test();

        let new_admin = Address::generate(&e);

        // All 5 guardians sign
        for i in 0..5 {
            client.sign_reset_admin(&guardians.get(i).unwrap(), &new_admin);
        }

        // Recovery should be active
        assert_eq!(client.is_recovery_active(), true);

        // Advance time by 72 hours
        e.ledger().set(LedgerInfo {
            timestamp: e.ledger().timestamp() + 259200,
            protocol_version: 22,
            sequence_number: e.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 3110400,
        });

        // Should succeed
        let result = client.try_finalize_recovery();
        assert!(result.is_ok());
        assert_eq!(client.get_admin(), Some(new_admin));
    }

    #[test]
    fn test_guardian_cannot_sign_twice() {
        let (_e, client, _admin, guardians) = setup_test();

        let new_admin = Address::generate(&_e);

        // Guardian signs twice
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);
        client.sign_reset_admin(&guardians.get(0).unwrap(), &new_admin);

        // Should only count once
        let recovery = client.get_recovery_state().unwrap();
        assert_eq!(recovery.approvals.len(), 1);
    }
}
