#![cfg(test)]
use super::admin::*;
use crate::errors::ErrorCode;
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register_contract(None, PredictIQ);
    (e, contract_id)
}

#[test]
fn test_set_and_get_admin() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        let stored_admin = get_admin(&e).unwrap();
        assert_eq!(stored_admin, admin);
    });
}

#[test]
fn test_require_admin_success() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        let result = require_admin(&e);
        assert!(result.is_ok());
    });
}

#[test]
fn test_require_admin_not_set() {
    let (e, contract_id) = setup();

    e.as_contract(&contract_id, || {
        let result = require_admin(&e);
        assert_eq!(result, Err(ErrorCode::NotAuthorized));
    });
}

#[test]
fn test_set_and_get_guardian() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();
        let stored_guardian = get_guardian(&e).unwrap();
        assert_eq!(stored_guardian, guardian);
    });
}

#[test]
fn test_require_guardian_success() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();
        let result = require_guardian(&e);
        assert!(result.is_ok());
    });
}

#[test]
fn test_require_guardian_not_set() {
    let (e, contract_id) = setup();

    e.as_contract(&contract_id, || {
        let result = require_guardian(&e);
        assert_eq!(result, Err(ErrorCode::NotAuthorized));
    });
}

#[test]
fn test_admin_can_change_guardian() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian1.clone()).unwrap();
        assert_eq!(get_guardian(&e).unwrap(), guardian1);
        set_guardian(&e, guardian2.clone()).unwrap();
        assert_eq!(get_guardian(&e).unwrap(), guardian2);
    });
}

#[test]
fn test_admin_and_guardian_are_independent() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();
        assert_eq!(get_admin(&e).unwrap(), admin);
        assert_eq!(get_guardian(&e).unwrap(), guardian);
        assert_ne!(admin, guardian);
    });
}

// --- Separation of powers: admin-transfer path (issue #1538) ---
//
// `governance::initialize_guardians` and `add_guardian` reject a guardian
// address that equals the current Admin. The admin-transfer path
// (`propose_admin` / `accept_admin`) must enforce the symmetric invariant:
// an incoming admin that is already in the guardian set must be rejected so
// that no address can hold both roles simultaneously.

#[test]
fn test_propose_admin_rejects_existing_guardian() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();

        // The guardian is already in the guardian set, so proposing it as the
        // incoming admin must be rejected to preserve separation of powers.
        let result = propose_admin(&e, guardian.clone());
        assert_eq!(result, Err(ErrorCode::NotAuthorized));
    });
}

#[test]
fn test_accept_admin_rejects_existing_guardian() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();

        // Even if a pending admin was somehow set, accepting it while the
        // address is a guardian must be rejected.
        let result = accept_admin(&e, guardian.clone());
        assert_eq!(result, Err(ErrorCode::NotAuthorized));
    });
}

#[test]
fn test_accept_admin_allows_non_guardian() {
    let (e, contract_id) = setup();
    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&contract_id, || {
        set_admin(&e, admin.clone());
        set_guardian(&e, guardian.clone()).unwrap();

        // A non-guardian incoming admin is allowed to complete the transfer.
        propose_admin(&e, new_admin.clone()).unwrap();
        accept_admin(&e, new_admin.clone()).unwrap();
        assert_eq!(get_admin(&e).unwrap(), new_admin);
        assert_eq!(get_guardian(&e).unwrap(), guardian);
    });
}
