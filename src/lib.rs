#![no_std]
extern crate alloc;

use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype, Address, Bytes, Env,
    String, Vec,
};

contractmeta!(
    key = "Description",
    val = "SorobanDID: W3C-compatible decentralized identity document registry with key rotation, controller delegation, and DID resolution on Soroban"
);

#[contracterror]
#[repr(u32)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    DIDNotFound = 4,
    DIDAlreadyExists = 5,
    DIDDeactivated = 6,
    KeyNotFound = 7,
    KeyAlreadyExists = 8,
    ServiceNotFound = 9,
    ServiceAlreadyExists = 10,
    ControllerNotFound = 11,
    ControllerAlreadyExists = 12,
    InvalidKey = 13,
    InvalidService = 14,
    Paused = 15,
    MaxKeysReached = 16,
    MaxServicesReached = 17,
    MaxControllersReached = 18,
    CannotRemoveLastKey = 19,
    CannotRemoveLastController = 20,
    DelegationNotFound = 21,
    DelegationExpired = 22,
}

/// Key type for verification methods
#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum KeyType {
    Ed25519VerificationKey2020 = 0,
    EcdsaSecp256k1VerificationKey2019 = 1,
    JsonWebKey2020 = 2,
    X25519KeyAgreementKey2020 = 3,
}

/// Purpose of a verification method
#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum KeyPurpose {
    Authentication = 0,
    AssertionMethod = 1,
    KeyAgreement = 2,
    CapabilityInvocation = 3,
    CapabilityDelegation = 4,
}

/// Status of a DID document
#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum DIDStatus {
    Active = 1,
    Deactivated = 2,
}

/// A verification method (public key) in the DID document
#[contracttype]
#[derive(Clone)]
pub struct VerificationMethod {
    /// Fragment identifier - e.g. "key-1"
    pub id: String,
    pub key_type: KeyType,
    pub purpose: KeyPurpose,
    /// Public key bytes (raw)
    pub public_key: Bytes,
    /// Controller of this specific key (defaults to DID subject)
    pub controller: String,
    /// Whether this key is currently active
    pub active: bool,
    /// Ledger when key was added
    pub added_at: u32,
    /// Ledger when key was revoked (0 if still active)
    pub revoked_at: u32,
}

/// A service endpoint in the DID document
#[contracttype]
#[derive(Clone)]
pub struct ServiceEndpoint {
    /// Fragment identifier - e.g. "messaging"
    pub id: String,
    /// Service type - e.g. "MessagingService"
    pub service_type: String,
    /// Endpoint URL or address
    pub endpoint: String,
    /// Ledger when added
    pub added_at: u32,
}

/// A controller delegation - allows another DID to act on behalf of this DID
#[contracttype]
#[derive(Clone)]
pub struct Delegation {
    /// DID string of the delegate
    pub delegate_did: String,
    /// Stellar address of the delegate
    pub delegate_address: Address,
    /// Specific capability being delegated
    pub capability: KeyPurpose,
    /// Ledger when delegation expires (0 = no expiry)
    pub expires_at: u32,
    /// Ledger when delegation was granted
    pub granted_at: u32,
    pub active: bool,
}

/// Core DID document
#[contracttype]
#[derive(Clone)]
pub struct DIDDocument {
    /// The DID string - e.g. "did:stellar:GABC..."
    pub did: String,
    /// Stellar address that controls this DID
    pub controller_address: Address,
    /// Verification methods (public keys)
    pub verification_methods: Vec<VerificationMethod>,
    /// Service endpoints
    pub services: Vec<ServiceEndpoint>,
    /// Active delegations to other DIDs
    pub delegations: Vec<Delegation>,
    pub status: DIDStatus,
    /// Ledger when DID was created
    pub created: u32,
    /// Ledger when DID document was last updated
    pub updated: u32,
    /// Version counter - increments on every update
    pub version: u32,
    /// Optional metadata (JSON string)
    pub metadata: String,
}

/// Lightweight resolution result for external consumers
#[contracttype]
#[derive(Clone)]
pub struct DIDResolutionResult {
    pub did: String,
    pub controller_address: Address,
    pub status: DIDStatus,
    pub created: u32,
    pub updated: u32,
    pub version: u32,
    pub key_count: u32,
    pub service_count: u32,
    pub delegation_count: u32,
}

#[contracttype]
pub enum StorageKey {
    Admin,
    Paused,
    DIDCount,
    /// DID string -> DIDDocument
    DID(String),
    /// Stellar address -> DID string (reverse lookup)
    AddressToDID(Address),
}

