/**
 * WalletDataService  –  Phase 2: Backend-Logik & Data Fetching
 *
 * Responsibilities:
 *   - Fetch wallet data from the two JSON API endpoints
 *   - Apply business logic (formatting, sign detection, state detection)
 *   - Return typed, display-ready objects to the UI layer
 *
 * Endpoints consumed:
 *   GET /api/wallet/balance       → WalletBalanceResponse
 *   GET /api/wallet/transactions  → WalletTransactionsResponse
 */
const WalletDataService = (function () {
    "use strict";

    // ─── Constants ────────────────────────────────────────────────
    const BALANCE_ENDPOINT = "/api/wallet/balance";
    const TRANSACTIONS_ENDPOINT = "/api/wallet/transactions";

    // ─── Internal Helpers ─────────────────────────────────────────

    /**
     * Generic fetch wrapper.
     * @param {string} url
     * @returns {Promise<any>}
     */
    async function apiFetch(url) {
        const res = await fetch(url, {
            method: "GET",
            credentials: "same-origin",
            headers: { Accept: "application/json" },
        });
        if (!res.ok) {
            throw new Error(`API ${url} returned ${res.status} ${res.statusText}`);
        }
        return res.json();
    }

    /**
     * Format a raw cents value → "USD X,XXX.XX"
     * (mirrors the Rust format_usd helper for consistency)
     * @param {number} cents
     * @returns {string}
     */
    function formatUsd(cents) {
        if (typeof cents !== "number" || isNaN(cents)) return "USD 0.00";
        const negative = cents < 0;
        const abs = Math.abs(cents);
        const dollars = Math.floor(abs / 100);
        const remainder = abs % 100;
        const dollarsStr = dollars
            .toString()
            .replace(/\B(?=(\d{3})+(?!\d))/g, ",");
        const formatted = `USD ${dollarsStr}.${String(remainder).padStart(2, "0")}`;
        return negative ? `USD -${dollarsStr}.${String(remainder).padStart(2, "0")}` : formatted;
    }

    /**
     * Map a raw transaction type string to a display label.
     * @param {string} type
     * @returns {string}
     */
    function txTypeLabel(type) {
        const map = {
            deposit: "Deposit",
            withdrawal: "Withdraw",
            purchase: "Investment",
            sale: "Sale",
            dividend: "Rent Paid",
            reward: "Reward",
            refund: "Refund",
            fee: "Fee",
        };
        return map[type] || type.charAt(0).toUpperCase() + type.slice(1);
    }

    /**
     * Map a raw transaction type → icon key (matches wallet.html SVG switch).
     * @param {string} type
     * @returns {string}
     */
    function txIconKey(type) {
        const positive = ["deposit", "dividend", "reward", "refund"];
        if (positive.includes(type)) return type === "deposit" ? "deposit" : "dividend";
        if (type === "withdrawal") return "withdrawal";
        return "purchase";
    }

    /**
     * Map a raw status string to CSS badge class and display label.
     * @param {string} status
     * @returns {{ label: string, cssClass: string }}
     */
    function mapStatus(status) {
        switch (status) {
            case "completed":
                return { label: "Completed", cssClass: "status-completed" };
            case "pending":
            case "processing":
                return { label: "In process", cssClass: "status-in-process" };
            case "failed":
            case "cancelled":
                return { label: "Declined", cssClass: "status-declined" };
            default:
                return { label: "Pending", cssClass: "status-in-process" };
        }
    }

    /**
     * Format an ISO date string like "2026-03-09T06:16:17Z" → "09 Mar 2026"
     * @param {string} iso
     * @returns {string}
     */
    function formatDate(iso) {
        try {
            const d = new Date(iso);
            return d.toLocaleDateString("en-GB", {
                day: "2-digit",
                month: "short",
                year: "numeric",
            });
        } catch {
            return iso;
        }
    }

    /**
     * Map a wallet_type string → display label.
     * @param {string} wt
     * @returns {string}
     */
    function walletLabel(wt) {
        return wt === "rewards" ? "Rewards balance" : "Cash balance";
    }

    // ─── Public API ───────────────────────────────────────────────

    /**
     * Fetch and process wallet balances.
     *
     * @returns {Promise<WalletBalanceSummary>}
     *   {
     *     cashDisplay:    string,   // "USD 2,732.00"
     *     rewardsDisplay: string,
     *     assetDisplay:   string,
     *     cashCents:      number,
     *     rewardsCents:   number,
     *     assetCents:     number,
     *     isEmpty:        boolean,  // true when all three are zero
     *   }
     */
    async function getBalances() {
        const raw = await apiFetch(BALANCE_ENDPOINT);

        const cashCents = raw.cash_cents ?? 0;
        const rewardsCents = raw.rewards_cents ?? 0;
        const assetCents = raw.asset_cents ?? 0;

        return {
            cashDisplay: raw.cash_display || formatUsd(cashCents),
            rewardsDisplay: raw.rewards_display || formatUsd(rewardsCents),
            assetDisplay: raw.asset_display || formatUsd(assetCents),
            cashCents,
            rewardsCents,
            assetCents,
            // Page is "empty" when user has no activity at all
            isEmpty: cashCents === 0 && rewardsCents === 0 && assetCents === 0,
        };
    }

    /**
     * Fetch and process wallet transactions.
     *
     * @returns {Promise<WalletTransactionsSummary>}
     *   {
     *     transactions: WalletTxRow[],
     *     count:        number,
     *     hasData:      boolean,
     *   }
     *
     * WalletTxRow shape:
     *   {
     *     id:           string,
     *     typeKey:      string,   // raw type from API
     *     typeLabel:    string,   // "Deposit", "Withdraw" …
     *     iconKey:      string,   // "deposit" | "withdrawal" | "dividend" | "purchase"
     *     statusLabel:  string,
     *     statusCss:    string,
     *     dateDisplay:  string,   // "09 Mar 2026"
     *     dateIso:      string,
     *     walletLabel:  string,   // "Cash balance" | "Rewards balance"
     *     amountCents:  number,
     *     amountDisplay:string,   // "USD 175.00"
     *     amountPrefix: string,   // "+" or "-"
     *     amountCss:    string,   // "amount-positive" | "amount-negative"
     *   }
     */
    async function getTransactions(page = 1, pageSize = 10) {
        const url = `${TRANSACTIONS_ENDPOINT}?page=${page}&page_size=${pageSize}`;
        const raw = await apiFetch(url);
        const rows = raw.transactions ?? [];

        const transactions = rows.map((tx) => {
            const positive = tx.amount_cents >= 0;
            const absCents = Math.abs(tx.amount_cents);
            const { label: statusLabel, cssClass: statusCss } = mapStatus(tx.status);

            return {
                id: tx.id,
                typeKey: tx.type,
                typeLabel: txTypeLabel(tx.type),
                iconKey: txIconKey(tx.type),
                statusLabel,
                statusCss,
                dateDisplay: formatDate(tx.created_at),
                dateIso: tx.created_at,
                walletLabel: walletLabel(tx.wallet_type),
                amountCents: tx.amount_cents,
                amountDisplay: formatUsd(absCents),
                amountPrefix: positive ? "+" : "-",
                amountCss: positive ? "amount-positive" : "amount-negative",
            };
        });

        return {
            transactions,
            count: transactions.length,
            total: raw.total ?? transactions.length,
            page: raw.page ?? page,
            pageSize: raw.page_size ?? pageSize,
            hasData: raw.total > 0,
        };
    }

    /**
     * Fetch everything the wallet page needs in parallel.
     *
     * @returns {Promise<{ balances: WalletBalanceSummary, txSummary: WalletTransactionsSummary }>}
     */
    async function getWalletPageData(page = 1, pageSize = 10) {
        const [balances, txSummary] = await Promise.all([
            getBalances(),
            getTransactions(page, pageSize),
        ]);
        return { balances, txSummary };
    }

    return {
        getBalances,
        getTransactions,
        getWalletPageData,
        // Expose helpers so UI tests can use them
        formatUsd,
        txTypeLabel,
        mapStatus,
        formatDate,
    };
})();
