/// <reference path="./typescript/portfolio.d.ts" />

/**
 * Helper to format cents into USD currency string
 */
function formatCurrency(cents: number): string {
    return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD',
        minimumFractionDigits: 0,
        maximumFractionDigits: 2
    }).format(cents / 100);
}

/**
 * Format string as US currency without the generic sign since HTML already has $
 */
function formatAmount(cents: number): string {
    return new Intl.NumberFormat('en-US', {
        minimumFractionDigits: 0,
        maximumFractionDigits: 2
    }).format(cents / 100);
}

/**
 * Helper to format basis points into percentage string
 */
function formatPercentage(bps: number, suffix: string = '%'): string {
    const percentage = bps / 100;
    return `${percentage.toFixed(1)}${suffix}`;
}

async function fetchPortfolioData() {
    try {
        const response = await fetch('/api/portfolio');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        // Data casting & structure validation based on TS interface Phase 1
        const data = await response.json() as PortfolioResponse;

        // --- DATA BINDING (Phase 3 Prep) ---

        // 1. Update Portfolio Value Header
        const totalValueEl = document.getElementById('portfolio-total-value');
        if (totalValueEl) totalValueEl.textContent = formatCurrency(data.total_value_cents);

        const appreciationPctEl = document.getElementById('portfolio-appreciation-percentage');
        if (appreciationPctEl) {
            // Simplified appreciation calc based on total return vs purchase
            let overallAppreciation = 0;
            if (data.total_purchase_cents > 0) {
                overallAppreciation = (data.total_appreciation_cents / data.total_purchase_cents) * 100;
            }
            appreciationPctEl.textContent = `${overallAppreciation > 0 ? '+' : ''}${overallAppreciation.toFixed(1)}%`;
        }

        // 2. Key Financials
        const monthlyIncomeEl = document.getElementById('portfolio-monthly-income');
        if (monthlyIncomeEl) monthlyIncomeEl.textContent = formatAmount(data.monthly_income_cents);

        const totalRentalEl = document.getElementById('portfolio-total-rental');
        if (totalRentalEl) totalRentalEl.textContent = formatAmount(data.total_rental_cents);

        const totalAppreciationEl = document.getElementById('portfolio-total-appreciation');
        if (totalAppreciationEl) totalAppreciationEl.textContent = formatAmount(data.total_appreciation_cents);

        // 3. Quick Insights
        const numPropertiesEl = document.getElementById('insights-value-number-of-properties');
        if (numPropertiesEl) numPropertiesEl.textContent = data.investment_count.toString();

        const occupancyEl = document.getElementById('insights-value-occupancy-rate');
        if (occupancyEl) occupancyEl.textContent = formatPercentage(data.occupancy_rate_bps);

        const yieldEl = document.getElementById('insights-value-annual-rental-yield');
        if (yieldEl) yieldEl.textContent = formatPercentage(data.annual_yield_bps);

        // 4. Annual Investment Limit Limits
        if (data.annual_limit) {
            const limit = data.annual_limit;
            const limitAnnualEl = document.getElementById('investment-limit-annual-value');
            if (limitAnnualEl) limitAnnualEl.textContent = `USD ${formatAmount(limit.annual_limit_cents)}`;

            const limitInvestedEl = document.getElementById('investment-limit-invested-value');
            if (limitInvestedEl) limitInvestedEl.textContent = `USD ${formatAmount(limit.invested_12m_cents)}`;

            const limitAvailableEl = document.getElementById('investment-limit-available-value');
            if (limitAvailableEl) limitAvailableEl.textContent = `USD ${formatAmount(limit.available_cents)}`;

            // Progress Bar
            let percentUsed = 0;
            if (limit.annual_limit_cents > 0) {
                percentUsed = Math.min(100, (limit.invested_12m_cents / limit.annual_limit_cents) * 100);
            }

            const progressTextEl = document.getElementById('progress-percentage-text');
            if (progressTextEl) progressTextEl.textContent = `${Math.round(percentUsed)}% of limit used`;

            const progressBarEl = document.getElementById('progress-bar-fill-desktop');
            if (progressBarEl) progressBarEl.style.width = `${percentUsed}%`;
        }

        // 5. Render Asset Table
        renderAssetsTable(data.investments);

        // Remove empty state/loaders and show content
        document.getElementById('portfolio-loading-skeleton')?.classList.add('hidden');
        document.getElementById('portfolio-value-section')?.classList.remove('hidden');
        document.getElementById('key-financials-section')?.classList.remove('hidden');
        document.getElementById('insights-limit-section')?.classList.remove('hidden');
        document.getElementById('assets-section')?.classList.remove('hidden');

    } catch (error) {
        console.error("Failed to load portfolio dashboard:", error);
        // Show error / empty state here
        document.getElementById('portfolio-loading-skeleton')?.classList.add('hidden');
        const emptyState = document.getElementById('portfolio-empty-state');
        if (emptyState) emptyState.classList.remove('hidden');
    }
}

