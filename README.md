# Coin 🪙

> A campus currency token on Stellar that enables tipping, micro-payments, and creator monetization among students.

---

## Problem

Campus communities lack a simple way to reward peer tutoring, freelance gigs, or creative contributions.

## Solution

A campus currency token on Stellar enables tipping, micro-payments, and creator monetization among students. Coin is a custom token managed by a Soroban smart contract — the campus admin (treasury or institution) mints tokens, and students transfer them freely to tip tutors, pay for gigs, or support creators, all on-chain with no bank account required.

---

## Stellar Features Used

| Feature | Purpose |
|---|---|
| **Custom tokens** | Coin is a campus-issued token, not XLM |
| **Soroban smart contract** | Mint, transfer, and balance logic on-chain |
| **XLM/USDC transfer** | Gas fees paid in XLM; future USDC redemption path |
| **Trustline** | Students must trust the Coin token before receiving it |

---

## Target Users

University students in Southeast Asia (Philippines, Indonesia, Vietnam) who tutor peers, take on campus freelance gigs, or create content for their college community — and want instant, low-friction payments without a bank account.

---

## Core Feature (MVP)

`transfer(from, to, amount)` — a student sends Coin tokens directly to a peer tutor, freelancer, or creator. The contract verifies the sender has enough balance, debits them, and credits the recipient. One transaction proves the full campus micro-payment loop end-to-end.

---

## Constraints

| Dimension | Selection |
|---|---|
| Region | SEA (Southeast Asia) |
| User Type | Students, Freelancers, Creators |
| Complexity | Soroban required, No-code friendly UI |

## Theme

**Education** → Campus Currency + Tip Economy + Creator Monetization

---

## Suggested MVP Timeline

| Week | Milestone |
|---|---|
| 1 | Contract written, 3 tests passing locally |
| 2 | Deploy to Stellar testnet; admin mints tokens to pilot cohort |
| 3 | Simple web front-end: wallet view + transfer form |
| 4 | Pilot with one university org; gather feedback |

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable, ≥ 1.74)
- WASM target: `rustup target add wasm32-unknown-unknown`
- Stellar CLI ≥ v21: `cargo install --locked stellar-cli --features opt`
- [Freighter Wallet](https://freighter.app) set to **Testnet**

---

## Build

```bash
stellar contract build
# Output: target/wasm32-unknown-unknown/release/coin.wasm
```

---

## Test

```bash
cargo test
```

Expected passing tests:
- `test_happy_path_tip_peer_tutor`
- `test_transfer_fails_when_insufficient_balance`
- `test_state_correct_after_mint_and_transfer`

---

## Deploy to Testnet

```bash
# 1. Generate identity (first time only)
stellar keys generate --global my-key --network testnet
stellar keys address my-key

# 2. Fund via Friendbot
stellar keys fund my-key --network testnet

# 3. Build
stellar contract build

# 4. Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/coin.wasm \
  --source my-key \
  --network testnet
# → Copy the Contract ID (starts with C...)
```

Verify on Stellar Expert:
```
https://stellar.expert/explorer/testnet/contract/<YOUR_CONTRACT_ID>
```

---

## Sample CLI Invocations

```bash
# Initialise with admin address
stellar contract invoke \
  --id <CONTRACT_ID> --source my-key --network testnet \
  -- init \
  --admin GADMIN000000000000000000000000000000000000000000000000

# Mint 100 Coin to a student
stellar contract invoke \
  --id <CONTRACT_ID> --source my-key --network testnet \
  -- mint \
  --to GSTUDENT0000000000000000000000000000000000000000000000 \
  --amount 100

# Student tips a tutor 30 Coin
stellar contract invoke \
  --id <CONTRACT_ID> --source my-key --network testnet \
  -- transfer \
  --from GSTUDENT0000000000000000000000000000000000000000000000 \
  --to GTUTOR00000000000000000000000000000000000000000000000000 \
  --amount 30

# Check a balance
stellar contract invoke \
  --id <CONTRACT_ID> --source my-key --network testnet \
  -- balance \
  --account GTUTOR00000000000000000000000000000000000000000000000000
```

---

## Project Structure

```
coin/
├── src/
│   ├── lib.rs      # Soroban smart contract
│   └── test.rs     # Unit tests (3 tests)
├── Cargo.toml
└── README.md
```

---

## Resources

| Resource | Link |
|---|---|
| Stellar Bootcamp 2026 | [github.com/armlynobinguar/Stellar-Bootcamp-2026](https://github.com/armlynobinguar/Stellar-Bootcamp-2026) |
| Stellar Docs | [developers.stellar.org](https://developers.stellar.org) |
| Soroban SDK | [docs.rs/soroban-sdk](https://docs.rs/soroban-sdk) |
| Stellar CLI | [developers.stellar.org/docs/tools/stellar-cli](https://developers.stellar.org/docs/tools/stellar-cli) |
| Stellar Expert (Testnet) | [stellar.expert/explorer/testnet](https://stellar.expert/explorer/testnet) |

---

## License

MIT License

Copyright (c) 2026 Coin Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