const MAX_KEYS: u32 = 20;
const MAX_SERVICES: u32 = 20;
const MAX_CONTROLLERS: u32 = 10;

#[contract]
pub struct SorobanDID;

#[contractimpl]
impl SorobanDID {
    /// Initialize the DID registry.
    pub fn initialize(e: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        if e.storage().instance().has(&StorageKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        e.storage().instance().set(&StorageKey::Admin, &admin);
        e.storage().instance().set(&StorageKey::Paused, &false);
        e.storage().instance().set(&StorageKey::DIDCount, &0u32);

        Ok(())
    }

    /// Register a new DID document.
    /// The caller becomes the controller of the DID.
    /// `did_string` is the full DID - e.g. "did:stellar:GABC..."
    /// An initial verification method must be provided.
    pub fn register(
        e: Env,
        controller: Address,
        did_string: String,
        initial_key_id: String,
        initial_key_type: KeyType,
        initial_public_key: Bytes,
        metadata: String,
    ) -> Result<(), Error> {
        controller.require_auth();
        Self::require_not_paused(&e)?;

        if e.storage()
            .instance()
            .has(&StorageKey::DID(did_string.clone()))
        {
            return Err(Error::DIDAlreadyExists);
        }

        // One DID per address
        if e.storage()
            .instance()
            .has(&StorageKey::AddressToDID(controller.clone()))
        {
            return Err(Error::DIDAlreadyExists);
        }

        if initial_public_key.len() == 0 {
            return Err(Error::InvalidKey);
        }

        let current = e.ledger().sequence();

        let initial_key = VerificationMethod {
            id: initial_key_id,
            key_type: initial_key_type,
            purpose: KeyPurpose::Authentication,
            public_key: initial_public_key,
            controller: did_string.clone(),
            active: true,
            added_at: current,
            revoked_at: 0,
        };

        let mut verification_methods = Vec::new(&e);
        verification_methods.push_back(initial_key);

        let doc = DIDDocument {
            did: did_string.clone(),
            controller_address: controller.clone(),
            verification_methods,
            services: Vec::new(&e),
            delegations: Vec::new(&e),
            status: DIDStatus::Active,
            created: current,
            updated: current,
            version: 1,
            metadata,
        };

        let count: u32 = e.storage().instance().get(&StorageKey::DIDCount).unwrap_or(0);

        e.storage()
            .instance()
            .set(&StorageKey::DID(did_string.clone()), &doc);
        e.storage()
            .instance()
            .set(&StorageKey::AddressToDID(controller), &did_string);
        e.storage()
            .instance()
            .set(&StorageKey::DIDCount, &(count + 1));

        Ok(())
    }

    /// Add a new verification method (public key) to a DID document.
    pub fn add_verification_method(
        e: Env,
        controller: Address,
        did_string: String,
        key_id: String,
        key_type: KeyType,
        purpose: KeyPurpose,
        public_key: Bytes,
        key_controller: String,
    ) -> Result<(), Error> {
        controller.require_auth();
        Self::require_not_paused(&e)?;

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        if doc.verification_methods.len() >= MAX_KEYS {
            return Err(Error::MaxKeysReached);
        }
        if public_key.len() == 0 {
            return Err(Error::InvalidKey);
        }

        // Check for duplicate key ID
        for i in 0..doc.verification_methods.len() {
            let existing = doc.verification_methods.get(i).unwrap();
            if existing.id == key_id {
                return Err(Error::KeyAlreadyExists);
            }
        }

        let current = e.ledger().sequence();
        let vm = VerificationMethod {
            id: key_id,
            key_type,
            purpose,
            public_key,
            controller: key_controller,
            active: true,
            added_at: current,
            revoked_at: 0,
        };

        doc.verification_methods.push_back(vm);
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Revoke a verification method by its fragment ID.
    /// Cannot revoke the last active key.
    pub fn revoke_verification_method(
        e: Env,
        controller: Address,
        did_string: String,
        key_id: String,
    ) -> Result<(), Error> {
        controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        // Count active keys before revoking
        let active_count = doc.verification_methods.iter().filter(|k| k.active).count();

        if active_count <= 1 {
            return Err(Error::CannotRemoveLastKey);
        }

        let current = e.ledger().sequence();
        let mut found = false;

        let mut new_methods = Vec::new(&e);
        for i in 0..doc.verification_methods.len() {
            let mut vm = doc.verification_methods.get(i).unwrap();
            if vm.id == key_id && vm.active {
                vm.active = false;
                vm.revoked_at = current;
                found = true;
            }
            new_methods.push_back(vm);
        }

        if !found {
            return Err(Error::KeyNotFound);
        }

        doc.verification_methods = new_methods;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Rotate the controller key - transfer DID control to a new Stellar address.
    pub fn rotate_controller(
        e: Env,
        current_controller: Address,
        did_string: String,
        new_controller: Address,
    ) -> Result<(), Error> {
        current_controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &current_controller)?;
        Self::require_active(&doc)?;

        let current = e.ledger().sequence();

        // Update reverse lookup
        e.storage()
            .instance()
            .remove(&StorageKey::AddressToDID(current_controller));
        e.storage()
            .instance()
            .set(&StorageKey::AddressToDID(new_controller.clone()), &did_string);

        doc.controller_address = new_controller;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Add a service endpoint to the DID document.
    pub fn add_service(
        e: Env,
        controller: Address,
        did_string: String,
        service_id: String,
        service_type: String,
        endpoint: String,
    ) -> Result<(), Error> {
        controller.require_auth();
        Self::require_not_paused(&e)?;

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        if doc.services.len() >= MAX_SERVICES {
            return Err(Error::MaxServicesReached);
        }

        // Check duplicate service ID
        for i in 0..doc.services.len() {
            let svc = doc.services.get(i).unwrap();
            if svc.id == service_id {
                return Err(Error::ServiceAlreadyExists);
            }
        }

        let current = e.ledger().sequence();
        let service = ServiceEndpoint {
            id: service_id,
            service_type,
            endpoint,
            added_at: current,
        };

        doc.services.push_back(service);
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Remove a service endpoint by its fragment ID.
    pub fn remove_service(
        e: Env,
        controller: Address,
        did_string: String,
        service_id: String,
    ) -> Result<(), Error> {
        controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        let mut found = false;
        let mut new_services = Vec::new(&e);

        for i in 0..doc.services.len() {
            let svc = doc.services.get(i).unwrap();
            if svc.id == service_id {
                found = true;
            } else {
                new_services.push_back(svc);
            }
        }

        if !found {
            return Err(Error::ServiceNotFound);
        }

        let current = e.ledger().sequence();
        doc.services = new_services;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Grant a delegation to another DID address.
    pub fn grant_delegation(
        e: Env,
        controller: Address,
        did_string: String,
        delegate_did: String,
        delegate_address: Address,
        capability: KeyPurpose,
        expires_at: u32,
    ) -> Result<(), Error> {
        controller.require_auth();
        Self::require_not_paused(&e)?;

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        if doc.delegations.len() >= MAX_CONTROLLERS {
            return Err(Error::MaxControllersReached);
        }

        // Check no duplicate delegation for same delegate+capability
        for i in 0..doc.delegations.len() {
            let d = doc.delegations.get(i).unwrap();
            if d.delegate_did == delegate_did && d.capability as u32 == capability as u32 && d.active {
                return Err(Error::ControllerAlreadyExists);
            }
        }

        let current = e.ledger().sequence();
        if expires_at > 0 && expires_at <= current {
            return Err(Error::DelegationExpired);
        }

        let delegation = Delegation {
            delegate_did,
            delegate_address,
            capability,
            expires_at,
            granted_at: current,
            active: true,
        };

        doc.delegations.push_back(delegation);
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Revoke a delegation.
    pub fn revoke_delegation(
        e: Env,
        controller: Address,
        did_string: String,
        delegate_did: String,
        capability: KeyPurpose,
    ) -> Result<(), Error> {
        controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        let mut found = false;
        let mut new_delegations = Vec::new(&e);
        let current = e.ledger().sequence();

        for i in 0..doc.delegations.len() {
            let mut d = doc.delegations.get(i).unwrap();
            if d.delegate_did == delegate_did && d.capability as u32 == capability as u32 && d.active {
                d.active = false;
                found = true;
            }
            new_delegations.push_back(d);
        }

        if !found {
            return Err(Error::DelegationNotFound);
        }

        doc.delegations = new_delegations;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Update the DID document metadata.
    pub fn update_metadata(
        e: Env,
        controller: Address,
        did_string: String,
        metadata: String,
    ) -> Result<(), Error> {
        controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        let current = e.ledger().sequence();
        doc.metadata = metadata;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Deactivate a DID permanently.
    /// Once deactivated a DID cannot be reactivated.
    pub fn deactivate(e: Env, controller: Address, did_string: String) -> Result<(), Error> {
        controller.require_auth();

        let mut doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        Self::require_controller(&doc, &controller)?;
        Self::require_active(&doc)?;

        let current = e.ledger().sequence();
        doc.status = DIDStatus::Deactivated;
        doc.updated = current;
        doc.version += 1;

        e.storage().instance().set(&StorageKey::DID(did_string), &doc);

        Ok(())
    }

    /// Resolve a DID document by its DID string.
    pub fn resolve(e: Env, did_string: String) -> Result<DIDDocument, Error> {
        e.storage()
            .instance()
            .get(&StorageKey::DID(did_string))
            .ok_or(Error::DIDNotFound)
    }

    /// Resolve a lightweight summary of a DID - cheaper to call than full resolve.
    pub fn resolve_summary(e: Env, did_string: String) -> Result<DIDResolutionResult, Error> {
        let doc: DIDDocument = e
            .storage()
            .instance()
            .get(&StorageKey::DID(did_string.clone()))
            .ok_or(Error::DIDNotFound)?;

        let active_keys = doc.verification_methods.iter().filter(|k| k.active).count();

        Ok(DIDResolutionResult {
            did: doc.did,
            controller_address: doc.controller_address,
            status: doc.status,
            created: doc.created,
            updated: doc.updated,
            version: doc.version,
            key_count: active_keys as u32,
            service_count: doc.services.len(),
            delegation_count: doc.delegations.iter().filter(|d| d.active).count() as u32,
        })
    }

    /// Resolve a DID by its controller Stellar address.
    pub fn resolve_by_address(e: Env, address: Address) -> Result<DIDDocument, Error> {
        let did_string: String = e
            .storage()
            .instance()
            .get(&StorageKey::AddressToDID(address))
            .ok_or(Error::DIDNotFound)?;

        e.storage()
            .instance()
            .get(&StorageKey::DID(did_string))
            .ok_or(Error::DIDNotFound)
    }

    /// Check if a delegation is currently valid.
    pub fn is_delegation_valid(
        e: Env,
        did_string: String,
        delegate_address: Address,
        capability: KeyPurpose,
    ) -> bool {
        let doc: DIDDocument = match e.storage().instance().get(&StorageKey::DID(did_string)) {
            Some(d) => d,
            None => return false,
        };

        if doc.status == DIDStatus::Deactivated {
            return false;
        }

        let current = e.ledger().sequence();

        for i in 0..doc.delegations.len() {
            let d = doc.delegations.get(i).unwrap();
            if d.delegate_address == delegate_address
                && d.capability as u32 == capability as u32
                && d.active
                && (d.expires_at == 0 || current <= d.expires_at)
            {
                return true;
            }
        }

        false
    }

    /// Check if a specific key is currently active for a DID.
    pub fn is_key_active(e: Env, did_string: String, key_id: String) -> bool {
        let doc: DIDDocument = match e.storage().instance().get(&StorageKey::DID(did_string)) {
            Some(d) => d,
            None => return false,
        };

        for i in 0..doc.verification_methods.len() {
            let vm = doc.verification_methods.get(i).unwrap();
            if vm.id == key_id && vm.active {
                return true;
            }
        }

        false
    }

    /// Get total DID count.
    pub fn did_count(e: Env) -> u32 {
        e.storage()
            .instance()
            .get(&StorageKey::DIDCount)
            .unwrap_or(0)
    }

    /// Pause or unpause. Admin only.
    pub fn set_paused(e: Env, caller: Address, paused: bool) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&e, &caller)?;
        e.storage().instance().set(&StorageKey::Paused, &paused);
        Ok(())
    }

    // --- Internal helpers ---

    fn require_controller(doc: &DIDDocument, caller: &Address) -> Result<(), Error> {
        if doc.controller_address != *caller {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    fn require_active(doc: &DIDDocument) -> Result<(), Error> {
        if doc.status == DIDStatus::Deactivated {
            return Err(Error::DIDDeactivated);
        }
        Ok(())
    }

    fn require_admin(e: &Env, caller: &Address) -> Result<(), Error> {
        let admin: Address = e
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if *caller != admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    fn require_not_paused(e: &Env) -> Result<(), Error> {
        let paused: bool = e
            .storage()
            .instance()
            .get(&StorageKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::Paused);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
