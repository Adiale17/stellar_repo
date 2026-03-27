# soroban-did

Decentralized Identity Document (DID) registry on Stellar. Addresses publish and update W3C-compatible DID documents containing public keys, service endpoints, and verification methods. Supports key rotation, controller delegation, and DID resolution. The self-sovereign identity primitive for Stellar — no central authority, no custodian, no platform dependency.

---

## The Problem

### Digital identity is owned by platforms, not people

Every system of digital identity today — email accounts, social profiles, government IDs, professional credentials — is controlled by an intermediary. The intermediary issues the identifier, stores the associated data, and can revoke access at any time. Users do not own their identity. They borrow it.

**Identifiers are platform-dependent.** Your Google identity only works while Google exists and chooses to honor it. Your LinkedIn profile only exists while LinkedIn operates. Your username on any platform disappears the moment the platform decides to remove it or shut down. There is no portable identity that travels with you across systems.

**Key management has no on-chain standard.** When a protocol on Stellar needs to verify that a message was signed by a specific address, or that an address controls a specific key, there is no standard registry to query. Every protocol implements its own key management, creating fragmentation and incompatibility.

**Delegating authority requires trusting a third party.** If you want to allow another address to act on your behalf — a smart contract agent, a corporate wallet, a guardian — there is no trustless, on-chain mechanism to express and verify that delegation. It requires off-chain agreements or centralized permission systems.

**Key rotation has no record.** When a key is compromised and rotated, there is no auditable trail of when the old key was invalidated and when the new one became authoritative. Verifiers cannot distinguish an old valid signature from a post-compromise signature.

**There is no DID infrastructure on Stellar.** The W3C DID standard is the established specification for decentralized identity. Without a Soroban implementation, Stellar protocols cannot participate in the broader DID ecosystem — they cannot issue W3C Verifiable Credentials, cannot interoperate with DID-based identity wallets, and cannot leverage existing DID tooling.

---

## The Solution

`soroban-did` implements a full W3C-compatible DID document registry on Soroban. Each Stellar address can register a DID document containing their public keys, service endpoints, and delegations. The document is owned and controlled entirely by the registrant — no admin can modify, delete, or interfere with another address's DID document. Key rotation, delegation, and deactivation are all performed by the controller through standard contract calls.

| Problem | Solution |
|---|---|
| Platform-owned identifiers | DIDs are registered on-chain and owned by the controller address |
| No standard key registry | DID documents store public keys in a standard queryable format |
| No trustless delegation | On-chain delegations with configurable expiry and capability scope |
| No key rotation record | Revoked keys stored with timestamp — full rotation history preserved |
| No DID infrastructure on Stellar | W3C-compatible DID document structure queryable by any contract |

---

## W3C DID Compatibility

