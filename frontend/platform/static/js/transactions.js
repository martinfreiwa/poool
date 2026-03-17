// PHASE 2 & 3: Back-End Logik & UI State-Binding für Transactions API

const TRANSACTIONS_API_URL = '/api/wallet/transactions';

async function apiCall(url, options = {}) {
    const defaultOptions = {
        headers: {
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        }
    };
    try {
        const response = await fetch(url, { ...defaultOptions, ...options });

        if (response.status === 401) {
            window.location.href = '/auth/login';
            return null;
        }

        if (!response.ok) {
            console.error(`API Error on ${url}: ${response.status} ${response.statusText}`);
            return null;
        }

        return await response.json();
    } catch (error) {
        console.error('Network Error:', error);
        return null; // Return null so the UI can show empty/error state
    }
}

function formatDate(isoString) {
    if (!isoString) return 'N/A';
    const date = new Date(isoString);
    if (isNaN(date.getTime())) return isoString;
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const d = date.getDate().toString().padStart(2, '0');
    const m = months[date.getMonth()];
    const y = date.getFullYear();
    return `${d} ${m} ${y}`;
}

function formatUSD(amountCents) {
    const absValueCents = Math.abs(amountCents);
    const dollars = Math.floor(absValueCents / 100);
    const cents = absValueCents % 100;
    const dollarsStr = dollars.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    return `${dollarsStr}.${cents.toString().padStart(2, '0')}`;
}

function getTxTypeIcon(txType) {
    switch (txType.toLowerCase()) {
        case 'deposit':
            return `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M10 15V5M10 5L5 10M10 5L15 10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round" /></svg>`;
        case 'withdrawal':
            return `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M10 5V15M10 15L5 10M10 15L15 10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round" /></svg>`;
        case 'dividend':
        case 'rent_paid':
        case 'rent paid':
            return `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M2.5 10C2.5 10 5 7.5 10 7.5C15 7.5 17.5 10 17.5 10M2.5 10V15C2.5 16.3807 3.61929 17.5 5 17.5H15C16.3807 17.5 17.5 16.3807 17.5 15V10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round" /><circle cx="10" cy="5" r="2.5" stroke="#717680" stroke-width="1.66667" /></svg>`;
        default:
            return `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M2.5 7.5L10 2.5L17.5 7.5V16.25C17.5 16.5815 17.3683 16.8995 17.1339 17.1339C16.8995 17.3683 16.5815 17.5 16.25 17.5H3.75C3.41848 17.5 3.10054 17.3683 2.86612 17.1339C2.6317 16.8995 2.5 16.5815 2.5 16.25V7.5Z" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round" /><path d="M7.5 17.5V10H12.5V17.5" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round" /></svg>`;
    }
}

function getStatusBadge(status) {
    let css = 'status-completed';
    let label = 'Completed';
    switch (status.toLowerCase()) {
        case 'pending': css = 'status-in-process'; label = 'Pending'; break;
        case 'failed':
        case 'declined': css = 'status-declined'; label = 'Declined'; break;
        default: css = 'status-completed'; label = 'Completed'; break;
    }
    return `
        <div class="wallet-transaction-status-badge ${css}">
            <div class="wallet-transaction-status-dot"></div>
            <span class="wallet-transaction-status-text">${label}</span>
        </div>`;
}

function processTransactions(transactionsData) {
    const listBody = document.getElementById('wallet-transactions-list-body');
    const listContainer = document.getElementById('wallet-transactions-list-container');
    const emptyState = document.getElementById('wallet-transactions-empty');
    if (!listBody) return;

    // Clear list
    listBody.innerHTML = '';

    if (!transactionsData || !transactionsData.transactions || transactionsData.transactions.length === 0) {
        if (listContainer) listContainer.classList.add('hidden');
        if (emptyState) emptyState.classList.remove('hidden');
        return;
    }

    if (listContainer) listContainer.classList.remove('hidden');
    if (emptyState) emptyState.classList.add('hidden');

    transactionsData.transactions.forEach((tx, idx) => {
        const row = document.createElement('div');
        row.className = 'table__row';

        let txTypeLabel = tx.type.charAt(0).toUpperCase() + tx.type.slice(1);
        if (tx.type.toLowerCase() === 'rent_paid') txTypeLabel = 'Rent Paid';

        const amountPrefix = tx.amount_cents >= 0 ? '+' : '-';
        const amountCss = tx.amount_cents >= 0 ? 'amount-positive' : 'amount-negative';

        row.innerHTML = `
            <div class="table__cell table__cell--type">
                <div class="wallet-transaction-type-icon">
                    <div class="featured-icon">
                        ${getTxTypeIcon(tx.type)}
                    </div>
                </div>
                <span class="wallet-transaction-type-text">${txTypeLabel}</span>
            </div>
            <div class="table__cell table__cell--status">
                ${getStatusBadge(tx.status)}
            </div>
            <div class="table__cell table__cell--date">
                <span class="table__cell-text-value">${formatDate(tx.created_at)}</span>
            </div>
            <div class="table__cell table__cell--wallet">
                <span class="table__cell-text-value">${tx.wallet_type === 'cash' ? 'Cash balance' : 'Rewards balance'}</span>
            </div>
            <div class="table__cell table__cell--amount">
                <span class="${amountCss}">${amountPrefix} USD ${formatUSD(tx.amount_cents)}</span>
            </div>
            <div class="table__cell table__cell--actions">
                <button class="wallet-transaction-action-btn">
                    View details
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 12L10 8L6 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </button>
            </div>
        `;
        listBody.appendChild(row);
    });
}

async function loadTransactions() {
    console.log("Loading transactions...");
    const skeleton = document.getElementById('transactions-loading-skeleton');
    const content = document.getElementById('transactions-content');
    const fetchError = document.getElementById('transactions-fetch-error');

    if (skeleton) skeleton.classList.remove('hidden');
    if (content) content.classList.add('hidden');
    if (fetchError) fetchError.classList.add('hidden');

    const data = await apiCall(TRANSACTIONS_API_URL);

    if (skeleton) skeleton.classList.add('hidden');

    if (data) {
        if (content) content.classList.remove('hidden');
        processTransactions(data);
    } else {
        // Show fetch error
        if (fetchError) fetchError.classList.remove('hidden');
    }
}

// Initialise when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    loadTransactions();
});

// Expose reload function for "Retry" buttons
window.loadTransactions = loadTransactions;
