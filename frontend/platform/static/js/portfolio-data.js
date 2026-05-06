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

  // ─── XSS-safe HTML escaper ───────────────────────────────────
  function escHtml(str) {
    if (typeof str !== "string") return String(str);
    var d = document.createElement("div");
    d.appendChild(document.createTextNode(str));
    return d.innerHTML;
  }

  /** Sanitize slug to only allow safe URL path characters */
  function safeSlug(slug) {
    if (typeof slug !== "string") return "";
    return slug.replace(/[^a-zA-Z0-9_-]/g, "");
  }

  /**
   * Prompt the user's browser wallet (MetaMask, Coinbase Wallet, etc.) to
   * add a POOOL property NFT (ERC-1155, token id = 1) to their wallet's
   * NFTs tab. Uses the EIP-747 `wallet_watchAsset` RPC.
   *
   * Pre-flight: ensures the wallet is on the same chain as the contract,
   * because `wallet_watchAsset` checks `balanceOf(activeAccount, tokenId)`
   * and rejects with "ownership details don't match" if the wallet sees a
   * 0 balance (which it does when on a different network).
   *
   * On failure (no extension, wrong chain unrecoverable, ERC-1155 not
   * supported by this wallet version, the active account doesn't own
   * any tokens) we copy the contract address to the clipboard and surface
   * manual-import instructions so the user is never stuck.
   *
   * Exposed as `window._addPropertyToMetaMask(contractAddress, name, image, network)`.
   * `network` is "polygon" or "polygon_amoy" (matches `assets.chain_network`).
   */
  const CHAINS = {
    polygon: {
      chainId: "0x89", // 137
      chainName: "Polygon",
      nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 },
      rpcUrls: ["https://polygon-rpc.com"],
      blockExplorerUrls: ["https://polygonscan.com"],
    },
    polygon_amoy: {
      chainId: "0x13882", // 80002
      chainName: "Polygon Amoy Testnet",
      nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 },
      rpcUrls: ["https://rpc-amoy.polygon.technology"],
      blockExplorerUrls: ["https://amoy.polygonscan.com"],
    },
  };

  async function ensureChain(eth, networkKey) {
    const target = CHAINS[networkKey] || CHAINS.polygon;
    let current;
    try { current = await eth.request({ method: "eth_chainId" }); } catch (_) {}
    if (current && current.toLowerCase() === target.chainId.toLowerCase()) return target;
    try {
      await eth.request({
        method: "wallet_switchEthereumChain",
        params: [{ chainId: target.chainId }],
      });
      return target;
    } catch (e) {
      // 4902 = chain not added; add it then retry.
      if (e && (e.code === 4902 || (e.data && e.data.originalError && e.data.originalError.code === 4902))) {
        await eth.request({ method: "wallet_addEthereumChain", params: [target] });
        return target;
      }
      throw e;
    }
  }

  function manualImportHint(contractAddress, target) {
    return (
      "Manual import:\n" +
      "  → Switch wallet to " + (target ? target.chainName : "the correct network") + "\n" +
      "  → MetaMask → NFTs tab → Import NFT\n" +
      "  → Address: " + contractAddress + " (copied to clipboard)\n" +
      "  → Token ID: 1\n\n" +
      "Note: your active wallet account must be the one that owns the tokens."
    );
  }

  window._addPropertyToMetaMask = async function (contractAddress, name, image, networkKey) {
    if (!contractAddress) return;
    const eth = window.ethereum;
    if (!eth || typeof eth.request !== "function") {
      try { await navigator.clipboard.writeText(contractAddress); } catch (_) {}
      alert(
        "No browser wallet detected.\n\n" +
        "Install MetaMask first, then come back and click again.\n\n" +
        manualImportHint(contractAddress, CHAINS[networkKey] || CHAINS.polygon)
      );
      return;
    }
    let target;
    try {
      target = await ensureChain(eth, networkKey);
    } catch (e) {
      const msg = (e && e.message) || String(e);
      try { await navigator.clipboard.writeText(contractAddress); } catch (_) {}
      alert(
        "Could not switch your wallet to the right network (" + msg + ").\n\n" +
        manualImportHint(contractAddress, CHAINS[networkKey] || CHAINS.polygon)
      );
      return;
    }
    try {
      const ok = await eth.request({
        method: "wallet_watchAsset",
        params: {
          type: "ERC1155",
          options: {
            address: contractAddress,
            tokenId: "1",
            name: name || "POOOL Property",
            image: image || undefined,
          },
        },
      });
      if (ok === false) throw new Error("Wallet declined the request.");
    } catch (err) {
      const msg = (err && err.message) || String(err);
      const ownership = /ownership|owner|balance/i.test(msg);
      try { await navigator.clipboard.writeText(contractAddress); } catch (_) {}
      alert(
        (ownership
          ? "MetaMask couldn't add this NFT because the active account doesn't own any tokens of it.\n\n" +
            "Switch MetaMask to the wallet you bound during KYC (the one that received the property tokens) and try again."
          : "Could not add to your wallet automatically (" + msg + ")."
        ) + "\n\n" + manualImportHint(contractAddress, target)
      );
    }
  };

  // ─── State Layer IDs ─────────────────────────────────────────
  const SECTION_IDS = [
    "portfolio-value-section",
    "key-financials-section",
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

  /** Mark onboarding steps complete based on the current user profile. */
  function markEmptyStateSteps() {
    const user = window.__POOOL_USER;
    if (!user) return;
    const kycDone = ["verified", "approved", "completed"].includes(
      String(user.kyc_status || user.kycStatus || "").toLowerCase()
    );
    const walletFunded =
      Number(user.wallet_balance ?? user.walletBalance ?? 0) > 0;
    const completed = {
      verify: kycDone,
      fund: walletFunded,
      pick: false,
    };
    document.querySelectorAll(".portfolio-empty__step").forEach((el) => {
      const id = el.getAttribute("data-step-id");
      if (completed[id]) el.setAttribute("data-complete", "true");
      else el.removeAttribute("data-complete");
    });
  }

  // ─── DOM Updaters ────────────────────────────────────────────

  function updateValueCard(data) {
    setText("portfolio-total-value", data.totalValue);
    if (window.CurrencyService && data.totalValue) {
      window.CurrencyService.attachIdrSubtitle(data.totalValue, "portfolio-total-value");
    }

    // Mobile
    const mobileAmount = document.querySelector(".mobile-portfolio-value-amount");
    if (mobileAmount) {
      mobileAmount.textContent = data.totalValue;
      if (!mobileAmount.id) mobileAmount.id = "mobile-portfolio-total-value-id";
      if (window.CurrencyService && data.totalValue) {
        window.CurrencyService.attachIdrSubtitle(data.totalValue, mobileAmount.id);
      }
    }

    const appEl = document.getElementById("portfolio-appreciation-percentage");
    if (appEl) appEl.textContent = data.appreciation.display;

    const mobileApp = document.querySelector(".mobile-portfolio-value-change");
    if (mobileApp) mobileApp.textContent = data.appreciation.display;

    // Tone badge to match actual direction: zero = neutral grey, negative = red.
    const v = data.appreciation.value;
    const tone = v === 0 ? "is-neutral" : v < 0 ? "is-negative" : "";
    document.querySelectorAll(".portfolio-value-badge, .mobile-portfolio-value-badge").forEach((b) => {
      b.classList.remove("is-neutral", "is-negative");
      if (tone) b.classList.add(tone);
    });
  }

  function updateKeyFinancials(data) {
    function updateChange(id, val, isPositive) {
      const el = document.getElementById(id);
      if (!el) return;
      el.textContent = val;
      const parent = el.closest(".financials-change") || el.closest(".mobile-financial-change-badge") || el.closest(".mobile-financial-change");
      if (parent) {
        parent.classList.remove("change-increase", "change-decrease", "change-neutral");
        if (val === "—") {
          parent.classList.add("change-neutral");
        } else {
          parent.classList.add(isPositive ? "change-increase" : "change-decrease");
        }
      }
    }

    // Desktop
    setText("portfolio-monthly-income", data.monthlyIncome);

    setText("portfolio-total-rental", data.totalRental);

    setText("portfolio-total-appreciation", data.totalAppreciation);
    
    updateChange("portfolio-monthly-income-change", "—", false);
    updateChange("portfolio-total-rental-change", "—", false);
    updateChange("portfolio-total-appreciation-change", data.appreciation.display, data.appreciation.isPositive);

    const period = data.periodLabel;
    setText("portfolio-monthly-income-period", period);
    setText("portfolio-total-rental-period", period);
    setText("portfolio-total-appreciation-period", period);

    // Mobile
    setText("mobile-portfolio-monthly-income", data.monthlyIncome);

    setText("mobile-portfolio-total-rental", data.totalRental);

    setText("mobile-portfolio-total-appreciation", data.totalAppreciation);
    
    updateChange("mobile-portfolio-monthly-income-change", "—", false);
    updateChange("mobile-portfolio-total-rental-change", "—", false);
    updateChange("mobile-portfolio-total-appreciation-change", data.appreciation.display, data.appreciation.isPositive);
    
    setText("mobile-portfolio-monthly-income-period", period);
    setText("mobile-portfolio-total-rental-period", period);
    setText("mobile-portfolio-total-appreciation-period", period);
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



  function buildStatusBadgeHtml(statusCss, statusLabel) {
    return `<div class="portfolio-assets-status ${statusCss}">
      <span class="portfolio-assets-status-icon"></span>
      <span class="portfolio-assets-status-text">${statusLabel}</span>
    </div>`;
  }

  const POLYGONSCAN_BASE = "https://polygonscan.com";

  function buildChainBadge(inv) {
    if (!inv.chainContractAddress) return "";
    const addr = escHtml(inv.chainContractAddress);
    const txLink = inv.chainTxHash
      ? `${POLYGONSCAN_BASE}/tx/${escHtml(inv.chainTxHash)}`
      : `${POLYGONSCAN_BASE}/address/${addr}`;
    return `<a href="${txLink}" target="_blank" rel="noopener noreferrer"
      title="View on Polygonscan" class="chain-badge"
      style="display:inline-flex; align-items:center; gap:4px; font-size:11px; color:#7C3AED; text-decoration:none; background:#F3F0FF; border:1px solid #DDD6FE; border-radius:4px; padding:2px 6px; margin-left:6px;">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#7C3AED" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
      </svg>
      <span>On-chain</span>
    </a>`;
  }

  function updateAssetsTable(investments) {
    const body = document.getElementById("portfolio-assets-body");
    if (!body) return;

    if (investments.length === 0) {
      body.innerHTML = `
        <div class="portfolio-assets-row" style="justify-content:center; padding:48px;">
            <div style="display:flex; flex-direction:column; align-items:center; gap:12px; justify-content:center;">
              <img src="/static/images/icons/home-smile.svg" alt="No assets" width="48" height="48" style="opacity:0.4;">
              <span style="color:#667085; font-size:14px;">No investments found.</span>
              <a href="/marketplace" style="color:#0000ff; font-size:14px; text-decoration:none;">Browse the Marketplace →</a>
            </div>
        </div>`;
      return;
    }

    body.innerHTML = investments.map((inv) => {
      const slug = safeSlug(inv.assetSlug);
      const title = escHtml(inv.assetTitle);
      const cover = escHtml(inv.coverImage);
      const statusCss = escHtml(inv.statusCss);
      const statusLabel = escHtml(inv.statusLabel);
      
      return `
      <div class="portfolio-assets-row" onclick="window.location.href='/property/${slug}'" style="cursor:pointer;">
        <div class="portfolio-assets-cell property-col">
          <div style="display:flex; align-items:center; gap:16px;">
            <img src="${cover}" alt="${title}" style="width: 56px; height: 40px; border-radius: 6px; object-fit: cover;" onerror="this.outerHTML='<div class=\'property-image-placeholder\'><svg width=\'20\' height=\'20\' viewBox=\'0 0 24 24\' fill=\'none\' stroke=\'currentColor\' stroke-width=\'1.5\' stroke-linecap=\'round\' stroke-linejoin=\'round\'><rect x=\'3\' y=\'3\' width=\'18\' height=\'18\' rx=\'2\' ry=\'2\'></rect><circle cx=\'8.5\' cy=\'8.5\' r=\'1.5\'></circle><polyline points=\'21 15 16 10 5 21\'></polyline></svg></div>'">
            <div style="font-weight: 700; color: #101828; font-size: 14px; line-height: 1.4; max-width: 200px;">
              ${title}
            </div>
          </div>
        </div>
        <div class="portfolio-assets-cell investment-col">
          <div style="font-weight: 700; color: #101828; font-size: 14px;">${escHtml(inv.currentValueDisplay)}</div>
        </div>
        <div class="portfolio-assets-cell appreciation-col">
          <span class="ds-badge" style="background: #FFFFFF; color: #475467; border: 1px solid #E9EAEB; padding: 2px 6px; font-weight: 600; font-size: 11px; display:inline-flex; align-items:center; gap:2px; border-radius: 4px;">
            <svg width="8" height="8" viewBox="0 0 12 12" fill="none"><path d="M3.5 8.5L8.5 3.5M8.5 3.5H3.5M8.5 3.5V8.5" stroke="#475467" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
            ${escHtml(inv.appreciationDisplay)}
          </span>
        </div>
        <div class="portfolio-assets-cell rental-col">
          <div style="font-weight: 700; color: #101828; font-size: 14px;">${escHtml(inv.totalRentalDisplay)}</div>
        </div>
        <div class="portfolio-assets-cell status-col">
          ${buildStatusBadgeHtml(statusCss, statusLabel)}
        </div>
        <div class="portfolio-assets-cell actions-col" onclick="event.stopPropagation();">
          ${inv.chainContractAddress ? `
          <button class="portfolio-assets-detail-btn"
            type="button"
            aria-label="Add ${title} to MetaMask"
            title="View NFT in MetaMask"
            style="margin-right:4px;"
            onclick="window._addPropertyToMetaMask('${escHtml(inv.chainContractAddress)}', '${title}', '${cover}', '${escHtml(inv.chainNetwork || 'polygon')}');">
            <svg width="18" height="18" viewBox="0 0 32 32" aria-hidden="true">
              <path d="M28.8 3.2 17.6 11.4l2.1-4.9z" fill="#E2761B"/>
              <path d="M3.2 3.2 14.3 11.5 12.3 6.5z" fill="#E4761B"/>
              <path d="M24.5 21.6l-3 4.6 6.4 1.8 1.8-6.3z" fill="#E4761B"/>
              <path d="M2.4 21.7l1.8 6.3 6.4-1.8-3-4.6z" fill="#E4761B"/>
              <path d="M10.2 14.4 8.5 17l6.3.3-.2-6.8z" fill="#E4761B"/>
              <path d="M21.8 14.4 17.4 10.4l-.1 6.9 6.3-.3z" fill="#E4761B"/>
              <path d="M10.6 26.2 14.4 24.3 11.1 21.8z" fill="#E4761B"/>
              <path d="M17.6 24.3 21.4 26.2 20.9 21.8z" fill="#E4761B"/>
            </svg>
          </button>
          ` : ''}
          <button class="portfolio-assets-detail-btn"
            type="button"
            aria-label="View details for ${title}"
            title="View details"
            onclick="window.location.href='/property/${slug}'">
            <svg width="18" height="18" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path d="M7.5 5L12.5 10L7.5 15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>
            </svg>
          </button>
        </div>
      </div>`;}).join("");
  }

  // ─── Cancel Action Binding ─────────────────────────────────────
  window.cancelInvestment = async function(id) {
    if (!confirm("Are you sure you want to unconditionally cancel this investment and receive a full refund?")) {
      return;
    }
    const btn = document.getElementById(`cancel-btn-${id}`);
    if (btn) btn.disabled = true;
    try {
      await PortfolioDataService.cancelInvestment(id);
      alert("Investment cancelled. Your wallet has been refunded.");
      window.location.reload();
    } catch (e) {
      alert("Error: " + e.message);
      if (btn) btn.disabled = false;
    }
  };


  function updateMobileAssetsTable(investments) {
    const body = document.getElementById("mobile-portfolio-assets-body");
    if (!body) return;

    if (investments.length === 0) {
      body.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:24px; color:#667085;">No investments found.</td></tr>`;
      return;
    }

    body.innerHTML = investments.map((inv) => {
      const slug = safeSlug(inv.assetSlug);
      const title = escHtml(inv.assetTitle);
      const cover = escHtml(inv.coverImage);
      const statusCss = escHtml(inv.statusCss);
      const statusLabel = escHtml(inv.statusLabel);
      return `
      <tr class="mobile-assets-row"
        onclick="window.location.href='/property/${slug}'" style="cursor:pointer;">
        <td class="mobile-assets-cell-property">
          <div class="mobile-assets-property-content">
            <img loading="lazy" src="${cover}" alt="Property"
              class="mobile-assets-property-image" onerror="this.src='/static/images/portfolio_asset_details/Property%20image.webp'" />
            <div class="mobile-assets-property-text-wrapper">
              <span class="mobile-assets-property-line"
                style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:120px; display:inline-block;">
                ${title}
              </span>
            </div>

          </div>
        </td>
        <td class="mobile-assets-cell-investment">
          <div class="mobile-assets-investment-content">
            <span class="mobile-assets-investment-value">${escHtml(inv.currentValueDisplay)}</span>
            <div class="mobile-assets-change-badge">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M3 9L9 3M9 3H5M9 3V7" stroke="#17B26A" stroke-width="1.5"
                  stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>${escHtml(inv.appreciationDisplay)}</span>
            </div>
          </div>
        </td>
        <td class="mobile-assets-cell-rental">
          <span class="mobile-assets-rental-value">${escHtml(inv.totalRentalDisplay)}</span>
        </td>
        <td class="mobile-assets-cell-status">
          <div class="mobile-assets-status-badge ${statusCss}">
            <span class="portfolio-assets-status-icon"></span>
            <span style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:60px;">${statusLabel}</span>
          </div>
          ${inv.chainContractAddress ? `
          <button type="button"
            aria-label="Add ${title} to MetaMask"
            title="View NFT in MetaMask"
            style="margin-top:6px; background:none; border:none; padding:2px; cursor:pointer;"
            onclick="event.stopPropagation(); window._addPropertyToMetaMask('${escHtml(inv.chainContractAddress)}', '${title}', '${cover}', '${escHtml(inv.chainNetwork || 'polygon')}');">
            <svg width="14" height="14" viewBox="0 0 32 32" aria-hidden="true">
              <path d="M28.8 3.2 17.6 11.4l2.1-4.9z" fill="#E2761B"/>
              <path d="M3.2 3.2 14.3 11.5 12.3 6.5z" fill="#E4761B"/>
              <path d="M24.5 21.6l-3 4.6 6.4 1.8 1.8-6.3z" fill="#E4761B"/>
              <path d="M2.4 21.7l1.8 6.3 6.4-1.8-3-4.6z" fill="#E4761B"/>
              <path d="M10.2 14.4 8.5 17l6.3.3-.2-6.8z" fill="#E4761B"/>
              <path d="M21.8 14.4 17.4 10.4l-.1 6.9 6.3-.3z" fill="#E4761B"/>
              <path d="M10.6 26.2 14.4 24.3 11.1 21.8z" fill="#E4761B"/>
              <path d="M17.6 24.3 21.4 26.2 20.9 21.8z" fill="#E4761B"/>
            </svg>
          </button>
          ` : ''}
        </td>
      </tr>`;}).join("");
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
        // Empty state: single CTA panel. Hide chart + zero-filled KPI grid to avoid
        // telling the user their empty portfolio "grew".
        switchState(["portfolio-empty-state"]);
        markEmptyStateSteps();
        return;
      }

      // Content state: populate all sections
      updateValueCard(data);
      updateKeyFinancials(data);
      updateInsights(data);
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