`soroban-did` follows the [W3C DID Core specification](https://www.w3.org/TR/did-core/). A DID document on this registry contains:

**DID Subject** — the DID string itself, e.g. `did:stellar:GABC...`

**Verification Methods** — public keys with their type, purpose, and controller:
- `Ed25519VerificationKey2020`
- `EcdsaSecp256k1VerificationKey2019`
- `JsonWebKey2020`
- `X25519KeyAgreementKey2020`

**Verification Relationships** — the purpose of each key:
- `authentication` — proving control of the DID
- `assertionMethod` — issuing Verifiable Credentials
- `keyAgreement` — encrypted communication
- `capabilityInvocation` — invoking authorized actions
- `capabilityDelegation` — delegating to other DIDs

**Service Endpoints** — URLs and addresses for protocol-specific communication (messaging, linked domains, credential status).

**Controller** — the Stellar address that controls the DID document.

---

## How It Works

### Registration

A controller calls `register` with a DID string and an initial public key. The contract stores the DID document and creates a reverse mapping from the controller address to the DID string. One DID per address — each Stellar address controls exactly one DID.

### Key Rotation

New verification methods are added with `add_verification_method`. Old keys are revoked with `revoke_verification_method` — revoked keys remain in the document with a `revoked_at` timestamp for audit purposes. The last active key cannot be revoked, ensuring the DID document always has at least one valid authentication key.

### Controller Rotation

The controller address itself can be changed with `rotate_controller`. This transfers complete control of the DID to a new Stellar address and updates the reverse address→DID lookup atomically.

### Delegations

Delegations grant another DID the right to act with a specific capability on behalf of this DID. Delegations have configurable expiry and can be revoked at any time. Any contract can call `is_delegation_valid` to check whether a specific address holds an active delegation for a given capability.

### Deactivation

`deactivate` permanently marks a DID as inactive. Once deactivated, the DID document cannot be updated and all delegations are considered invalid. Deactivation is irreversible.

### Version Tracking

Every modification to a DID document increments the `version` counter and updates the `updated` timestamp. This provides a reliable way to detect document changes and invalidate caches.

---

## DID Document Structure
```
DIDDocument {
    did: String                           // "did:stellar:GABC..."
    controller_address: Address           // Stellar address of controller
    verification_methods: Vec<VerificationMethod>
    services: Vec<ServiceEndpoint>
    delegations: Vec<Delegation>
    status: Active | Deactivated
    created: u32                          // Ledger sequence
    updated: u32                          // Ledger sequence
    version: u32                          // Increments on every change
    metadata: String                      // Optional JSON string
}
```

---

## Contract API

### Setup
```rust
fn initialize(e: Env, admin: Address) -> Result<(), Error>
```
Initializes the registry. Called once at deployment.

---

### DID Registration
```rust
fn register(
    e: Env,
    controller: Address,
    did_string: String,
    initial_key_id: String,
    initial_key_type: KeyType,
    initial_public_key: Bytes,
    metadata: String,
) -> Result<(), Error>
```
Registers a new DID document. One DID per controller address. An initial verification method with `Authentication` purpose is required.

---

### Verification Method Management
```rust
fn add_verification_method(
    e: Env,
    controller: Address,
    did_string: String,
    key_id: String,
    key_type: KeyType,
    purpose: KeyPurpose,
    public_key: Bytes,
    key_controller: String,
) -> Result<(), Error>
```
Adds a new public key to the DID document. Maximum 20 keys per DID. Key IDs must be unique within the document.
```rust
fn revoke_verification_method(
    e: Env,
    controller: Address,
    did_string: String,
    key_id: String,
) -> Result<(), Error>
```
Revokes a verification method. Cannot revoke the last active key. Revoked keys remain in the document with a `revoked_at` timestamp.

---

### Controller Rotation
```rust
fn rotate_controller(
    e: Env,
    current_controller: Address,
    did_string: String,
    new_controller: Address,
) -> Result<(), Error>
```
Transfers DID control to a new Stellar address. Updates the reverse address→DID mapping atomically.

---

### Service Endpoints
```rust
fn add_service(
    e: Env,
    controller: Address,
    did_string: String,
    service_id: String,
    service_type: String,
    endpoint: String,
) -> Result<(), Error>
```
Adds a service endpoint. Maximum 20 services per DID.
```rust
fn remove_service(e: Env, controller: Address, did_string: String, service_id: String) -> Result<(), Error>
```
Removes a service endpoint by its fragment ID.

---

### Delegations
```rust
fn grant_delegation(
    e: Env,
    controller: Address,
    did_string: String,
    delegate_did: String,
    delegate_address: Address,
    capability: KeyPurpose,
    expires_at: u32,
) -> Result<(), Error>
```
Grants a capability delegation to another DID. `expires_at = 0` means no expiry. Maximum 10 active delegations per DID.
```rust
fn revoke_delegation(
    e: Env,
    controller: Address,
    did_string: String,
    delegate_did: String,
    capability: KeyPurpose,
) -> Result<(), Error>
```
Revokes a delegation. Revoked delegations remain in the document for auditability.

---

### Document Operations
```rust
fn update_metadata(e: Env, controller: Address, did_string: String, metadata: String) -> Result<(), Error>
```
Updates the metadata JSON string of the DID document.
```rust
fn deactivate(e: Env, controller: Address, did_string: String) -> Result<(), Error>
```
Permanently deactivates a DID. Irreversible.

---

### Resolution
```rust
fn resolve(e: Env, did_string: String) -> Result<DIDDocument, Error>
```
Returns the full DID document for a given DID string.
```rust
fn resolve_by_address(e: Env, address: Address) -> Result<DIDDocument, Error>
```
Returns the DID document controlled by a given Stellar address.
```rust
fn resolve_summary(e: Env, did_string: String) -> Result<DIDResolutionResult, Error>
```
Returns a lightweight summary — cheaper to call than full resolution when only counts and status are needed.

---

### Verification Helpers
```rust
fn is_delegation_valid(e: Env, did_string: String, delegate_address: Address, capability: KeyPurpose) -> bool
```
Returns `true` if the delegate address holds an active, non-expired delegation for the given capability. The primary integration point for consuming contracts.
```rust
fn is_key_active(e: Env, did_string: String, key_id: String) -> bool
```
Returns `true` if the specified key exists and is not revoked.
```rust
fn did_count(e: Env) -> u32
```

---

### Admin
```rust
fn set_paused(e: Env, caller: Address, paused: bool) -> Result<(), Error>
```
Pauses or unpauses new DID registration and updates. Admin only.

---

## Development

### Prerequisites

- Rust stable toolchain
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli)

