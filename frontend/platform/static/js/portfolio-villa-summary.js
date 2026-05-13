/**
 * Portfolio Villa Summary cards — Villa-Returns P3.
 * Calls GET /api/investors/me/portfolio-villa-summary and renders 4 KPI cards
 * on the authenticated portfolio page.
 *
 * Values are USD cents from the existing wallet/investments pipeline.
 * No per-asset breakdown here — that's a separate slice on my-trading.html.
 */

(function () {
  document.addEventListener("DOMContentLoaded", () => {
    hydrate();
  });

  async function hydrate() {
    const meta = document.getElementById("vs-meta");
    try {
      const r = await fetch("/api/investors/me/portfolio-villa-summary");
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const s = await r.json();
      render(s);
    } catch (err) {
      if (meta) meta.textContent = `Failed: ${err.message}`;
    }
  }

  function render(s) {
    setText("vs-positions", String(s.active_position_count));
    setText("vs-invested", fmtUsd(s.total_invested_cents));
    setText("vs-current", fmtUsd(s.current_value_cents));
    setText("vs-dividends", fmtUsd(s.lifetime_dividends_usd_cents));

    const pnlEl = document.getElementById("vs-pnl");
    if (pnlEl) {
      const pnl = Number(s.unrealised_pnl_cents);
      const pct = s.total_invested_cents > 0
        ? (pnl * 10000) / s.total_invested_cents / 100
        : 0;
      if (pnl > 0) {
        pnlEl.style.color = "#059669";
        pnlEl.textContent = `+${fmtUsd(pnl)} (${pct.toFixed(2)} %)`;
      } else if (pnl < 0) {
        pnlEl.style.color = "#dc2626";
        pnlEl.textContent = `${fmtUsd(pnl)} (${pct.toFixed(2)} %)`;
      } else {
        pnlEl.style.color = "#6b7280";
        pnlEl.textContent = "Flat";
      }
    }
    const dcEl = document.getElementById("vs-dividend-count");
    if (dcEl) {
      dcEl.textContent = `${s.lifetime_dividend_count} payout${s.lifetime_dividend_count === 1 ? "" : "s"}`;
    }
    const meta = document.getElementById("vs-meta");
    if (meta) {
      meta.textContent =
        s.active_position_count === 0 ? "No active positions yet" : "Across active positions";
    }
  }

  function fmtUsd(cents) {
    const sign = cents < 0 ? "-" : "";
    const abs = Math.abs(Number(cents)) / 100;
    return `${sign}USD ${abs.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  }
})();
