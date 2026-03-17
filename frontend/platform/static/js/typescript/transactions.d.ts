// PHASE 1: Data Model & Typisierung für die Transactions-Seite

export interface WalletTransactionApiEntry {
    id: string;
    tx_type: 'deposit' | 'withdrawal' | 'dividend' | 'rent_paid' | string;
    status: 'completed' | 'pending' | 'failed' | string;
    amount_cents: number;
    amount_usd: number;
    wallet_type: 'cash' | 'rewards' | 'assets' | string;
    created_at: string;
}

export interface WalletTransactionsResponse {
    transactions: WalletTransactionApiEntry[];
    count: number;
}
