#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Bytes, Env, String,
};

fn setup() -> (Env, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.sequence_number = 100);

    let admin = Address::generate(&e);
    let contract_id = e.register(SorobanDID, ());
    let client = SorobanDIDClient::new(&e, &contract_id);

    client.initialize(&admin);

    (e, contract_id, admin)
}

fn register_did(e: &Env, contract_id: &Address, controller: &Address, did_suffix: &str) -> String {
    let client = SorobanDIDClient::new(e, contract_id);
    let did = String::from_str(e, &alloc::format!("did:stellar:{}", did_suffix));
    let key_id = String::from_str(e, "key-1");
    let public_key = Bytes::from_slice(e, &[1u8; 32]);

    client.register(
        controller,
        &did,
        &key_id,
        &KeyType::Ed25519VerificationKey2020,
        &public_key,
        &String::from_str(e, "{}"),
    );

    did
}

#[test]
fn test_initialize() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);
    assert_eq!(client.did_count(), 0);
}

#[test]
fn test_double_initialize_rejected() {
    let (e, contract_id, admin) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);
    let result = client.try_initialize(&admin);
    assert!(result.is_err());
}

#[test]
fn test_register_did() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let doc = client.resolve(&did);
    assert_eq!(doc.did, did);
    assert_eq!(doc.controller_address, controller);
    assert_eq!(doc.status, DIDStatus::Active);
    assert_eq!(doc.version, 1);
    assert_eq!(doc.verification_methods.len(), 1);
    assert_eq!(client.did_count(), 1);
}

#[test]
fn test_duplicate_did_rejected() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let result = client.try_register(
        &controller,
        &did,
        &String::from_str(&e, "key-2"),
        &KeyType::Ed25519VerificationKey2020,
        &Bytes::from_slice(&e, &[2u8; 32]),
        &String::from_str(&e, "{}"),
    );
    assert!(result.is_err());
}

#[test]
fn test_one_did_per_address() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    register_did(&e, &contract_id, &controller, "GABC123");

    let did2 = String::from_str(&e, "did:stellar:GDEF456");
    let result = client.try_register(
        &controller,
        &did2,
        &String::from_str(&e, "key-1"),
        &KeyType::Ed25519VerificationKey2020,
        &Bytes::from_slice(&e, &[1u8; 32]),
        &String::from_str(&e, "{}"),
    );
    assert!(result.is_err());
}

#[test]
fn test_resolve_by_address() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let doc = client.resolve_by_address(&controller);
    assert_eq!(doc.did, did);
}

#[test]
fn test_resolve_summary() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let summary = client.resolve_summary(&did);
    assert_eq!(summary.did, did);
    assert_eq!(summary.key_count, 1);
    assert_eq!(summary.service_count, 0);
    assert_eq!(summary.version, 1);
}

#[test]
fn test_add_verification_method() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-2"),
        &KeyType::EcdsaSecp256k1VerificationKey2019,
        &KeyPurpose::AssertionMethod,
        &Bytes::from_slice(&e, &[2u8; 32]),
        &did,
    );

    let doc = client.resolve(&did);
    assert_eq!(doc.verification_methods.len(), 2);
    assert_eq!(doc.version, 2);
}

#[test]
fn test_duplicate_key_id_rejected() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let result = client.try_add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-1"),
        &KeyType::Ed25519VerificationKey2020,
        &KeyPurpose::Authentication,
        &Bytes::from_slice(&e, &[3u8; 32]),
        &did,
    );
    assert!(result.is_err());
}

#[test]
fn test_revoke_verification_method() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-2"),
        &KeyType::Ed25519VerificationKey2020,
        &KeyPurpose::AssertionMethod,
        &Bytes::from_slice(&e, &[2u8; 32]),
        &did,
    );

    client.revoke_verification_method(&controller, &did, &String::from_str(&e, "key-1"));

    let doc = client.resolve(&did);
    let key1 = doc.verification_methods.get(0).unwrap();
    assert!(!key1.active);
    assert!(key1.revoked_at > 0);

    assert!(!client.is_key_active(&did, &String::from_str(&e, "key-1")));
    assert!(client.is_key_active(&did, &String::from_str(&e, "key-2")));
}

#[test]
fn test_cannot_revoke_last_key() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let result = client.try_revoke_verification_method(&controller, &did, &String::from_str(&e, "key-1"));
    assert!(result.is_err());
}

