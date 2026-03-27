#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Env,
};

// ─── Storage Key Types ────────────────────────────────────────────────────────

/// All top-level storage keys used by the Coin contract.
#[contracttype]
pub enum DataKey {
    /// Maps Address → token balance (in smallest unit)
    Balance(Address),
    /// The admin/issuer address — only they can mint tokens
    Admin,
    /// Total supply of Coin tokens ever minted
    TotalSupply,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct Coin;

#[contractimpl]
impl Coin {
    // ── Initialisation ───────────────────────────────────────────────────────

    /// Set the admin (campus treasury / institution) once at deploy time.
    /// Prevents re-initialisation by checking for existing admin key.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialised");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalSupply, &0_i128);
    }

    // ── Minting (Admin only) ─────────────────────────────────────────────────

    /// Mint Coin tokens to a student or contributor.
    /// Only the admin (e.g. campus treasury) may call this.
    pub fn mint(env: Env, to: Address, amount: i128) {
        // Require admin signature — no one else can issue campus currency
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Credit the recipient's balance
        let current: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(current + amount));

        // Update total supply
        let supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(supply + amount));
    }

    // ── Core MVP Transaction: Tip / Transfer ─────────────────────────────────

    /// Send Coin tokens from one campus user to another.
    /// This is the single transaction that proves the concept end-to-end:
    /// a student tips a peer tutor, pays for a freelance gig, or rewards
    /// a creator — all settled on Stellar with no intermediary.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // The sender must sign this transaction — prevents unauthorised spending
        from.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Debit sender — panic if funds are insufficient
        let from_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);

        if from_balance < amount {
            panic!("insufficient balance");
        }

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_balance - amount));

        // Credit recipient
        let to_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_balance + amount));
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Returns the Coin balance for any address.
    pub fn balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(account))
            .unwrap_or(0)
    }

    /// Returns the total supply of Coin tokens minted so far.
    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }
}

mod test;
