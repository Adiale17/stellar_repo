#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use crate::{Coin, CoinClient};

    /// Helper: deploy contract, init with admin, return (env, client, admin).
    fn setup() -> (Env, CoinClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, Coin);
        let client = CoinClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init(&admin);

        (env, client, admin)
    }

    // ── Test 1: Happy Path ────────────────────────────────────────────────────
    // Admin mints tokens to a student; student tips a peer tutor successfully.

    #[test]
    fn test_happy_path_tip_peer_tutor() {
        let (env, client, _admin) = setup();

        let student = Address::generate(&env);
        let tutor   = Address::generate(&env);

        // Admin mints 100 Coin to the student
        client.mint(&student, &100_i128);

        // Student tips the tutor 30 Coin for a tutoring session
        client.transfer(&student, &tutor, &30_i128);

        // Student should have 70 left; tutor should have 30
        assert_eq!(client.balance(&student), 70);
        assert_eq!(client.balance(&tutor),   30);
    }

    // ── Test 2: Edge Case ─────────────────────────────────────────────────────
    // A transfer with insufficient balance must be rejected.

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_transfer_fails_when_insufficient_balance() {
        let (env, client, _admin) = setup();

        let student = Address::generate(&env);
        let creator = Address::generate(&env);

        // Student has only 10 Coin
        client.mint(&student, &10_i128);

        // Attempting to send 50 Coin must panic
        client.transfer(&student, &creator, &50_i128);
    }

    // ── Test 3: State Verification ────────────────────────────────────────────
    // After mint + transfer, balances and total supply reflect correct state.

    #[test]
    fn test_state_correct_after_mint_and_transfer() {
        let (env, client, _admin) = setup();

        let alice = Address::generate(&env);
        let bob   = Address::generate(&env);

        // Mint 200 Coin to Alice and 50 Coin to Bob
        client.mint(&alice, &200_i128);
        client.mint(&bob,   &50_i128);

        // Total supply must be 250
        assert_eq!(client.total_supply(), 250);

        // Alice pays Bob 80 Coin for a freelance gig
        client.transfer(&alice, &bob, &80_i128);

        // Alice: 200 - 80 = 120 | Bob: 50 + 80 = 130
        assert_eq!(client.balance(&alice), 120);
        assert_eq!(client.balance(&bob),   130);

        // Total supply is unchanged by a transfer (no tokens created/burned)
        assert_eq!(client.total_supply(), 250);
    }
}