#[test]
fn test_rotate_controller() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let old_controller = Address::generate(&e);
    let new_controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &old_controller, "GABC123");

    client.rotate_controller(&old_controller, &did, &new_controller);

    let doc = client.resolve(&did);
    assert_eq!(doc.controller_address, new_controller);
    assert_eq!(doc.version, 2);

    let result = client.try_update_metadata(&old_controller, &did, &String::from_str(&e, "new"));
    assert!(result.is_err());

    let result2 = client.try_update_metadata(&new_controller, &did, &String::from_str(&e, "new"));
    assert!(result2.is_ok());
}

#[test]
fn test_rotate_controller_updates_reverse_lookup() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let old_controller = Address::generate(&e);
    let new_controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &old_controller, "GABC123");

    client.rotate_controller(&old_controller, &did, &new_controller);

    let doc = client.resolve_by_address(&new_controller);
    assert_eq!(doc.controller_address, new_controller);

    let result = client.try_resolve_by_address(&old_controller);
    assert!(result.is_err());
}

#[test]
fn test_add_service() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "messaging"),
        &String::from_str(&e, "MessagingService"),
        &String::from_str(&e, "https://messaging.example.com"),
    );

    let doc = client.resolve(&did);
    assert_eq!(doc.services.len(), 1);
    assert_eq!(doc.version, 2);

    let summary = client.resolve_summary(&did);
    assert_eq!(summary.service_count, 1);
}

#[test]
fn test_remove_service() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "messaging"),
        &String::from_str(&e, "MessagingService"),
        &String::from_str(&e, "https://messaging.example.com"),
    );

    client.remove_service(&controller, &did, &String::from_str(&e, "messaging"));

    let doc = client.resolve(&did);
    assert_eq!(doc.services.len(), 0);
}

#[test]
fn test_duplicate_service_rejected() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "messaging"),
        &String::from_str(&e, "MessagingService"),
        &String::from_str(&e, "https://messaging.example.com"),
    );

    let result = client.try_add_service(
        &controller,
        &did,
        &String::from_str(&e, "messaging"),
        &String::from_str(&e, "Other"),
        &String::from_str(&e, "https://other.com"),
    );
    assert!(result.is_err());
}

#[test]
fn test_grant_delegation() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let delegate = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");
    let delegate_did = String::from_str(&e, "did:stellar:DELEGATE");

    client.grant_delegation(
        &controller,
        &did,
        &delegate_did,
        &delegate,
        &KeyPurpose::Authentication,
        &0u32,
    );

    let doc = client.resolve(&did);
    assert_eq!(doc.delegations.len(), 1);
    assert_eq!(doc.version, 2);

    assert!(client.is_delegation_valid(&did, &delegate, &KeyPurpose::Authentication));
}

#[test]
fn test_delegation_expires() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let delegate = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.grant_delegation(
        &controller,
        &did,
        &String::from_str(&e, "did:stellar:DEL"),
        &delegate,
        &KeyPurpose::AssertionMethod,
        &200u32,
    );

    assert!(client.is_delegation_valid(&did, &delegate, &KeyPurpose::AssertionMethod));

    e.ledger().with_mut(|l| l.sequence_number = 201);

    assert!(!client.is_delegation_valid(&did, &delegate, &KeyPurpose::AssertionMethod));
}

#[test]
fn test_revoke_delegation() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let delegate = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");
    let delegate_did = String::from_str(&e, "did:stellar:DEL");

    client.grant_delegation(
        &controller,
        &did,
        &delegate_did,
        &delegate,
        &KeyPurpose::CapabilityInvocation,
        &0u32,
    );

    assert!(client.is_delegation_valid(
        &did,
        &delegate,
        &KeyPurpose::CapabilityInvocation
    ));

    client.revoke_delegation(
        &controller,
        &did,
        &delegate_did,
        &KeyPurpose::CapabilityInvocation,
    );

    assert!(!client.is_delegation_valid(
        &did,
        &delegate,
        &KeyPurpose::CapabilityInvocation
    ));
}

#[test]
fn test_update_metadata() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let new_metadata = String::from_str(&e, r#"{"name":"Alice","bio":"Developer"}"#);
    client.update_metadata(&controller, &did, &new_metadata);

    let doc = client.resolve(&did);
    assert_eq!(doc.metadata, new_metadata);
    assert_eq!(doc.version, 2);
}

#[test]
fn test_update_metadata_unauthorized_rejected() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let rando = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    let result = client.try_update_metadata(&rando, &did, &String::from_str(&e, "{}"));
    assert!(result.is_err());
}

#[test]
fn test_deactivate_did() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.deactivate(&controller, &did);

    let doc = client.resolve(&did);
    assert_eq!(doc.status, DIDStatus::Deactivated);
}

