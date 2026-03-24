// frontend/platform/static/js/rewards-service.js

/**
 * Data Fetching Service für die Rewards-Seite.
 * Trennt die reinen API-Aufrufe und Business-Logik (Datenaufbereitung) vom UI.
 */
const RewardsDataService = (function () {
    const API_ENDPOINT = "/api/rewards";

    /**
     * Holt die Rohdaten vom Backend.
     * Implementiert rudimentäres Fehler-Handling.
     */
    async function fetchRawData() {
        try {
            const response = await fetch(API_ENDPOINT, {
                method: 'GET',
                headers: {
                    'Accept': 'application/json',
                },
                credentials: 'same-origin',
            });

            if (!response.ok) {
                throw new Error(`API Error: ${response.status} ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error('Failed to fetch rewards data:', error);
            throw error;
        }
    }

    /**
     * Holt die Daten und mappt sie in unsere strikten TypeScript-Interfaces aus Phase 1.
     * Enthält die Business-Logik (z.B. Summierungen, Prozentberechnungen).
     */
    async function getRewardsData() {
        const rawData = await fetchRawData();

        // 1. Business Logic: Dynamische Berechnung der Total Balance
        // Anstatt der API blind zu vertrauen, summieren wir die Einzelposten auf,
        // um Datenintegrität zu garantieren (Single Source of Truth).
        const breakdowns = [
            { type: 'cashback', amount: rawData.cashback || 0 },
            { type: 'referral', amount: rawData.referrals || 0 },
            { type: 'promotion', amount: rawData.promotions || 0 }
        ];

        // Berechne Gesamtguthaben
        const calculatedTotal = breakdowns.reduce((sum, item) => sum + item.amount, 0);

        const balance = {
            currency: rawData.currency || 'USD',
            totalAvailable: calculatedTotal,
            breakdowns: breakdowns
        };

        // 2. Business Logic: Dynamische Berechnung des Tier-Fortschritts
        const invested = rawData.invested_12m || 0;
        const threshold = rawData.tier_target_amount || 0;
        // Prozentwert dynamisch aus dem Backend übernehmen,
        // da das Backend den Offset des aktuellen Tiers berücksichtigt.
        const progressPercentage = rawData.progress_pct !== undefined 
            ? rawData.progress_pct 
            : (threshold > 0 ? Math.min((invested / threshold) * 100, 100) : 100);

        const tier = {
            currentTier: rawData.tier_name || 'Standard',
            nextTier: rawData.tier_target || null,
            investedLast12Months: invested,
            thresholdForNextTier: threshold,
            progressPercentage: progressPercentage
        };

        // 3. Referral Daten aufbereiten
        const referral = {
            referralLink: rawData.referral_url || '',
            friendRewardAmount: 3000, // Könnte perspektivisch aus der DB/Config kommen (in Cent)
            userRewardAmount: 3000,
            investmentRequired: 100000
        };

        // 4. Partner Metrics aufbereiten
        const metrics = {
            totalClicks: rawData.total_clicks || 0,
            totalSignups: rawData.total_signups || 0,
            qualifiedInvestors: rawData.qualified_investors || 0,
            networkTotalIn: rawData.network_total_in || 0
        };

        return { balance, tier, referral, metrics };
    }

    /**
     * Fetches campaign breakdown data from the API.
     */
    async function getCampaignData() {
        try {
            const response = await fetch('/api/rewards/campaigns', {
                method: 'GET',
                headers: { 'Accept': 'application/json' },
                credentials: 'same-origin',
            });
            if (!response.ok) return [];
            return await response.json();
        } catch (error) {
            console.warn('Failed to fetch campaign data:', error);
            return [];
        }
    }

    return {
        getRewardsData,
        getCampaignData
    };
})();