### Build
```bash
cargo build --release --target wasm32-unknown-unknown
```

Output: `target/wasm32-unknown-unknown/release/soroban_did.wasm`

### Test
```bash
cargo test
```

| Test | Description |
|---|---|
| `test_initialize` | Registry initializes with zero DIDs |
| `test_double_initialize_rejected` | Second init reverts |
| `test_register_did` | DID registered with correct document structure |
| `test_duplicate_did_rejected` | Duplicate DID string reverts |
| `test_one_did_per_address` | Same controller cannot register two DIDs |
| `test_resolve_by_address` | Reverse lookup returns correct DID |
| `test_resolve_summary` | Summary returns correct counts |
| `test_add_verification_method` | New key added to document |
| `test_duplicate_key_id_rejected` | Duplicate key fragment ID reverts |
| `test_revoke_verification_method` | Key revoked with timestamp recorded |
| `test_cannot_revoke_last_key` | Revoking last key reverts |
| `test_rotate_controller` | Control transferred to new address |
| `test_rotate_controller_updates_reverse_lookup` | Old address lookup fails after rotation |
| `test_add_service` | Service endpoint added to document |
| `test_remove_service` | Service endpoint removed |
| `test_duplicate_service_rejected` | Duplicate service ID reverts |
| `test_grant_delegation` | Delegation granted and queryable |
| `test_delegation_expires` | Expired delegation returns false |
| `test_revoke_delegation` | Revoked delegation returns false |
| `test_update_metadata` | Metadata updated and version incremented |
| `test_update_metadata_unauthorized_rejected` | Non-controller update reverts |
| `test_deactivate_did` | DID status set to Deactivated |
| `test_deactivated_did_cannot_be_updated` | Update on deactivated DID reverts |
| `test_deactivated_did_blocks_delegation` | Delegation check returns false on deactivated DID |
| `test_version_increments_on_each_update` | Version counter increments on every change |
| `test_pause_blocks_registration` | Paused registry rejects new registrations |
| `test_full_did_lifecycle` | Complete register → add keys → add services → delegate → rotate → revoke → deactivate flow |

---

## Deployment

### Step 1 — Add Testnet Network
```bash
soroban network add \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015" \
  testnet
```

### Step 2 — Generate and Fund a Keypair
```bash
stellar keys generate deployer --network testnet --fund
stellar keys address deployer
```

### Step 3 — Build
```bash
cargo build --release --target wasm32-unknown-unknown
```

### Step 4 — Deploy
```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/soroban_did.wasm \
  --source-account deployer \
  --network testnet
```

### Step 5 — Initialize
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- initialize \
  --admin $(stellar keys address deployer)
```

### Step 6 — Register a DID
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- register \
  --controller $(stellar keys address deployer) \
  --did_string "did:stellar:$(stellar keys address deployer)" \
  --initial_key_id "key-1" \
  --initial_key_type Ed25519VerificationKey2020 \
  --initial_public_key <HEX_PUBLIC_KEY> \
  --metadata "{}"
```

### Step 7 — Add a Service Endpoint
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- add_service \
  --controller $(stellar keys address deployer) \
  --did_string "did:stellar:$(stellar keys address deployer)" \
  --service_id "messaging" \
  --service_type "DIDCommMessaging" \
  --endpoint "https://messaging.example.com"
```

### Step 8 — Resolve a DID
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- resolve \
  --did_string "did:stellar:$(stellar keys address deployer)"
```

### Step 9 — Grant a Delegation
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- grant_delegation \
  --controller $(stellar keys address deployer) \
  --did_string "did:stellar:$(stellar keys address deployer)" \
  --delegate_did "did:stellar:<DELEGATE_ADDRESS>" \
  --delegate_address <DELEGATE_ADDRESS> \
  --capability Authentication \
  --expires_at 0
```

### Step 10 — Check Delegation Validity
```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source-account deployer \
  --network testnet \
  -- is_delegation_valid \
  --did_string "did:stellar:$(stellar keys address deployer)" \
  --delegate_address <DELEGATE_ADDRESS> \
  --capability Authentication
```

---

## Project Structure
```
.
├── src/
│   ├── lib.rs       # DID document registry, key management, delegation logic
│   └── test.rs      # Unit tests
├── Cargo.toml
└── README.md
```