#[test]
fn test_deactivated_did_cannot_be_updated() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.deactivate(&controller, &did);

    let result = client.try_update_metadata(&controller, &did, &String::from_str(&e, "{}"));
    assert!(result.is_err());
}

#[test]
fn test_deactivated_did_blocks_delegation() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let delegate = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    client.deactivate(&controller, &did);

    assert!(!client.is_delegation_valid(
        &did,
        &delegate,
        &KeyPurpose::Authentication
    ));
}

#[test]
fn test_version_increments_on_each_update() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "GABC123");

    assert_eq!(client.resolve(&did).version, 1);

    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "svc-1"),
        &String::from_str(&e, "Type"),
        &String::from_str(&e, "https://example.com"),
    );
    assert_eq!(client.resolve(&did).version, 2);

    client.update_metadata(&controller, &did, &String::from_str(&e, "{}"));
    assert_eq!(client.resolve(&did).version, 3);

    client.add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-2"),
        &KeyType::Ed25519VerificationKey2020,
        &KeyPurpose::AssertionMethod,
        &Bytes::from_slice(&e, &[2u8; 32]),
        &did,
    );
    assert_eq!(client.resolve(&did).version, 4);
}

#[test]
fn test_pause_blocks_registration() {
    let (e, contract_id, admin) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    client.set_paused(&admin, &true);

    let controller = Address::generate(&e);
    let did = String::from_str(&e, "did:stellar:PAUSED");

    let result = client.try_register(
        &controller,
        &did,
        &String::from_str(&e, "key-1"),
        &KeyType::Ed25519VerificationKey2020,
        &Bytes::from_slice(&e, &[1u8; 32]),
        &String::from_str(&e, "{}"),
    );
    assert!(result.is_err());
}

#[test]
fn test_full_did_lifecycle() {
    let (e, contract_id, _) = setup();
    let client = SorobanDIDClient::new(&e, &contract_id);

    let controller = Address::generate(&e);
    let did = register_did(&e, &contract_id, &controller, "ALICE123");
    assert_eq!(client.resolve(&did).version, 1);

    client.add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-assertion"),
        &KeyType::Ed25519VerificationKey2020,
        &KeyPurpose::AssertionMethod,
        &Bytes::from_slice(&e, &[2u8; 32]),
        &did,
    );

    client.add_verification_method(
        &controller,
        &did,
        &String::from_str(&e, "key-agreement"),
        &KeyType::X25519KeyAgreementKey2020,
        &KeyPurpose::KeyAgreement,
        &Bytes::from_slice(&e, &[3u8; 32]),
        &did,
    );

    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "messaging"),
        &String::from_str(&e, "DIDCommMessaging"),
        &String::from_str(&e, "https://alice.example.com/msg"),
    );
    client.add_service(
        &controller,
        &did,
        &String::from_str(&e, "profile"),
        &String::from_str(&e, "LinkedDomains"),
        &String::from_str(&e, "https://alice.example.com"),
    );

    let delegate_addr = Address::generate(&e);
    client.grant_delegation(
        &controller,
        &did,
        &String::from_str(&e, "did:stellar:BOB456"),
        &delegate_addr,
        &KeyPurpose::CapabilityDelegation,
        &500u32,
    );
    assert!(client.is_delegation_valid(
        &did,
        &delegate_addr,
        &KeyPurpose::CapabilityDelegation
    ));

    client.update_metadata(
        &controller,
        &did,
        &String::from_str(&e, r#"{"name":"Alice","role":"Developer"}"#),
    );

    let summary = client.resolve_summary(&did);
    assert_eq!(summary.key_count, 3);
    assert_eq!(summary.service_count, 2);
    assert_eq!(summary.delegation_count, 1);

    let new_controller = Address::generate(&e);
    client.rotate_controller(&controller, &did, &new_controller);

    let doc = client.resolve(&did);
    assert_eq!(doc.controller_address, new_controller);

    client.revoke_verification_method(&new_controller, &did, &String::from_str(&e, "key-agreement"));
    assert!(!client.is_key_active(&did, &String::from_str(&e, "key-agreement")));

    client.revoke_delegation(
        &new_controller,
        &did,
        &String::from_str(&e, "did:stellar:BOB456"),
        &KeyPurpose::CapabilityDelegation,
    );
    assert!(!client.is_delegation_valid(
        &did,
        &delegate_addr,
        &KeyPurpose::CapabilityDelegation
    ));

    let final_doc = client.resolve(&did);
    assert!(final_doc.version > 1);
    assert_eq!(final_doc.status, DIDStatus::Active);
    assert_eq!(client.did_count(), 1);
}
