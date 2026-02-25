#![cfg(test)]
use super::admin::*;
use crate::errors::ErrorCode;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_set_and_get_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);

    set_admin(&e, admin.clone());

    let stored_admin = get_admin(&e).unwrap();
    assert_eq!(stored_admin, admin);
}

#[test]
fn test_require_admin_success() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    set_admin(&e, admin.clone());

    let result = require_admin(&e);
    assert!(result.is_ok());
}

#[test]
fn test_require_admin_not_set() {
    let e = Env::default();

    let result = require_admin(&e);
    assert_eq!(result, Err(ErrorCode::NotAuthorized));
}

#[test]
fn test_set_and_get_guardian() {
    let e = Env::default();
    let guardian = Address::generate(&e);

    set_guardian(&e, guardian.clone()).unwrap();

    let stored_guardian = get_guardian(&e).unwrap();
    assert_eq!(stored_guardian, guardian);
}

#[test]
fn test_require_guardian_success() {
    let e = Env::default();
    e.mock_all_auths();

    let guardian = Address::generate(&e);
    set_guardian(&e, guardian.clone()).unwrap();

    let result = require_guardian(&e);
    assert!(result.is_ok());
}

#[test]
fn test_require_guardian_not_set() {
    let e = Env::default();

    let result = require_guardian(&e);
    assert_eq!(result, Err(ErrorCode::NotAuthorized));
}

#[test]
fn test_admin_can_change_guardian() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);

    set_admin(&e, admin.clone());
    set_guardian(&e, guardian1.clone()).unwrap();

    assert_eq!(get_guardian(&e).unwrap(), guardian1);

    // Change guardian
    set_guardian(&e, guardian2.clone()).unwrap();
    assert_eq!(get_guardian(&e).unwrap(), guardian2);
}

#[test]
fn test_admin_and_guardian_are_independent() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);

    set_admin(&e, admin.clone());
    set_guardian(&e, guardian.clone()).unwrap();

    assert_eq!(get_admin(&e).unwrap(), admin);
    assert_eq!(get_guardian(&e).unwrap(), guardian);
    assert_ne!(admin, guardian);
}
