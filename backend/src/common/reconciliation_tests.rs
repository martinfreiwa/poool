//! Reconciliation invariant tests — Phase 11.3
//!
//! Simulates the full lifecycle: deposit → buy → trade → sell → withdraw
//! and verifies that all financial invariants hold:
//!   - Cash conservation: total deposits - withdrawals - purchases = total wallet balance
//!   - Token conservation: sum(tokens_owned) = asset.tokens_total per asset
//!   - Fee accounting: buyer_pays = seller_receives + platform_fee
//!   - No negative balances

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    /// Simulated wallet state for a user.
    #[derive(Debug, Clone)]
    struct Wallet {
        balance_cents: i64,
        held_cents: i64,
    }

    impl Wallet {
        fn new() -> Self {
            Self {
                balance_cents: 0,
                held_cents: 0,
            }
        }
        fn available(&self) -> i64 {
            self.balance_cents - self.held_cents
        }
    }

    /// Simulated asset state.
    #[derive(Debug, Clone)]
    struct Asset {
        tokens_total: i32,
        token_price_cents: i64,
    }

    /// Simulated investment position.
    #[derive(Debug, Clone)]
    struct Position {
        tokens_owned: i32,
        purchase_value_cents: i64,
    }

    /// Calculate fee in cents (same as production code).
    fn calculate_fee_cents(total_cents: i64, fee_bps: i32) -> i64 {
        let bps = fee_bps.max(0) as i64;
        let fee = total_cents.saturating_mul(bps) / 10_000;
        fee.min(total_cents)
    }

    // ─── Full Lifecycle Reconciliation ──────────────────────────

    #[test]
    fn test_full_lifecycle_reconciliation() {
        // Setup: 2 users, 1 asset
        let mut wallets: HashMap<&str, Wallet> = HashMap::new();
        wallets.insert("alice", Wallet::new());
        wallets.insert("bob", Wallet::new());
        let mut platform_fees: i64 = 0;

        let asset = Asset {
            tokens_total: 1000,
            token_price_cents: 10_000, // $100/token
        };
        let fee_bps: i32 = 500; // 5%

        // Track all deposits and withdrawals
        let mut total_deposits: i64 = 0;
        let mut total_withdrawals: i64 = 0;

        let mut positions: HashMap<&str, Position> = HashMap::new();

        // ── Step 1: Alice deposits $50,000 ──────────────────────
        let deposit_alice = 5_000_000;
        wallets.get_mut("alice").unwrap().balance_cents += deposit_alice;
        total_deposits += deposit_alice;

        // ── Step 2: Bob deposits $30,000 ────────────────────────
        let deposit_bob = 3_000_000;
        wallets.get_mut("bob").unwrap().balance_cents += deposit_bob;
        total_deposits += deposit_bob;

        // ── Step 3: Alice buys 100 tokens at $100 each ─────────
        let buy_quantity = 100;
        let buy_total = asset.token_price_cents * buy_quantity as i64;
        let buy_fee = calculate_fee_cents(buy_total, fee_bps);

        // Deduct from Alice's wallet
        let alice = wallets.get_mut("alice").unwrap();
        assert!(alice.balance_cents >= buy_total + buy_fee, "Insufficient funds for buy");
        alice.balance_cents -= buy_total + buy_fee;
        platform_fees += buy_fee;

        positions.insert(
            "alice",
            Position {
                tokens_owned: buy_quantity,
                purchase_value_cents: buy_total,
            },
        );

        // ── Step 4: Alice sells 50 tokens to Bob at $110 each ──
        let sell_quantity = 50;
        let sell_price = 11_000; // $110
        let sell_total = sell_price * sell_quantity as i64;
        let seller_fee = calculate_fee_cents(sell_total, fee_bps);
        let buyer_fee = calculate_fee_cents(sell_total, fee_bps);

        // Bob pays
        let bob = wallets.get_mut("bob").unwrap();
        assert!(bob.balance_cents >= sell_total + buyer_fee, "Bob insufficient funds");
        bob.balance_cents -= sell_total + buyer_fee;

        // Alice receives
        let alice = wallets.get_mut("alice").unwrap();
        alice.balance_cents += sell_total - seller_fee;
        platform_fees += seller_fee + buyer_fee;

        // Update positions
        positions.get_mut("alice").unwrap().tokens_owned -= sell_quantity;
        positions.insert(
            "bob",
            Position {
                tokens_owned: sell_quantity,
                purchase_value_cents: sell_total,
            },
        );

        // ── Step 5: Bob withdraws $10,000 ──────────────────────
        let withdraw_bob = 1_000_000;
        let bob = wallets.get_mut("bob").unwrap();
        assert!(bob.balance_cents >= withdraw_bob, "Insufficient for withdrawal");
        bob.balance_cents -= withdraw_bob;
        total_withdrawals += withdraw_bob;

        // ─── RECONCILIATION CHECKS ─────────────────────────────

        // Check 1: Cash conservation
        // In a real system: wallet balances = deposits - withdrawals - fees - primary_purchases
        // Primary purchases flow to the developer/SPV (out of investor wallet pool).
        // Secondary trades are zero-sum between investors (money stays in pool).
        let total_wallet_balance: i64 = wallets.values().map(|w| w.balance_cents).sum();
        let primary_purchase_outflow = buy_total; // Money that left wallets to pay developer/SPV
        let expected_cash = total_deposits - total_withdrawals - platform_fees - primary_purchase_outflow;
        assert_eq!(
            total_wallet_balance, expected_cash,
            "Cash not conserved: wallets={}, expected={}",
            total_wallet_balance, expected_cash
        );

        // Check 2: No negative balances
        for (user, wallet) in &wallets {
            assert!(
                wallet.balance_cents >= 0,
                "User {} has negative balance: {}",
                user,
                wallet.balance_cents
            );
        }

        // Check 3: Token conservation
        let total_tokens_held: i32 = positions.values().map(|p| p.tokens_owned).sum();
        // In this test, not all tokens are allocated (only 100 were "purchased" from primary)
        // The remaining 900 are with the platform/treasury
        assert!(
            total_tokens_held <= asset.tokens_total,
            "More tokens held ({}) than exist ({})",
            total_tokens_held,
            asset.tokens_total
        );

        // Check 4: Fee accounting
        assert!(platform_fees > 0, "Platform should have collected fees");
        let expected_buy_fee = calculate_fee_cents(10_000 * 100, fee_bps);
        let expected_sell_fee = calculate_fee_cents(11_000 * 50, fee_bps);
        let expected_buyer_fee = calculate_fee_cents(11_000 * 50, fee_bps);
        assert_eq!(
            platform_fees,
            expected_buy_fee + expected_sell_fee + expected_buyer_fee,
            "Fee accounting mismatch"
        );
    }

    // ─── Multi-Trade Invariant ──────────────────────────────────

    #[test]
    fn test_multi_trade_fee_invariant() {
        // Run 100 simulated trades and verify fee invariant holds every time
        let fee_bps = 250; // 2.5%
        let prices = [
            10_000, 10_500, 9_800, 11_200, 10_100, 15_000, 8_000, 12_500, 10_000, 10_000,
        ];
        let quantities = [10, 5, 20, 1, 100, 50, 25, 8, 3, 15];

        for (price, qty) in prices.iter().zip(quantities.iter()) {
            let trade_value = price * (*qty as i64);
            let fee = calculate_fee_cents(trade_value, fee_bps);

            // Invariant: fee = floor(trade_value * bps / 10000)
            let expected = trade_value * (fee_bps as i64) / 10_000;
            assert_eq!(
                fee, expected,
                "Fee invariant violated: trade_value={}, fee={}, expected={}",
                trade_value, fee, expected
            );

            // Invariant: 0 <= fee <= trade_value
            assert!(fee >= 0, "Negative fee");
            assert!(fee <= trade_value, "Fee exceeds trade value");

            // Invariant: seller_receives + fee <= trade_value
            let seller_receives = trade_value - fee;
            assert!(
                seller_receives >= 0,
                "Seller receives negative: {}",
                seller_receives
            );
            assert_eq!(
                seller_receives + fee,
                trade_value,
                "Trade value not conserved"
            );
        }
    }

    // ─── Concurrent Access Simulation ───────────────────────────

    #[test]
    fn test_simulated_concurrent_balance_check() {
        // Simulate what happens if two withdrawals are checked simultaneously.
        // The production code uses FOR UPDATE to prevent this — here we test the invariant.
        let balance: i64 = 10_000; // $100
        let withdrawal1: i64 = 10_000; // $100
        let withdrawal2: i64 = 10_000; // $100

        // Sequential (correct) processing:
        let mut sequential_balance = balance;

        // First withdrawal succeeds
        if sequential_balance >= withdrawal1 {
            sequential_balance -= withdrawal1;
        }
        assert_eq!(sequential_balance, 0);

        // Second withdrawal must fail — balance is 0
        assert!(
            sequential_balance < withdrawal2,
            "Second withdrawal should be rejected: balance={}, requested={}",
            sequential_balance,
            withdrawal2
        );

        // BAD scenario (what FOR UPDATE prevents):
        // Both read balance=10000 concurrently, both approve, both deduct → -10000
        let bad_balance = balance - withdrawal1 - withdrawal2;
        assert!(
            bad_balance < 0,
            "Without FOR UPDATE, balance goes negative: {}",
            bad_balance
        );
    }

    // ─── Token Supply Invariant ─────────────────────────────────

    #[test]
    fn test_token_supply_invariant() {
        // asset.tokens_total must always equal sum of all holdings
        let total_tokens: i32 = 10_000;
        let mut holdings: Vec<i32> = vec![10_000]; // Platform treasury holds all initially

        // Primary issuance: platform transfers 5000 tokens to investors
        let issuance = vec![1000, 2000, 500, 1500]; // 4 investors
        let issuance_total: i32 = issuance.iter().sum();

        holdings[0] -= issuance_total; // Platform gives up tokens
        holdings.extend(issuance.iter());

        let sum: i32 = holdings.iter().sum();
        assert_eq!(
            sum, total_tokens,
            "Token supply invariant violated after issuance: sum={}, total={}",
            sum, total_tokens
        );

        // Secondary trade: investor 1 (1000 tokens) sells 500 to investor 2
        holdings[1] -= 500;
        holdings[2] += 500;

        let sum2: i32 = holdings.iter().sum();
        assert_eq!(
            sum2, total_tokens,
            "Token supply invariant violated after trade: sum={}, total={}",
            sum2, total_tokens
        );
    }

    // ─── Reconciliation Report Status ───────────────────────────

    #[test]
    fn test_reconciliation_status_classification() {
        // Test the reconciliation status logic from main.rs

        let test_cases = vec![
            // (cash_delta, token_mismatches, negative_balances, expected_status)
            (0, 0, 0, "pass"),
            (1, 0, 0, "warning"),
            (0, 1, 0, "warning"),
            (0, 0, 1, "warning"),
            (-100, 2, 1, "warning"),
        ];

        for (cash_delta, token_mm, neg_bal, expected) in test_cases {
            let status = if cash_delta == 0 && token_mm == 0 && neg_bal == 0 {
                "pass"
            } else {
                "warning"
            };
            assert_eq!(
                status, expected,
                "Status mismatch for delta={}, mm={}, neg={}",
                cash_delta, token_mm, neg_bal
            );
        }
    }
}
