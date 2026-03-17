// frontend/platform/static/js/rewards.js
(function () {
  "use strict";

  // Formatiert Cents in eine saubere Währungsdarstellung (z.B. "USD 1,300")
  function formatCurrency(cents, currency = 'USD') {
    return `${currency} ${(cents / 100).toLocaleString("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })}`;
  }

  // 1. Data-Binding: UI mit den prozessierten Daten aktualisieren
  function renderRewardsUI(data) {
    // Total Balance
    const totalEl = document.getElementById("rewards-total-balance");
    if (totalEl) totalEl.textContent = formatCurrency(data.balance.totalAvailable, data.balance.currency);

    // Breakdowns dynamisch mappen (basierend auf der Reihenfolge)
    const breakdownValues = document.querySelectorAll(".breakdown-value");
    if (breakdownValues.length >= 3) {
      // Mapping basierend auf unserer Service-Struktur
      const cashback = data.balance.breakdowns.find(b => b.type === 'cashback')?.amount || 0;
      const referral = data.balance.breakdowns.find(b => b.type === 'referral')?.amount || 0;
      const promotion = data.balance.breakdowns.find(b => b.type === 'promotion')?.amount || 0;

      breakdownValues[0].textContent = formatCurrency(cashback, data.balance.currency);
      breakdownValues[1].textContent = formatCurrency(referral, data.balance.currency);
      breakdownValues[2].textContent = formatCurrency(promotion, data.balance.currency);
    }

    // Tier Progress Card
    const amountEl = document.querySelector(".tp-amount");
    if (amountEl) amountEl.textContent = formatCurrency(data.tier.investedLast12Months, data.balance.currency);

    const badgeEl = document.querySelector(".tp-badge");
    if (badgeEl) {
      // Konsistentes CSS: dynamische Zuweisung basierend auf dem Tier Status
      const tierClass = data.tier.currentTier.toLowerCase();
      badgeEl.className = `tp-badge ${tierClass} text-blue`;
      badgeEl.textContent = data.tier.currentTier;
    }

    // Bedingte Logik: Progress Bar Color
    const fill = document.querySelector(".tp-progress-fill");
    if (fill) {
      const pct = data.tier.progressPercentage;
      fill.style.width = `${pct}%`;
      // Beispiel: Bei 100% ein anderes Grün (succcess statt hint)
      fill.style.background = pct >= 100 ? '#12B76A' : '#98fb96';
    }

    const hint = document.querySelector(".tp-hint");
    if (hint) {
      if (data.tier.nextTier && data.tier.thresholdForNextTier > 0) {
        hint.innerHTML = `Invest <strong class="text-blue">${formatCurrency(data.tier.thresholdForNextTier, data.balance.currency)}</strong> to reach ${data.tier.nextTier}`;
      } else {
        // Bedingtes Rendering: UI reagiert auf Endstufe
        hint.innerHTML = `<strong class="text-blue">Maximum Tier Reached</strong>`;
      }
    }

    // Referral Infos anbinden 
    const input = document.getElementById("rewards-referral-input");
    const linkGenInput = document.getElementById("campaign-generated-link");
    let baseReferralLink = data.referral.referralLink || '';
    
    if (input) {
      input.value = baseReferralLink;
      input.placeholder = "No Link generated";
    }
    
    if (linkGenInput) {
      linkGenInput.value = baseReferralLink;
      linkGenInput.dataset.baseLink = baseReferralLink;
    }

    // Dynamische Texte in der Referral Checklist (vermeidet harte Sub-Strings im HTML)
    const checklistItems = document.querySelectorAll(".refer-checklist li");
    if (checklistItems.length >= 2) {
      // Bewahrt das SVG Icon, tauscht nur den Text-Node dahinter aus
      const icon1 = checklistItems[0].querySelector('svg').outerHTML;
      checklistItems[0].innerHTML = `${icon1} Friends get ${formatCurrency(data.referral.friendRewardAmount, data.balance.currency)} upon signing up`;

      const icon2 = checklistItems[1].querySelector('svg').outerHTML;
      checklistItems[1].innerHTML = `${icon2} You get ${formatCurrency(data.referral.userRewardAmount, data.balance.currency)} after they invest ${formatCurrency(data.referral.investmentRequired, data.balance.currency)}`;
    }

    // Partner Metrics
    if (data.metrics) {
      const metricValues = document.querySelectorAll(".partner-metrics-grid .metric-card-value");
      if (metricValues.length >= 4) {
        metricValues[0].textContent = data.metrics.totalClicks.toLocaleString();
        metricValues[1].textContent = data.metrics.totalSignups.toLocaleString();
        metricValues[2].textContent = data.metrics.qualifiedInvestors.toLocaleString();
        metricValues[3].textContent = formatCurrency(data.metrics.networkTotalIn, data.balance.currency);
      }

      // Calculate CVR (Signups / Clicks * 100)
      const cvr = data.metrics.totalClicks > 0
        ? ((data.metrics.totalSignups / data.metrics.totalClicks) * 100).toFixed(1) + '% CVR'
        : '0.0% CVR';

      const trendValues = document.querySelectorAll(".partner-metrics-grid .metric-card-trend");
      if (trendValues.length >= 2) {
        trendValues[1].innerHTML = `
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="22" y1="12" x2="2" y2="12"></line>
                  <polyline points="15 5 22 12 15 19"></polyline>
                </svg>
                ${cvr}
            `;
      }
    }
  }

  // 2. Interaktivität: Event Listeners
  function copyReferralLink(elementId = "rewards-referral-input") {
    const input = document.getElementById(elementId);
    if (!input || !input.value) {
      showToast("No valid referral link to copy", "error");
      return;
    }

    // Fallback and modern clipboard API (UX Best Practice)
    if (navigator.clipboard) {
      navigator.clipboard.writeText(input.value)
        .then(() => showToast("Referral link copied!", "success"))
        .catch(() => execCopyCmd(input));
    } else {
      execCopyCmd(input);
    }
  }

  function execCopyCmd(inputElement) {
    inputElement.select();
    try {
      document.execCommand("copy");
      showToast("Referral link copied!", "success");
    } catch (err) {
      showToast("Failed to copy link", "error");
    }
  }

  // Konsistentes Styling für System Feedback
  function showToast(message, type) {
    let container = document.getElementById("rewards-toast-container");
    if (!container) {
      container = document.createElement("div");
      container.id = "rewards-toast-container";
      container.style.cssText = "position:fixed;top:24px;right:24px;z-index:9999;display:flex;flex-direction:column;gap:8px;";
      document.body.appendChild(container);
    }
    const toast = document.createElement("div");
    // Bedingtes Rendering von Farben (Rot = Fehler, Grün = Success)
    const bgColor = type === "success" ? "#12B76A" : "#F04438";
    toast.style.cssText = `padding:12px 20px;border-radius:8px;color:#fff;font-size:14px;font-weight:500;box-shadow:0 4px 12px rgba(0,0,0,0.15);transition:opacity 0.3s;background:${bgColor};`;
    toast.textContent = message;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.opacity = "0";
      setTimeout(() => toast.remove(), 300);
    }, 3000);
  }

  // Layer Manager
  function switchState(state) {
    const layers = ['loading', 'error', 'empty', 'content'];
    layers.forEach(l => {
      const el = document.getElementById(`rewards-${l}-layer`);
      if (el) {
        if (l === state) el.classList.remove('hidden');
        else el.classList.add('hidden');
      }
    });
  }

  // State for Campaign Table
  let allCampaigns = [];
  let currentSort = 'clicks';
  let sortAsc = false;
  let currentSearch = '';
  let currentCurrency = 'USD';
  let campaignChartInstance = null;

  // 3. Main Controller Logic
  async function initRewardsPage() {
    // 0. Zeige zuerst den Skeleton-Loader
    switchState('loading');

    try {
      // 1. Fetching von der neu gebauten Brücke (Service)
      const data = await RewardsDataService.getRewardsData();

      // Edge Case: Empty State anzeigen, wenn Balance 0 ist UND Tier unreached
      if (data.balance.totalAvailable === 0 && data.tier.investedLast12Months === 0 && !data.referral.referralLink) {
        switchState('empty');
      } else {
        // 2. Direktes Binding an das UI (entfernt alle Fallbacks im HTML sofort)
        renderRewardsUI(data);
        switchState('content');

        // 3. Fetch and render campaign breakdown table and chart
        try {
          const campaigns = await RewardsDataService.getCampaignData();
          allCampaigns = campaigns;
          currentCurrency = data.balance.currency;
          renderCampaignTable();
          bindCampaignTableEvents();
          
          if (typeof Chart !== 'undefined') {
            renderCampaignChart();
          }
        } catch (e) {
          console.warn("Campaign data load failed:", e);
        }
      }
    } catch (e) {
      console.error("Failed to render rewards UI:", e);
      // Fallback: Error UI
      switchState('error');
    }

    // Events sauber binden
    const copyBtns = document.querySelectorAll("#rewards-copy-btn, .copy-icon-btn, .copy-link-btn");
    copyBtns.forEach((btn) => {
      btn.addEventListener("click", function (e) {
        e.preventDefault();
        // check if this is the campaign copy btn
        if (this.id === 'campaign-copy-btn' || this.id === 'campaign-copy-icon') {
          copyReferralLink('campaign-generated-link');
        } else {
          copyReferralLink('rewards-referral-input');
        }
      });
    });

    // SubID Generator Logic
    const subidInput = document.getElementById('campaign-subid-input');
    const generatedLinkInput = document.getElementById('campaign-generated-link');
    
    if (subidInput && generatedLinkInput) {
      subidInput.addEventListener('input', function(e) {
        let val = e.target.value.trim();
        let base = generatedLinkInput.dataset.baseLink || '';
        if (val) {
          // clean the value somewhat
          val = encodeURIComponent(val.replace(/[^a-zA-Z0-9_-]/g, ''));
          const separator = base.includes('?') ? '&' : '?';
          generatedLinkInput.value = `${base}${separator}subid=${val}`;
        } else {
          generatedLinkInput.value = base;
        }
      });
    }
  }

  // Bind new events for search, sort and export
  function bindCampaignTableEvents() {
    const searchInput = document.getElementById('campaign-search-input');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        currentSearch = e.target.value.trim();
        renderCampaignTable();
      });
    }

    const headers = document.querySelectorAll('#campaign-table th.sortable');
    headers.forEach(th => {
      th.addEventListener('click', () => {
        const field = th.dataset.sort;
        if (currentSort === field) {
          sortAsc = !sortAsc;
        } else {
          currentSort = field;
          sortAsc = false; // default to desc on new column
        }
        
        // update UI headers
        headers.forEach(h => {
          const icon = h.querySelector('.sort-icon');
          if (icon) icon.textContent = '';
        });
        const activeIcon = th.querySelector('.sort-icon');
        if (activeIcon) activeIcon.textContent = sortAsc ? '↑' : '↓';
        
        renderCampaignTable();
      });
    });

    const exportBtn = document.getElementById('campaign-export-btn');
    if (exportBtn) {
      exportBtn.addEventListener('click', () => {
        exportCampaignsCSV();
      });
    }
  }

  // Export to CSV
  function exportCampaignsCSV() {
    if (!allCampaigns || allCampaigns.length === 0) {
      showToast("No data to export", "error");
      return;
    }
    
    // Use filtered data for export
    let dataToExport = getFilteredAndSortedCampaigns();
    
    let csvContent = "data:text/csv;charset=utf-8,";
    csvContent += "Campaign,Clicks,Signups,CVR (%),Qualified,Revenue (" + currentCurrency + ")\n";

    dataToExport.forEach(c => {
      const cvr = c.clicks > 0 ? ((c.signups / c.clicks) * 100).toFixed(2) : "0.00";
      const revenue = (c.revenue_cents / 100).toFixed(2);
      // Escape subid if it contains commas
      let subid = c.subid;
      if (subid.includes(",")) subid = `"${subid}"`;
      
      const row = `${subid},${c.clicks},${c.signups},${cvr},${c.qualified},${revenue}`;
      csvContent += row + "\n";
    });

    const encodedUri = encodeURI(csvContent);
    const link = document.createElement("a");
    link.setAttribute("href", encodedUri);
    link.setAttribute("download", "campaign_performance.csv");
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }

  function getFilteredAndSortedCampaigns() {
    let filtered = allCampaigns;
    if (currentSearch) {
      const lowerSearch = currentSearch.toLowerCase();
      filtered = allCampaigns.filter(c => c.subid.toLowerCase().includes(lowerSearch));
    }

    // clone to avoid mutating original if needed, though sort() mutates array
    let sorted = [...filtered];
    sorted.sort((a, b) => {
      let valA = a[currentSort];
      let valB = b[currentSort];
      
      if (currentSort === 'subid') {
        valA = valA.toLowerCase();
        valB = valB.toLowerCase();
        if (valA < valB) return sortAsc ? -1 : 1;
        if (valA > valB) return sortAsc ? 1 : -1;
        return 0;
      } else {
        return sortAsc ? valA - valB : valB - valA;
      }
    });
    return sorted;
  }

  // Campaign Breakdown Table Renderer
  function renderCampaignTable() {
    const tbody = document.getElementById("campaign-table-body");
    if (!tbody) return;

    if (!allCampaigns || allCampaigns.length === 0) return; // keep the empty state row

    let displayData = getFilteredAndSortedCampaigns();

    tbody.innerHTML = '';
    
    if (displayData.length === 0) {
      tbody.innerHTML = `<tr><td colspan="6" style="text-align:center; padding:32px 0; color:#98a2b3;">No campaigns match your search.</td></tr>`;
      return;
    }

    displayData.forEach(c => {
      const isDirect = c.subid === '(direct)';
      const badgeClass = isDirect ? 'campaign-subid-badge direct' : 'campaign-subid-badge';
      const cvrClass = c.cvr >= 5 ? 'campaign-cvr' : 'campaign-cvr low';

      const row = document.createElement('tr');
      row.innerHTML = `
        <td><span class="${badgeClass}">${escapeHtml(c.subid)}</span></td>
        <td>${c.clicks.toLocaleString()}</td>
        <td>${c.signups.toLocaleString()}</td>
        <td><span class="${cvrClass}">${c.cvr.toFixed(1)}%</span></td>
        <td>${c.qualified.toLocaleString()}</td>
        <td>${formatCurrency(c.revenue_cents, currentCurrency)}</td>
      `;
      tbody.appendChild(row);
    });
  }

  // Active chart range (in days) — default 30
  let chartRangeDays = 30;

  // Generate date labels for last N days (or custom range)
  function generateDateLabels(days) {
    const labels = [];
    const startDate = customDateFrom ? new Date(customDateFrom) : (() => { const d = new Date(); d.setDate(d.getDate() - (days - 1)); return d; })();
    for (let i = 0; i < days; i++) {
      const d = new Date(startDate);
      d.setDate(d.getDate() + i);
      const month = d.toLocaleString('en-US', { month: 'short' });
      const day = d.getDate();
      labels.push(`${month} ${day}`);
    }
    return labels;
  }

  // Generate simulated daily data from campaign totals spread across the range
  function generateDailyData(total, days) {
    const data = new Array(days).fill(0);
    if (total === 0) return data;
    // Distribute somewhat randomly for realistic look
    for (let i = 0; i < total; i++) {
      // Weight towards more recent days
      const idx = Math.floor(Math.pow(Math.random(), 0.7) * days);
      data[days - 1 - idx] = (data[days - 1 - idx] || 0) + 1;
    }
    return data;
  }

  // Analytics Chart Renderer
  function renderCampaignChart() {
    const canvas = document.getElementById('campaignChart');
    if (!canvas) return;

    const days = chartRangeDays;
    const labels = generateDateLabels(days);

    // Calculate totals from campaign data
    let totalClicks = 0;
    let totalSignups = 0;
    if (allCampaigns && allCampaigns.length > 0) {
      allCampaigns.forEach(c => {
        totalClicks += c.clicks || 0;
        totalSignups += c.signups || 0;
      });
    }

    const clicksData = generateDailyData(totalClicks, days);
    const signupsData = generateDailyData(totalSignups, days);

    // Calculate smart Y-axis max
    const maxVal = Math.max(...clicksData, ...signupsData, 1);
    const yMax = Math.max(10, Math.ceil(maxVal / 5) * 5); // round up to nearest 5, min 10

    // Determine tick spacing for X-axis based on range
    let maxTicksLimit;
    if (days <= 7) maxTicksLimit = 7;
    else if (days <= 30) maxTicksLimit = 10;
    else if (days <= 90) maxTicksLimit = 12;
    else maxTicksLimit = 12;

    if (campaignChartInstance) {
      campaignChartInstance.destroy();
    }

    const ctx = canvas.getContext('2d');
    campaignChartInstance = new Chart(ctx, {
      type: 'bar',
      data: {
        labels: labels,
        datasets: [
          {
            label: 'Clicks',
            data: clicksData,
            backgroundColor: 'rgba(0, 0, 255, 0.7)',
            hoverBackgroundColor: 'rgba(0, 0, 255, 0.9)',
            borderRadius: 6,
            borderSkipped: false,
          },
          {
            label: 'Signups',
            data: signupsData,
            backgroundColor: 'rgba(3, 255, 136, 0.7)',
            hoverBackgroundColor: 'rgba(3, 255, 136, 0.9)',
            borderRadius: 6,
            borderSkipped: false,
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            position: 'top',
            align: 'center',
            labels: {
              usePointStyle: true,
              pointStyle: 'circle',
              padding: 20,
              font: { family: "'TT Norms Pro', sans-serif", size: 13, weight: '500' },
              color: '#475467',
              boxWidth: 8,
              boxHeight: 8,
            }
          },
          tooltip: {
            mode: 'index',
            intersect: false,
            backgroundColor: '#101828',
            titleFont: { family: "'TT Norms Pro', sans-serif", size: 13 },
            bodyFont: { family: "'TT Norms Pro', sans-serif", size: 12 },
            padding: 12,
            cornerRadius: 8,
            boxPadding: 4,
          }
        },
        scales: {
          y: {
            beginAtZero: true,
            min: 0,
            max: yMax,
            ticks: {
              stepSize: yMax <= 10 ? 1 : Math.ceil(yMax / 10),
              font: { family: "'TT Norms Pro', sans-serif", size: 12 },
              color: '#98a2b3',
              padding: 8,
            },
            grid: {
              color: '#f2f4f7',
              drawBorder: false,
            },
            border: {
              display: false,
            }
          },
          x: {
            grid: {
              display: false,
              drawBorder: false,
            },
            border: {
              display: false,
            },
            ticks: {
              maxTicksLimit: maxTicksLimit,
              maxRotation: 0,
              font: { family: "'TT Norms Pro', sans-serif", size: 11 },
              color: '#98a2b3',
              padding: 8,
            }
          }
        },
        layout: {
          padding: { top: 4, bottom: 4 }
        }
      }
    });
  }

  // Global: switch chart range (called from onclick)
  window.switchChartRange = function(btn) {
    // Toggle active class
    document.querySelectorAll('.chart-filter-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');

    // Close custom date picker if open
    const picker = document.getElementById('chart-date-picker');
    if (picker) picker.classList.remove('open');

    // Reset custom label
    const label = document.getElementById('chart-custom-label');
    if (label) label.textContent = 'Custom';

    // Update range and re-render
    chartRangeDays = parseInt(btn.dataset.range, 10);
    customDateFrom = null;
    customDateTo = null;
    if (typeof Chart !== 'undefined') {
      renderCampaignChart();
    }
  };

  // Custom date range state
  let customDateFrom = null;
  let customDateTo = null;

  // Toggle custom date picker dropdown
  window.toggleCustomDatePicker = function(btn) {
    const picker = document.getElementById('chart-date-picker');
    if (!picker) return;

    const isOpen = picker.classList.contains('open');
    if (isOpen) {
      picker.classList.remove('open');
    } else {
      // Set defaults: from = 30 days ago, to = today
      const fromInput = document.getElementById('chart-date-from');
      const toInput = document.getElementById('chart-date-to');
      const today = new Date();
      const thirtyAgo = new Date();
      thirtyAgo.setDate(today.getDate() - 30);

      if (fromInput && !fromInput.value) fromInput.value = thirtyAgo.toISOString().split('T')[0];
      if (toInput && !toInput.value) toInput.value = today.toISOString().split('T')[0];

      picker.classList.add('open');
    }
  };

  // Apply custom date range
  window.applyCustomDateRange = function() {
    const fromInput = document.getElementById('chart-date-from');
    const toInput = document.getElementById('chart-date-to');
    const picker = document.getElementById('chart-date-picker');

    if (!fromInput || !toInput) return;

    const from = new Date(fromInput.value);
    const to = new Date(toInput.value);

    if (isNaN(from.getTime()) || isNaN(to.getTime())) {
      showToast('Please select valid dates', 'error');
      return;
    }

    if (from > to) {
      showToast('"From" date must be before "To" date', 'error');
      return;
    }

    // Calculate days between
    const diffTime = to.getTime() - from.getTime();
    const days = Math.max(1, Math.ceil(diffTime / (1000 * 60 * 60 * 24)) + 1);

    customDateFrom = from;
    customDateTo = to;
    chartRangeDays = days;

    // Update button states
    document.querySelectorAll('.chart-filter-btn').forEach(b => b.classList.remove('active'));
    const customBtn = document.getElementById('chart-custom-btn');
    if (customBtn) customBtn.classList.add('active');

    // Update label to show selected range
    const label = document.getElementById('chart-custom-label');
    if (label) {
      const fmtFrom = from.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      const fmtTo = to.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      label.textContent = `${fmtFrom} – ${fmtTo}`;
    }

    // Close dropdown
    if (picker) picker.classList.remove('open');

    // Re-render chart
    if (typeof Chart !== 'undefined') {
      renderCampaignChart();
    }
  };

  // Close custom date picker when clicking outside
  document.addEventListener('click', function(e) {
    const wrapper = document.querySelector('.chart-custom-date-wrapper');
    const picker = document.getElementById('chart-date-picker');
    if (wrapper && picker && !wrapper.contains(e.target)) {
      picker.classList.remove('open');
    }
  });

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
  }

  // Social Sharing Logic (Exposed to window)
  window.shareSocial = function(platform) {
    const linkInput = document.getElementById('campaign-generated-link') || document.getElementById('rewards-referral-input');
    if (!linkInput || !linkInput.value || linkInput.value === 'Loading...') {
      showToast('Please wait or try again, your link is not ready.', 'error');
      return;
    }
    
    const url = encodeURIComponent(linkInput.value);
    const text = encodeURIComponent("I'm investing in properties globally with POOOL. Sign up with my link and get USD 30!");
    
    let shareUrl = '';
    
    switch(platform) {
      case 'whatsapp':
        shareUrl = `https://api.whatsapp.com/send?text=${text}%20${url}`;
        break;
      case 'twitter':
        shareUrl = `https://twitter.com/intent/tweet?text=${text}&url=${url}`;
        break;
      case 'linkedin':
        // LinkedIn prefers you just share the url directly
        shareUrl = `https://www.linkedin.com/sharing/share-offsite/?url=${url}`;
        break;
      case 'email':
        shareUrl = `mailto:?subject=${encodeURIComponent("Join me on POOOL")}&body=${text}%20${url}`;
        break;
      default:
        return;
    }
    
    if (platform === 'email') {
      window.location.href = shareUrl;
    } else {
      window.open(shareUrl, '_blank', 'width=600,height=400');
    }
  };

  // Marketing Tab Helpers (Exposed to window)
  window.copyEmailTemplate = function(elementId) {
    const el = document.getElementById(elementId);
    if (!el) return;
    const text = el.textContent || el.innerText;
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text)
        .then(() => showToast("Email template copied!", "success"))
        .catch(() => showToast("Failed to copy template", "error"));
    } else {
      // Fallback
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.left = "-9999px";
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand("copy");
        showToast("Email template copied!", "success");
      } catch (e) {
        showToast("Failed to copy template", "error");
      }
      document.body.removeChild(ta);
    }
  };

  window.showMarketingToast = function(message) {
    showToast(message, "success");
  };

  // ============================================================
  // COMMISSIONS TAB LOGIC
  // ============================================================

  let commissionsTabLoaded = false;

  // Called when Commissions tab is activated
  window.loadCommissionsTab = async function() {
    if (commissionsTabLoaded) return;
    commissionsTabLoaded = true;

    // Set default date range (last 12 months)
    const today = new Date();
    const oneYearAgo = new Date();
    oneYearAgo.setFullYear(today.getFullYear() - 1);

    const fromInput = document.getElementById('commissions-date-from');
    const toInput = document.getElementById('commissions-date-to');
    if (fromInput) fromInput.value = oneYearAgo.toISOString().split('T')[0];
    if (toInput) toInput.value = today.toISOString().split('T')[0];

    // Listen for date changes
    if (fromInput) fromInput.addEventListener('change', loadCommissions);
    if (toInput) toInput.addEventListener('change', loadCommissions);

    // Load both sections
    await Promise.all([loadPayoutSettings(), loadCommissions()]);
  };

  async function loadPayoutSettings() {
    try {
      const resp = await fetch('/api/rewards/payout-settings', { credentials: 'include' });
      if (!resp.ok) return;
      const data = await resp.json();
      const ps = data.payout_settings;
      if (!ps) return;

      const setVal = (id, val) => { 
        const el = document.getElementById(id); 
        if (el) {
          el.value = val || ''; 
          if (el.tagName === 'SELECT') {
            const wrapper = el.closest('.poool-dropdown');
            if (wrapper && wrapper._pooolDropdown) {
              wrapper._pooolDropdown.setValue(val || '');
            }
          }
        }
      };
      setVal('payout-payment-method', ps.payment_method);
      setVal('payout-account-email', ps.account_email);
      setVal('payout-full-name', ps.full_name);
      setVal('payout-vat', ps.vat_number);
    } catch (e) {
      console.warn('Failed to load payout settings:', e);
    }
  }

  window.savePayoutSettings = async function(e) {
    e.preventDefault();
    const btn = document.getElementById('payout-save-btn');
    if (btn) { btn.disabled = true; btn.textContent = 'Saving…'; }

    const body = {
      payment_method: document.getElementById('payout-payment-method')?.value || 'paypal',
      account_email: document.getElementById('payout-account-email')?.value || null,
      full_name: document.getElementById('payout-full-name')?.value || null,
      vat_number: document.getElementById('payout-vat')?.value || null,
    };

    try {
      const resp = await fetch('/api/rewards/payout-settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify(body),
      });
      if (resp.ok) {
        showToast('Payout settings saved successfully!', 'success');
      } else {
        showToast('Failed to save payout settings.', 'error');
      }
    } catch (e) {
      showToast('Network error — please try again.', 'error');
    } finally {
      if (btn) { btn.disabled = false; btn.textContent = 'SAVE'; }
    }
  };

  async function loadCommissions() {
    const tbody = document.getElementById('commissions-table-body');
    if (!tbody) return;

    const fromVal = document.getElementById('commissions-date-from')?.value || '';
    const toVal = document.getElementById('commissions-date-to')?.value || '';
    const params = new URLSearchParams();
    if (fromVal) params.set('from', fromVal);
    if (toVal) params.set('to', toVal);

    tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:32px 0; color:#98a2b3;">Loading commissions…</td></tr>';

    try {
      const resp = await fetch(`/api/rewards/commissions?${params.toString()}`, { credentials: 'include' });
      if (!resp.ok) throw new Error('API error');
      const data = await resp.json();
      const commissions = data.commissions || [];

      if (commissions.length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:32px 0; color:#98a2b3;">No commissions found for this period.</td></tr>';
        return;
      }

      tbody.innerHTML = '';
      commissions.forEach(c => {
        const periodStart = new Date(c.period_start).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
        const periodEnd = new Date(c.period_end).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
        const amount = (c.amount_cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
        const statusLabel = c.status.charAt(0).toUpperCase() + c.status.slice(1);
        const statusClass = `commission-status-dot--${c.status}`;

        const row = document.createElement('tr');
        row.innerHTML = `
          <td>${periodStart} - ${periodEnd}</td>
          <td style="text-align:right">${amount}</td>
          <td>${c.payment_method}</td>
          <td>
            <div class="commission-status">
              <span class="commission-status-dot ${statusClass}"></span>
              ${escapeHtml(statusLabel)}
            </div>
          </td>
          <td style="text-align:right">
            <button class="commission-export-btn" onclick="exportCommissionPdf('${c.id}')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="7 10 12 15 17 10"></polyline>
                <line x1="12" y1="15" x2="12" y2="3"></line>
              </svg>
              Export to PDF
            </button>
          </td>
        `;
        tbody.appendChild(row);
      });
    } catch (e) {
      console.error('Failed to load commissions:', e);
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:32px 0; color:#F04438;">Failed to load commissions. Please try again.</td></tr>';
    }
  }

  window.exportCommissionPdf = function(commissionId) {
    showToast('PDF export will be available soon.', 'success');
  };

  document.addEventListener("DOMContentLoaded", initRewardsPage);
})();

