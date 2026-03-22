/**
 * PortfolioDataService  –  Phase 2: Backend-Logik & Data Fetching
 *
 * Responsibilities:
 *   - Single API call to GET /api/portfolio
 *   - Strict data validation & business logic (appreciation calc, progress %)
 *   - Returns typed, display-ready objects consumed by the UI layer
 *
 * @typedef {Object} InvestmentItem
 * @property {string}      asset_title
 * @property {string}      asset_slug
 * @property {string|null} cover_image
 * @property {number}      current_value_cents
 * @property {number}      purchase_value_cents
 * @property {number}      appreciation_pct_bps  – basis points (100 = 1%)
 * @property {number}      total_rental_cents
 * @property {string}      status
 * @property {string|null} payout_expected_at
 * @property {number}      tokens_owned
 *
 * @typedef {Object} AnnualLimit
 * @property {number} annual_limit_cents
 * @property {number} invested_12m_cents
 * @property {number} available_cents
 *
 * @typedef {Object} PortfolioResponse
 * @property {number}          total_value_cents
 * @property {number}          total_purchase_cents
 * @property {number}          total_appreciation_cents
 * @property {number}          monthly_income_cents
 * @property {number}          total_rental_cents
 * @property {number}          investment_count
 * @property {number}          occupancy_rate_bps
 * @property {number}          annual_yield_bps
 * @property {AnnualLimit|null} annual_limit
 * @property {InvestmentItem[]} investments
 */
