/**
 * portfolio-data.js  –  Phase 3 & 4: Frontend UI, State Binding & QA
 *
 * Consumes PortfolioDataService (portfolio-service.js) and manages
 * four UI state layers:
 *   1. loading  – skeleton shimmer shown immediately
 *   2. error    – shown if the API call fails
 *   3. empty    – shown when the user has no investments
 *   4. content  – all portfolio sections
 *
 * All DOM mutation is isolated here; the service is pure data.
 */
(function () {
  "use strict";

  // ─── State Layer IDs ─────────────────────────────────────────
  const SECTION_IDS = [
    "portfolio-value-section",
    "key-financials-section",
    "insights-limit-section",
    "assets-section",
  ];

  const ALL_LAYER_IDS = [
    "portfolio-loading-skeleton",
    "portfolio-error-state",
    "portfolio-empty-state",
    ...SECTION_IDS,
  ];

  // ─── Helpers ─────────────────────────────────────────────────

  function show(id) {
    const el = document.getElementById(id);
    if (el) el.classList.remove("hidden");
  }

  function hide(id) {
    const el = document.getElementById(id);
    if (el) el.classList.add("hidden");
  }

  function setText(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  }

  /** Show only the listed IDs; hide everything else in ALL_LAYER_IDS. */
  function switchState(visibleIds) {
    ALL_LAYER_IDS.forEach((id) => {
      if (visibleIds.includes(id)) show(id);
      else hide(id);
    });
  }

  // ─── DOM Updaters ────────────────────────────────────────────

  function updateValueCard(data) {
    setText("portfolio-total-value", data.totalValue);
    // Mobile
    const mobileAmount = document.querySelector(".mobile-portfolio-value-amount");
    if (mobileAmount) mobileAmount.textContent = data.totalValue;

    const appEl = document.getElementById("portfolio-appreciation-percentage");
    if (appEl) appEl.textContent = data.appreciation.display;

    const mobileApp = document.querySelector(".mobile-portfolio-value-change");
    if (mobileApp) mobileApp.textContent = data.appreciation.display;
  }

  function updateKeyFinancials(data) {
    // Desktop
    setText("portfolio-monthly-income", data.monthlyIncome);
    setText("portfolio-total-rental", data.totalRental);
    setText("portfolio-total-appreciation", data.totalAppreciation);
    setText("portfolio-monthly-income-change", "N/A");
    setText("portfolio-total-rental-change", "N/A");
    setText("portfolio-total-appreciation-change", data.appreciation.display);

    const period = data.periodLabel;
    setText("portfolio-monthly-income-period", period);
    setText("portfolio-total-rental-period", `as of ${period}`);
    setText("portfolio-total-appreciation-period", `as of ${period}`);

    // Mobile
    setText("mobile-portfolio-monthly-income", data.monthlyIncome);
    setText("mobile-portfolio-total-rental", data.totalRental);
    setText("mobile-portfolio-total-appreciation", data.totalAppreciation);
    setText("mobile-portfolio-monthly-income-change", "N/A");
    setText("mobile-portfolio-total-rental-change", "N/A");
    setText("mobile-portfolio-total-appreciation-change", data.appreciation.display);
    setText("mobile-portfolio-monthly-income-period", period);
    setText("mobile-portfolio-total-rental-period", `as of ${period}`);
    setText("mobile-portfolio-total-appreciation-period", `as of ${period}`);
  }

  function updateInsights(data) {
    setText("insights-value-number-of-properties", data.investmentCount);
    setText("insights-value-occupancy-rate", data.occupancyRate);
    setText("insights-value-annual-rental-yield", data.annualYield);

    // Mobile
    setText("mobile-insights-value-number-of-properties", data.investmentCount);
    setText("mobile-insights-value-occupancy-rate", data.occupancyRate);
    setText("mobile-insights-value-annual-rental-yield", data.annualYield);
  }

  function updateInvestmentLimit(data) {
    if (!data.limit) return;
    const lim = data.limit;

    // Desktop
    setText("investment-limit-annual-value", lim.annualDisplay);
    setText("investment-limit-invested-value", lim.investedDisplay);
    setText("investment-limit-available-value", lim.availableDisplay);
    setText("progress-percentage-text", lim.progressLabel);

    const desktopBar = document.getElementById("progress-bar-fill-desktop");
    if (desktopBar) desktopBar.style.width = `${lim.progressPct}%`;

    // Mobile
    setText("mobile-investment-limit-annual-value", lim.annualDisplay);
    setText("mobile-investment-limit-invested-value", lim.investedDisplay);
    setText("mobile-investment-limit-available-value", lim.availableDisplay);
    setText("mobile-progress-percentage-text", lim.progressLabel);

    const mobileBar = document.getElementById("progress-bar-fill-mobile");
    if (mobileBar) mobileBar.style.width = `${lim.progressPct}%`;
  }

  function buildStatusBadgeHtml(statusCss, statusLabel) {
    return `<div class="portfolio-assets-status ${statusCss}">
      <span class="portfolio-assets-status-icon"></span>
      <span class="portfolio-assets-status-text">${statusLabel}</span>
    </div>`;
  }

  function updateAssetsTable(investments) {
    const body = document.getElementById("portfolio-assets-body");
    if (!body) return;

    if (investments.length === 0) {
      body.innerHTML = `
        <div class="portfolio-assets-row" style="justify-content:center; padding:48px;">
          <div style="display:flex; flex-direction:column; align-items:center; gap:12px; text-align:center;">
            <img src="/images/home-smile.svg" alt="No assets" width="48" height="48" style="opacity:0.4;">
            <span style="color:#667085; font-size:14px;">No investments found.</span>
            <a href="/marketplace" style="color:#0000ff; font-size:14px; text-decoration:none;">Browse the Marketplace →</a>
          </div>
        </div>`;
      return;
    }

    body.innerHTML = investments.map((inv) => `
      <div class="portfolio-assets-row">
        <div class="portfolio-assets-cell property-col">
          <div class="portfolio-assets-property">
            <div class="portfolio-assets-property-image">
              <img loading="lazy" src="${inv.coverImage}" alt="${inv.assetTitle}"
                width="48" height="48" onerror="this.src='/images/property-placeholder.webp'" />
            </div>
            <div class="portfolio-assets-property-info">
              <div class="portfolio-assets-property-name">${inv.assetTitle}</div>
            </div>
          </div>
        </div>
        <div class="portfolio-assets-cell investment-col">
          <div class="portfolio-assets-investment">
            <div class="portfolio-assets-value">${inv.currentValueDisplay}</div>
            <div class="portfolio-assets-change ${inv.appreciationClass}">
              <span class="portfolio-assets-change-icon"></span>
              <span>${inv.appreciationDisplay}</span>
            </div>
          </div>
        </div>
        <div class="portfolio-assets-cell rental-col">
          <div class="portfolio-assets-rental">
            <div class="portfolio-assets-value">${inv.totalRentalDisplay}</div>
          </div>
        </div>
        <div class="portfolio-assets-cell status-col">
          ${buildStatusBadgeHtml(inv.statusCss, inv.statusLabel)}
        </div>
        <div class="portfolio-assets-cell actions-col">
          <button class="portfolio-assets-action-btn"
            onclick="window.location.href='/property/${inv.assetSlug}'">
            See Details
          </button>
        </div>
      </div>`).join("");
  }

  function updateMobileAssetsTable(investments) {
    const body = document.getElementById("mobile-portfolio-assets-body");
    if (!body) return;

    if (investments.length === 0) {
      body.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:24px; color:#667085;">No investments found.</td></tr>`;
      return;
    }

    body.innerHTML = investments.map((inv) => `
      <tr class="mobile-assets-row"
        onclick="window.location.href='/property/${inv.assetSlug}'" style="cursor:pointer;">
        <td class="mobile-assets-cell-property">
          <div class="mobile-assets-property-content">
            <img loading="lazy" src="${inv.coverImage}" alt="Property"
              class="mobile-assets-property-image" onerror="this.src='/images/property-placeholder.webp'" />
            <div class="mobile-assets-property-text-wrapper">
              <span class="mobile-assets-property-line"
                style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:120px; display:inline-block;">
                ${inv.assetTitle}
              </span>
            </div>
          </div>
        </td>
        <td class="mobile-assets-cell-investment">
          <div class="mobile-assets-investment-content">
            <span class="mobile-assets-investment-value">${inv.currentValueDisplay}</span>
            <div class="mobile-assets-change-badge">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M3 9L9 3M9 3H5M9 3V7" stroke="#17B26A" stroke-width="1.5"
                  stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>${inv.appreciationDisplay}</span>
            </div>
          </div>
        </td>
        <td class="mobile-assets-cell-rental">
          <span class="mobile-assets-rental-value">${inv.totalRentalDisplay}</span>
        </td>
        <td class="mobile-assets-cell-status">
          <div class="mobile-assets-status-badge ${inv.statusCss}">
            <div class="status-dot"></div>
            <span style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:60px;">${inv.statusLabel}</span>
          </div>
        </td>
      </tr>`).join("");
  }

  function updatePieChart(pieChartData) {
    if (!pieChartData.length) return;

    const tryUpdate = () => {
      if (window.financialsPieChart) {
        window.financialsPieChart.updateData(pieChartData);
        if (window.mobileFinancialsPieChart) {
          window.mobileFinancialsPieChart.updateData(pieChartData);
        }
        return true;
      }
      return false;
    };

    if (!tryUpdate()) {
      // Retry until pie chart is initialized (max 5s)
      const interval = setInterval(() => {
        if (tryUpdate()) clearInterval(interval);
      }, 100);
      setTimeout(() => clearInterval(interval), 5000);
    }
  }

  // ─── Main Init ───────────────────────────────────────────────

  async function initPortfolioPage() {
    // Show loading skeleton immediately
    switchState(["portfolio-loading-skeleton"]);

    try {
      // Safety check
      if (typeof PortfolioDataService === "undefined") {
        console.error("PortfolioDataService not available");
        switchState(["portfolio-error-state"]);
        return;
      }

      const data = await PortfolioDataService.getPortfolioData();
      if (!data) return; // Redirected to login

      if (!data.hasInvestments) {
        // Empty state: show value card (with zeros) + empty state panel + insights
        updateValueCard(data);
        updateKeyFinancials(data);
        updateInsights(data);
        updateInvestmentLimit(data);
        switchState(["portfolio-empty-state", "portfolio-value-section", "key-financials-section", "insights-limit-section"]);
        return;
      }

      // Content state: populate all sections
      updateValueCard(data);
      updateKeyFinancials(data);
      updateInsights(data);
      updateInvestmentLimit(data);
      updateAssetsTable(data.investments);
      updateMobileAssetsTable(data.investments);
      updatePieChart(data.pieChartData);

      // Update the interactive chart with real data
      if (window.PortfolioChart && window.PortfolioChart.setData) {
        try {
          var rawJson = document.getElementById("server-portfolio-json")?.textContent.trim();
          if (rawJson && rawJson !== "null") {
            window.PortfolioChart.setData(JSON.parse(rawJson));
          }
        } catch (e) {
          console.warn("Could not pass portfolio data to chart:", e);
        }
      }

      switchState(SECTION_IDS);

    } catch (err) {
      console.error("Portfolio page load failed:", err);
      switchState(["portfolio-error-state"]);
    }
  }

  // ─── Boot ────────────────────────────────────────────────────
  function boot() {
    initPortfolioPage();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