function getStatusBadge(status: string, payoutExpectedAt: string | null): string {
    const statusLower = status.toLowerCase();

    if (statusLower === 'active') {
        return `<div class="portfolio-assets-status status-active">
                  <span class="portfolio-assets-status-icon"></span>
                  <span class="portfolio-assets-status-text">Active</span>
                </div>`;
    }
    else if (statusLower.includes('funded')) {
        return `<div class="portfolio-assets-status status-funded">
                  <span class="portfolio-assets-status-icon"></span>
                  <span class="portfolio-assets-status-text">Property funded</span>
                </div>`;
    }
    else if (statusLower.includes('payout')) {
        const month = payoutExpectedAt ? new Date(payoutExpectedAt).toLocaleString('en-US', { month: 'short' }) : 'Soon';
        return `<div class="portfolio-assets-status status-payout">
                  <span class="portfolio-assets-status-icon"></span>
                  <span class="portfolio-assets-status-text">Payout expected: ${month}</span>
                </div>`;
    }
    else if (statusLower.includes('rented')) {
        return `<div class="portfolio-assets-status status-rented">
                  <span class="portfolio-assets-status-icon"></span>
                  <span class="portfolio-assets-status-text">Rented</span>
                </div>`;
    }
    else if (statusLower.includes('exited')) {
        return `<div class="portfolio-assets-status status-funded">
                  <span class="portfolio-assets-status-icon"></span>
                  <span class="portfolio-assets-status-text">Exited</span>
                </div>`;
    }

    // Default / "In Process" (e.g. funding_in_progress)
    return `<div class="portfolio-assets-status status-process">
              <span class="portfolio-assets-status-icon"></span>
              <span class="portfolio-assets-status-text">In process</span>
            </div>`;
}

function renderAssetsTable(investments: InvestmentItem[]) {
    const tableBody = document.getElementById('portfolio-assets-body');
    if (!tableBody) return;

    if (investments.length === 0) {
        tableBody.innerHTML = `
            <div class="portfolio-assets-empty-state w-full py-12 flex flex-col items-center justify-center text-gray-500">
                <img src="/images/home-smile.svg" alt="No assets" class="mb-4 opacity-50" width="48" height="48">
                <p>No investments found.</p>
                <a href="/marketplace" class="mt-4 text-blue-600 hover:underline">Go to Marketplace</a>
            </div>
        `;
        return;
    }

    let rowsHtml = '';

    investments.forEach(inv => {
        const appreciationValue = formatPercentage(inv.appreciation_pct_bps);
        // Decide red/green based on appreciation 
        const isPositive = inv.appreciation_pct_bps >= 0;
        const changeClass = isPositive ? 'positive' : 'negative'; // TBD Map to exact CSS class in phase 3
        const changePrefix = isPositive ? '+' : '';

        rowsHtml += `
        <div class="portfolio-assets-row">
            <!-- Property Column -->
            <div class="portfolio-assets-cell property-col">
            <div class="portfolio-assets-property">
                <div class="portfolio-assets-property-image">
                <!-- Using generic image for now, normally use inv.asset_slug or specific field -->
                <img loading="lazy" src="/images/villa1.webp" alt="${inv.asset_title}"
                    width="48" height="48" onerror="this.src='/images/home-05.svg'" />
                </div>
                <div class="portfolio-assets-property-info">
                <a href="/commodity/${inv.asset_slug}" class="portfolio-assets-property-name hover:underline">
                    ${inv.asset_title}
                </a>
                <div class="text-xs text-gray-500 mt-1">${inv.tokens_owned} Tokens</div>
                </div>
            </div>
            </div>
            <!-- Investment Value Column -->
            <div class="portfolio-assets-cell investment-col">
            <div class="portfolio-assets-investment">
                <div class="portfolio-assets-value">${formatCurrency(inv.current_value_cents)}</div>
                <div class="portfolio-assets-change ${changeClass}">
                <span class="portfolio-assets-change-icon"></span>
                <span>${changePrefix}${appreciationValue}</span>
                </div>
            </div>
            </div>
            <!-- Rental Income Column -->
            <div class="portfolio-assets-cell rental-col">
            <div class="portfolio-assets-rental">
                <div class="portfolio-assets-value">${formatCurrency(inv.total_rental_cents)}</div>
            </div>
            </div>
            <!-- Status Column -->
            <div class="portfolio-assets-cell status-col">
               ${getStatusBadge(inv.status, inv.payout_expected_at)}
            </div>
            <!-- Actions Column -->
            <div class="portfolio-assets-cell actions-col">
            <button class="portfolio-assets-action-btn" aria-label="View actions">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <circle cx="5" cy="10" r="1.5" fill="#717680"></circle>
                <circle cx="10" cy="10" r="1.5" fill="#717680"></circle>
                <circle cx="15" cy="10" r="1.5" fill="#717680"></circle>
                </svg>
            </button>
            </div>
        </div>
        `;
    });

    tableBody.innerHTML = rowsHtml;
}

// Initialize on DOM Load
document.addEventListener('DOMContentLoaded', () => {
    // Hide standard elements while loading initially
    const valueSection = document.getElementById('portfolio-value-section');
    const keyFinSection = document.getElementById('key-financials-section');
    const insightsSection = document.getElementById('insights-limit-section');
    const assetsSection = document.getElementById('assets-section');

    if (valueSection) valueSection.classList.add('hidden');
    if (keyFinSection) keyFinSection.classList.add('hidden');
    if (insightsSection) insightsSection.classList.add('hidden');
    if (assetsSection) assetsSection.classList.add('hidden');

    fetchPortfolioData();
});