const PortfolioDataService = (function () {
    "use strict";

    const PORTFOLIO_ENDPOINT = "/api/portfolio";

    // ─── Formatters ───────────────────────────────────────────────

    /**
     * Format cents → "$X,XXX" (no decimals, standard US format)
     * @param {number} cents
     * @returns {string}
     */
    function formatCurrency(cents) {
        if (typeof cents !== "number" || isNaN(cents)) return "$0";
        return new Intl.NumberFormat("en-US", {
            style: "currency",
            currency: "USD",
            minimumFractionDigits: 0,
            maximumFractionDigits: 0,
        }).format(cents / 100);
    }

    /**
     * Format cents → "X,XXX" (no sign, no currency symbol)
     * @param {number} cents
     * @returns {string}
     */
    function formatAmount(cents) {
        if (typeof cents !== "number" || isNaN(cents)) return "0";
        return new Intl.NumberFormat("en-US", {
            minimumFractionDigits: 0,
            maximumFractionDigits: 0,
        }).format(cents / 100);
    }

    /**
     * Format basis points → percentage string, e.g. 1000 bps → "10.0%"
     * @param {number} bps
     * @param {number} [decimals=1]
     * @returns {string}
     */
    function formatBps(bps, decimals = 1) {
        if (typeof bps !== "number" || isNaN(bps)) return "0%";
        return `${(bps / 100).toFixed(decimals)}%`;
    }

    /**
     * Calculate the appreciation percentage from raw totals.
     * @param {number} appreciationCents
     * @param {number} purchaseCents
     * @returns {{ value: number, display: string, isPositive: boolean }}
     */
    function calcAppreciation(appreciationCents, purchaseCents) {
        const value =
            purchaseCents > 0 ? (appreciationCents / purchaseCents) * 100 : 0;
        const isPositive = value >= 0;
        const display = `${isPositive ? "+" : ""}${value.toFixed(1)}%`;
        return { value, display, isPositive };
    }

    /**
     * Calculate progress percentage for the investment limit bar.
     * @param {number} invested
     * @param {number} limit
     * @returns {number} capped 0-100
     */
    function calcLimitProgress(invested, limit) {
        if (!limit || limit === 0) return 0;
        return Math.min(100, (invested / limit) * 100);
    }

    /**
     * Map investment status string → display-ready badge info.
     * @param {string} status
     * @param {string|null} payoutAt
     * @returns {{ cssClass: string, label: string }}
     */
    function mapInvestmentStatus(status, payoutAt) {
        const s = (status || "").toLowerCase();
        if (s.includes("funded"))
            return { cssClass: "status-funded", label: "Property funded" };
        if (s.includes("payout")) {
            const month = payoutAt
                ? new Date(payoutAt).toLocaleString("en-US", { month: "short" })
                : "Soon";
            return {
                cssClass: "status-payout",
                label: `Payout expected: ${month}`,
            };
        }
        if (s.includes("rented"))
            return { cssClass: "status-rented", label: "Rented" };
        if (s.includes("exited"))
            return { cssClass: "status-exited", label: "Exited" };
        return { cssClass: "status-process", label: "In process" };
    }

    // ─── Fetch & Transform ────────────────────────────────────────

    /**
     * Fetch raw portfolio data and apply business logic.
     *
     * @returns {Promise<PortfolioPageData>} ready-to-render data object
     *
     * PortfolioPageData shape:
     * {
     *   hasInvestments:   boolean,
     *   totalValue:       string,      // "$39,000"
     *   appreciation:     { display, isPositive },
     *   monthlyIncome:    string,
     *   totalRental:      string,
     *   totalAppreciation:string,
     *   investmentCount:  number,
     *   occupancyRate:    string,      // "95%"
     *   annualYield:      string,      // "7.2%"
     *   periodLabel:      string,      // "March 2026"
     *   limit:            null | {
     *     annualDisplay:    string,
     *     investedDisplay:  string,
     *     availableDisplay: string,
     *     progressPct:      number,
     *     progressLabel:    string,
     *   },
     *   investments: InvestmentRow[],
     *   pieChartData: { label, percentage, color }[],
     * }
     *
     * InvestmentRow:
     * {
     *   assetTitle, assetSlug, coverImage,
     *   currentValueDisplay, appreciationDisplay, appreciationClass, appreciationPrefix,
     *   totalRentalDisplay,
     *   statusCss, statusLabel,
     *   tokensOwned,
     * }
     */
    async function getPortfolioData() {
        let raw = null;

        try {
            const rawInjected = document.getElementById("server-portfolio-json")?.textContent.trim();
            if (rawInjected && rawInjected !== "null") {
                raw = JSON.parse(rawInjected);
            }
        } catch (e) {
            console.warn("Failed to parse injected portfolio json, falling back to fetch", e);
        }

        if (!raw) {
            const res = await fetch(PORTFOLIO_ENDPOINT, {
                credentials: "same-origin",
                headers: { Accept: "application/json" },
            });

            if (!res.ok) {
                if (res.status === 401) {
                    window.location.href = "/auth/login";
                    return null;
                }
                throw new Error(`API returned ${res.status} ${res.statusText}`);
            }

            raw = await res.json();
        }

        // ── Scalars ──
        const appreciation = calcAppreciation(
            raw.total_appreciation_cents ?? 0,
            raw.total_purchase_cents ?? 0
        );

        const now = new Date();
        const periodLabel = now.toLocaleString("en-US", {
            month: "long",
            year: "numeric",
        });

        // ── Investment Limit ──
        let limit = null;
        if (raw.annual_limit) {
            const al = raw.annual_limit;
            const progressPct = calcLimitProgress(
                al.invested_12m_cents,
                al.annual_limit_cents
            );
            limit = {
                annualDisplay: formatCurrency(al.annual_limit_cents),
                investedDisplay: formatCurrency(al.invested_12m_cents),
                availableDisplay: formatCurrency(al.available_cents),
                progressPct,
                progressLabel: `${progressPct.toFixed(1)}% of limit used`,
            };
        }

        // ── Investment rows ──
        const investments = (raw.investments ?? []).map((inv) => {
            const appBps = inv.appreciation_pct_bps ?? 0;
            const isPositive = appBps >= 0;
            const { cssClass: statusCss, label: statusLabel } = mapInvestmentStatus(
                inv.status,
                inv.payout_expected_at
            );
            return {
                assetTitle: inv.asset_title,
                assetSlug: inv.asset_slug,
                coverImage: inv.cover_image || "/static/images/property-placeholder.webp",
                currentValueDisplay: formatCurrency(inv.current_value_cents ?? 0),
                appreciationDisplay: `${isPositive ? "+" : ""}${(appBps / 100).toFixed(1)}%`,
                appreciationClass: isPositive ? "positive" : "negative",
                appreciationPrefix: isPositive ? "+" : "",
                totalRentalDisplay: formatCurrency(inv.total_rental_cents ?? 0),
                statusCss,
                statusLabel,
                originalStatus: inv.status,
                tokensOwned: inv.tokens_owned ?? 0,
                id: inv.id,
                isWithin48h: inv.is_within_48h ?? false,
                chainContractAddress: inv.chain_contract_address || null,
                chainTxHash: inv.chain_tx_hash || null,
            };
        });


        // ── Pie chart data ──
        const totalValue = raw.total_value_cents ?? 0;
        const COLORS = ["#98FB96", "#0000FF", "#FF6B6B", "#FFD700", "#9B59B6"];
        const rawInvs = raw.investments ?? [];
        let pieChartData = rawInvs.map((inv, idx) => ({
            label: inv.asset_title,
            percentage: totalValue > 0
                ? Math.round((inv.current_value_cents / totalValue) * 100)
                : 0,
            color: COLORS[idx % COLORS.length],
        }));
        // Fix rounding to sum to 100
        const pctSum = pieChartData.reduce((s, c) => s + c.percentage, 0);
        if (pctSum !== 100 && pieChartData.length > 0) {
            pieChartData[0].percentage += 100 - pctSum;
        }

        return {
            hasInvestments: investments.length > 0,
            totalValue: formatCurrency(raw.total_value_cents ?? 0),
            appreciation,
            monthlyIncome: formatAmount(raw.monthly_income_cents ?? 0),
            totalRental: formatAmount(raw.total_rental_cents ?? 0),
            totalAppreciation: formatAmount(raw.total_appreciation_cents ?? 0),
            investmentCount: raw.investment_count ?? 0,
            occupancyRate: formatBps(raw.occupancy_rate_bps ?? 0, 0),
            annualYield: formatBps(raw.annual_yield_bps ?? 0, 1),
            periodLabel,
            limit,
            investments,
            pieChartData,
        };
    }

    async function cancelInvestment(investmentId) {
        const res = await fetch("/api/portfolio/cancel", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ investment_id: investmentId })
        });
        const json = await res.json();
        if (!res.ok) throw new Error(json.error || "Failed to cancel investment");
        return json;
    }

    // ─── Exports ──────────────────────────────────────────────────
    return {
        getPortfolioData,
        cancelInvestment,
        // Expose helpers for unit testing
        formatCurrency,
        formatAmount,
        formatBps,
        calcAppreciation,
        calcLimitProgress,
        mapInvestmentStatus,
    };
})();

