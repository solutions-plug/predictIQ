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
