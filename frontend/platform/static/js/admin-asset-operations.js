/**
 * Admin Asset Operations — Villa-Returns P2.4.
 * Lives on admin/asset-details.html; hydrates the "Operations" tab.
 *
 * Two sections:
 *   - Developer access (list / grant / revoke via /api/admin/villas/:id/developer-access)
 *   - Monthly operations strip (last 12 months via /api/admin/villas/:id/operations)
 *
 * Loads on tab activation (lazy). The asset id is read from ?id= query param
 * to match the existing admin-asset-details.js convention.
 */

(function () {
  let assetId = null;
  let loaded = false;

  document.addEventListener("DOMContentLoaded", () => {
    const params = new URLSearchParams(window.location.search);
    assetId = params.get("id");
    if (!assetId) return;

    // Tab switching is already wired in admin-asset-details.js; we just
    // observe clicks and hydrate when the operations tab activates.
    const tabs = document.getElementById("asset-tabs");
    if (tabs) {
      tabs.addEventListener("click", (e) => {
        const tab = e.target.closest(".asset-tab");
        if (tab?.dataset.tab === "operations" && !loaded) {
          hydrate();
          loaded = true;
        }
      });
    }

    document.getElementById("btn-grant-dev-access")?.addEventListener("click", openGrantModal);
    document.getElementById("grant-dev-cancel")?.addEventListener("click", closeGrantModal);
    document.getElementById("grant-dev-confirm")?.addEventListener("click", confirmGrant);
    document.getElementById("btn-enter-month")?.addEventListener("click", openLatestMonthEntry);
    document.getElementById("btn-new-valuation")?.addEventListener("click", openNewValuation);
    document.getElementById("btn-save-villa-config")?.addEventListener("click", saveVillaConfig);
    document.getElementById("btn-add-forecast-year")?.addEventListener("click", addForecastYear);
  });

  async function hydrate() {
    await Promise.all([
      loadDeveloperAccess(),
      loadOperationsStrip(),
      loadValuations(),
      loadPendingCapex(),
      loadPendingForecasts(),
      loadForecastAssumptions(),
      loadVillaConfig(),
    ]);
  }

  // ─── Developer access ────────────────────────────────────────

  async function loadDeveloperAccess() {
    const tbody = document.getElementById("dev-access-tbody");
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/developer-access`);
      if (!resp.ok) throw new Error(await responseError(resp));
      const rows = await resp.json();
      renderDeveloperAccess(rows);
    } catch (err) {
      tbody.innerHTML = `<tr><td colspan="4" style="padding: 16px; color: var(--admin-danger, #dc2626);">Failed to load: ${escapeHtml(err.message)}</td></tr>`;
    }
  }

  function renderDeveloperAccess(rows) {
    const tbody = document.getElementById("dev-access-tbody");
    if (!rows.length) {
      tbody.innerHTML = `<tr><td colspan="4" style="padding: 24px; text-align: center; color: var(--admin-text-muted, #6b7280);">No developers linked yet.</td></tr>`;
      return;
    }
    tbody.innerHTML = "";
    for (const r of rows) {
      const active = !r.effective_until;
      const tr = document.createElement("tr");
      tr.style.borderTop = "1px solid var(--admin-border, #e5e7eb)";

      const tdUser = document.createElement("td");
      tdUser.style.padding = "10px 12px";
      tdUser.innerHTML = `<div>${escapeHtml(r.developer_email || "(no email)")}</div><div style="font-size: 11px; color: var(--admin-text-muted, #6b7280);">${escapeHtml(r.developer_user_id)}</div>`;
      tr.appendChild(tdUser);

      const tdFrom = document.createElement("td");
      tdFrom.style.padding = "10px 12px";
      tdFrom.textContent = formatDate(r.effective_from);
      tr.appendChild(tdFrom);

      const tdStatus = document.createElement("td");
      tdStatus.style.padding = "10px 12px";
      tdStatus.innerHTML = active
        ? `<span style="padding: 3px 10px; border-radius: 10px; font-size: 11px; background: var(--admin-success-bg, #ecfdf5); color: var(--admin-success, #059669);">Active</span>`
        : `<span style="padding: 3px 10px; border-radius: 10px; font-size: 11px; background: var(--admin-neutral-bg, #f3f4f6); color: var(--admin-text-muted, #6b7280);">Revoked ${formatDate(r.effective_until)}</span>`;
      tr.appendChild(tdStatus);

      const tdAction = document.createElement("td");
      tdAction.style.padding = "10px 12px";
      tdAction.style.textAlign = "right";
      if (active) {
        const btn = document.createElement("button");
        btn.className = "admin-btn admin-btn--secondary";
        btn.type = "button";
        btn.textContent = "Revoke";
        btn.style.color = "var(--admin-danger, #dc2626)";
        btn.style.borderColor = "var(--admin-danger, #dc2626)";
        btn.addEventListener("click", () => revokeAccess(r.id, r.developer_email));
        tdAction.appendChild(btn);
      }
      tr.appendChild(tdAction);

      tbody.appendChild(tr);
    }
  }

  function openGrantModal() {
    const m = document.getElementById("grant-dev-modal");
    m.style.display = "flex";
    document.getElementById("grant-dev-uuid").value = "";
    document.getElementById("grant-dev-notes").value = "";
    document.getElementById("grant-dev-error").textContent = "";
  }
  function closeGrantModal() {
    document.getElementById("grant-dev-modal").style.display = "none";
  }

  async function confirmGrant() {
    const uuid = document.getElementById("grant-dev-uuid").value.trim();
    const notes = document.getElementById("grant-dev-notes").value.trim();
    const errEl = document.getElementById("grant-dev-error");
    errEl.textContent = "";
    if (!/^[0-9a-f-]{36}$/i.test(uuid)) {
      errEl.textContent = "Please enter a valid UUID.";
      return;
    }
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/developer-access`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ developer_user_id: uuid, notes: notes || null }),
      });
      if (!resp.ok) throw new Error(await responseError(resp));
      closeGrantModal();
      await loadDeveloperAccess();
    } catch (err) {
      errEl.textContent = err.message;
    }
  }

  async function revokeAccess(linkId, email) {
    if (!confirm(`Revoke access for ${email || linkId}? Append-only — this only sets effective_until.`)) return;
    try {
      const resp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/developer-access/${linkId}`,
        { method: "DELETE", headers: csrfHeaders() }
      );
      if (!resp.ok) throw new Error(await responseError(resp));
      await loadDeveloperAccess();
    } catch (err) {
      alert(`Revoke failed: ${err.message}`);
    }
  }

  // ─── Monthly operations strip ────────────────────────────────

  async function loadOperationsStrip() {
    const tbody = document.getElementById("ops-strip-tbody");
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/operations`);
      if (!resp.ok) throw new Error(await responseError(resp));
      const rows = await resp.json();
      renderOperationsStrip(rows);
    } catch (err) {
      tbody.innerHTML = `<tr><td colspan="5" style="padding: 16px; color: var(--admin-danger, #dc2626);">Failed to load: ${escapeHtml(err.message)}</td></tr>`;
    }
  }

  function renderOperationsStrip(rows) {
    const tbody = document.getElementById("ops-strip-tbody");
    // Latest non-superseded row per (year, month). Rows already sorted DESC by recorded_at.
    const latest = new Map();
    for (const r of rows) {
      const k = `${r.period_year}-${String(r.period_month).padStart(2, "0")}`;
      if (!latest.has(k)) latest.set(k, r);
    }
    const months = Array.from(latest.values()).slice(0, 12);
    const badge = document.getElementById("badge-operations");
    if (badge) badge.textContent = String(months.length);

    if (!months.length) {
      tbody.innerHTML = `<tr><td colspan="5" style="padding: 24px; text-align: center; color: var(--admin-text-muted, #6b7280);">No operations data yet.</td></tr>`;
      return;
    }
    tbody.innerHTML = "";
    for (const r of months) {
      const tr = document.createElement("tr");
      tr.style.borderTop = "1px solid var(--admin-border, #e5e7eb)";

      const tdPeriod = document.createElement("td");
      tdPeriod.style.padding = "10px 12px";
      tdPeriod.textContent = `${r.period_year}-${String(r.period_month).padStart(2, "0")}`;
      tr.appendChild(tdPeriod);

      const tdDist = document.createElement("td");
      tdDist.style.padding = "10px 12px";
      tdDist.style.textAlign = "right";
      tdDist.style.fontVariantNumeric = "tabular-nums";
      tdDist.textContent = Number(r.distributable_idr_cents).toLocaleString();
      tr.appendChild(tdDist);

      const tdOcc = document.createElement("td");
      tdOcc.style.padding = "10px 12px";
      tdOcc.style.textAlign = "right";
      tdOcc.textContent = `${(Number(r.occupancy_bps) / 100).toFixed(2)} %`;
      tr.appendChild(tdOcc);

      const tdStatus = document.createElement("td");
      tdStatus.style.padding = "10px 12px";
      tdStatus.innerHTML = statusBadge(r.status);
      tr.appendChild(tdStatus);

      const tdAction = document.createElement("td");
      tdAction.style.padding = "10px 12px";
      tdAction.style.textAlign = "right";
      tdAction.style.whiteSpace = "nowrap";

      const a = document.createElement("a");
      a.className = "admin-btn admin-btn--secondary";
      a.href = `/admin/villas/${encodeURIComponent(assetId)}/operations/${r.period_year}/${r.period_month}?log_id=${r.id}`;
      a.textContent = r.status === "published" ? "Correct" : "Open";
      a.style.marginRight = "6px";
      tdAction.appendChild(a);

      if (r.status === "published") {
        const dbtn = document.createElement("button");
        dbtn.type = "button";
        dbtn.className = "admin-btn admin-btn--primary";
        dbtn.textContent = "Distribute";
        dbtn.addEventListener("click", () => distribute(r));
        tdAction.appendChild(dbtn);

        // Q11 top-up: only show for corrected (supersedes_id != NULL) published rows.
        if (r.supersedes_id) {
          const tbtn = document.createElement("button");
          tbtn.type = "button";
          tbtn.className = "admin-btn admin-btn--secondary";
          tbtn.style.marginLeft = "6px";
          tbtn.textContent = "Top up";
          tbtn.title = "Pay each investor the positive delta vs what they already received for this period.";
          tbtn.addEventListener("click", () => topUp(r));
          tdAction.appendChild(tbtn);
        }
      }
      tr.appendChild(tdAction);

      tbody.appendChild(tr);
    }
  }

  function statusBadge(status) {
    const styles = {
      draft:      "background: var(--admin-neutral-bg, #f3f4f6); color: var(--admin-text-muted, #6b7280);",
      submitted:  "background: var(--admin-info-bg, #eff6ff); color: var(--admin-info, #2563eb);",
      approved:   "background: var(--admin-success-bg, #ecfdf5); color: var(--admin-success, #059669);",
      published:  "background: var(--admin-success-bg, #ecfdf5); color: var(--admin-success, #059669); font-weight: 600;",
      superseded: "background: var(--admin-warn-bg, #fffbeb); color: var(--admin-warn, #d97706);",
    };
    const s = styles[status] || styles.draft;
    return `<span style="padding: 3px 10px; border-radius: 10px; font-size: 11px; ${s}">${escapeHtml(status)}</span>`;
  }

  // ─── Pending CapEx ──────────────────────────────────────────

  async function loadPendingCapex() {
    const tbody = document.getElementById("capex-pending-tbody");
    if (!tbody) return;
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/capex`);
      if (!r.ok) throw new Error(await responseError(r));
      const rows = (await r.json()).filter((x) => x.status === "submitted");
      if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="5" style="padding: 12px; text-align: center; color: var(--admin-text-muted, #6b7280);">No CapEx events pending.</td></tr>`;
        return;
      }
      tbody.innerHTML = "";
      const me = await currentUserId();
      for (const r of rows) {
        const isSelfSubmitted = me && r.submitted_by === me;
        const tr = document.createElement("tr");
        tr.style.borderTop = "1px solid var(--admin-border, #e5e7eb)";
        tr.innerHTML = `
          <td style="padding: 8px 12px;">${escapeHtml(r.event_date)}</td>
          <td style="padding: 8px 12px;">${escapeHtml(r.description)}<div style="font-size: 11px; color: var(--admin-text-muted, #6b7280);">${escapeHtml((r.category || "").replace(/_/g, " "))}</div></td>
          <td style="padding: 8px 12px; text-align: right; font-variant-numeric: tabular-nums;">${Number(r.amount_idr_cents).toLocaleString()}</td>
          <td style="padding: 8px 12px;">${statusBadge(r.status)}${isSelfSubmitted ? '<div style="font-size: 10px; color: var(--admin-text-muted, #6b7280); margin-top: 2px;">You submitted</div>' : ""}</td>
        `;
        const tdAction = document.createElement("td");
        tdAction.style.padding = "8px 12px";
        tdAction.style.textAlign = "right";
        const approveBtn = document.createElement("button");
        approveBtn.type = "button";
        approveBtn.className = "admin-btn admin-btn--primary";
        approveBtn.textContent = "Approve";
        approveBtn.disabled = !!isSelfSubmitted;
        approveBtn.style.marginRight = "6px";
        approveBtn.addEventListener("click", () => capexAction(r.id, "approve"));
        tdAction.appendChild(approveBtn);
        const rejectBtn = document.createElement("button");
        rejectBtn.type = "button";
        rejectBtn.className = "admin-btn admin-btn--secondary";
        rejectBtn.style.color = "var(--admin-danger, #dc2626)";
        rejectBtn.style.borderColor = "var(--admin-danger, #dc2626)";
        rejectBtn.textContent = "Reject";
        rejectBtn.addEventListener("click", () => capexAction(r.id, "reject"));
        tdAction.appendChild(rejectBtn);
        tr.appendChild(tdAction);
        tbody.appendChild(tr);
      }
    } catch (err) {
      tbody.innerHTML = `<tr><td colspan="5" style="padding: 12px; color: var(--admin-danger, #dc2626);">Failed: ${escapeHtml(err.message)}</td></tr>`;
    }
  }

  async function capexAction(id, action) {
    let body = null;
    let headers = csrfHeaders();
    if (action === "reject") {
      const reason = prompt("Rejection reason (required):");
      if (!reason || !reason.trim()) return;
      headers = csrfHeaders({ "Content-Type": "application/json" });
      body = JSON.stringify({ reason });
    }
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/capex/${id}/${action}`, {
        method: "PUT",
        headers,
        body,
      });
      if (!r.ok) throw new Error(await responseError(r));
      await loadPendingCapex();
    } catch (err) {
      alert(`CapEx ${action} failed: ${err.message}`);
    }
  }

  // ─── Pending forecast suggestions ───────────────────────────

  async function loadPendingForecasts() {
    const tbody = document.getElementById("forecast-pending-tbody");
    if (!tbody) return;
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/forecast-suggestions`);
      if (!r.ok) throw new Error(await responseError(r));
      const rows = (await r.json()).filter((x) => x.status === "submitted");
      if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="4" style="padding: 12px; text-align: center; color: var(--admin-text-muted, #6b7280);">No forecast suggestions pending.</td></tr>`;
        return;
      }
      tbody.innerHTML = "";
      for (const r of rows) {
        const tr = document.createElement("tr");
        tr.style.borderTop = "1px solid var(--admin-border, #e5e7eb)";
        const occ = r.projected_occupancy_bps != null ? `${(r.projected_occupancy_bps / 100).toFixed(2)}% occupancy` : null;
        const adr = r.projected_adr_idr_cents != null ? `${Number(r.projected_adr_idr_cents).toLocaleString()} IDR ADR` : null;
        const meta = [occ, adr].filter(Boolean).join(" · ");
        tr.innerHTML = `
          <td style="padding: 8px 12px;">${escapeHtml(r.forecast_year)}</td>
          <td style="padding: 8px 12px;">${escapeHtml(r.notes || "(no notes)")}<div style="font-size: 11px; color: var(--admin-text-muted, #6b7280);">${escapeHtml(meta)}</div></td>
          <td style="padding: 8px 12px;">${statusBadge(r.status)}</td>
        `;
        const tdAction = document.createElement("td");
        tdAction.style.padding = "8px 12px";
        tdAction.style.textAlign = "right";
        const acceptBtn = document.createElement("button");
        acceptBtn.type = "button";
        acceptBtn.className = "admin-btn admin-btn--primary";
        acceptBtn.textContent = "Accept";
        acceptBtn.style.marginRight = "6px";
        acceptBtn.addEventListener("click", () => forecastAction(r.id, "accept"));
        tdAction.appendChild(acceptBtn);
        const discardBtn = document.createElement("button");
        discardBtn.type = "button";
        discardBtn.className = "admin-btn admin-btn--secondary";
        discardBtn.textContent = "Discard";
        discardBtn.addEventListener("click", () => forecastAction(r.id, "discard"));
        tdAction.appendChild(discardBtn);
        tr.appendChild(tdAction);
        tbody.appendChild(tr);
      }
    } catch (err) {
      tbody.innerHTML = `<tr><td colspan="4" style="padding: 12px; color: var(--admin-danger, #dc2626);">Failed: ${escapeHtml(err.message)}</td></tr>`;
    }
  }

  async function forecastAction(id, action) {
    const outcomeNotes = prompt(`${action === "accept" ? "Accept" : "Discard"} — optional outcome notes:`, "");
    if (outcomeNotes === null) return; // user cancelled
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/forecast-suggestions/${id}/${action}`, {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ outcome_notes: outcomeNotes || null }),
      });
      if (!r.ok) throw new Error(await responseError(r));
      await loadPendingForecasts();
    } catch (err) {
      alert(`Forecast ${action} failed: ${err.message}`);
    }
  }

  // ─── Forecast assumptions (direct admin edit) ───────────────────────────────

  let _forecastAssumptions = []; // cache for projection recompute

  async function loadForecastAssumptions() {
    const tbody = document.getElementById("forecast-assumptions-tbody");
    const errEl = document.getElementById("forecast-assumptions-error");
    if (!tbody) return;
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/forecast-assumptions`);
      if (!resp.ok) throw new Error(await responseError(resp));
      _forecastAssumptions = await resp.json();
      renderForecastAssumptions(_forecastAssumptions);
      renderForecastProjection(_forecastAssumptions);
      if (errEl) errEl.textContent = "";
    } catch (err) {
      if (tbody) tbody.innerHTML = `<tr><td colspan="9" style="padding:16px;text-align:center;color:var(--admin-danger,#D92D20);">${err.message}</td></tr>`;
    }
  }

  function renderForecastAssumptions(rows) {
    const tbody = document.getElementById("forecast-assumptions-tbody");
    if (!tbody) return;
    if (!rows.length) {
      tbody.innerHTML = `<tr><td colspan="9" style="padding:20px;text-align:center;color:var(--admin-text-muted);">No assumptions yet — click "+ Add year" to set them.</td></tr>`;
      return;
    }
    tbody.innerHTML = "";
    for (const r of rows) {
      tbody.appendChild(buildAssumptionRow(r));
    }
  }

  function buildAssumptionRow(r) {
    const tr = document.createElement("tr");
    tr.style.borderBottom = "1px solid var(--admin-border, #e5e7eb)";
    tr.dataset.year = r.forecast_year;

    const bps2pct = v => v == null ? "" : (v / 100).toFixed(2);
    const cents2idr = v => v == null ? "" : Math.round(v / 100).toLocaleString("en-US");

    const fields = [
      { key: "year",    val: r.forecast_year, type: "year",   align: "left"  },
      { key: "projected_occupancy_bps",         val: bps2pct(r.projected_occupancy_bps),         type: "pct",    align: "right" },
      { key: "projected_adr_idr_cents",         val: cents2idr(r.projected_adr_idr_cents),        type: "idr",    align: "right" },
      { key: "projected_rent_growth_bps",       val: bps2pct(r.projected_rent_growth_bps),        type: "pct",    align: "right" },
      { key: "projected_expense_inflation_bps", val: bps2pct(r.projected_expense_inflation_bps),  type: "pct",    align: "right" },
      { key: "projected_annual_net_yield_bps",  val: bps2pct(r.projected_annual_net_yield_bps),   type: "pct",    align: "right" },
      { key: "projected_appreciation_bps",      val: bps2pct(r.projected_appreciation_bps),       type: "pct",    align: "right" },
      { key: "projected_exit_yield_bps",        val: bps2pct(r.projected_exit_yield_bps),         type: "pct",    align: "right" },
    ];

    for (const f of fields) {
      const td = document.createElement("td");
      td.style.cssText = `padding: 6px 10px; text-align: ${f.align};`;
      if (f.key === "year") {
        td.textContent = f.val;
        td.style.fontWeight = "600";
      } else {
        const inp = document.createElement("input");
        inp.type = "text";
        inp.value = f.val;
        inp.dataset.key = f.key;
        inp.dataset.type = f.type;
        inp.style.cssText = "width:72px;padding:4px 6px;border:1px solid var(--admin-border,#d1d5db);border-radius:4px;font:inherit;font-size:12px;text-align:right;";
        td.appendChild(inp);
      }
      tr.appendChild(td);
    }

    // Save button cell
    const tdBtn = document.createElement("td");
    tdBtn.style.padding = "6px 10px";
    const saveBtn = document.createElement("button");
    saveBtn.type = "button";
    saveBtn.className = "admin-btn admin-btn--primary admin-btn--sm";
    saveBtn.textContent = "Save";
    saveBtn.addEventListener("click", () => saveForecastYear(tr, r.forecast_year));
    tdBtn.appendChild(saveBtn);
    tr.appendChild(tdBtn);

    return tr;
  }

  function addForecastYear() {
    const year = parseInt(prompt("Forecast year (e.g. " + new Date().getFullYear() + "):", new Date().getFullYear() + 1), 10);
    if (!year || year < 2000 || year > 2100) return;
    if (_forecastAssumptions.some(r => r.forecast_year === year)) {
      alert(`Year ${year} already exists.`);
      return;
    }
    const stub = {
      forecast_year: year,
      projected_occupancy_bps: null, projected_adr_idr_cents: null,
      projected_rent_growth_bps: null, projected_expense_inflation_bps: null,
      projected_annual_net_yield_bps: null, projected_appreciation_bps: null,
      projected_exit_yield_bps: null,
    };
    _forecastAssumptions.push(stub);
    _forecastAssumptions.sort((a, b) => a.forecast_year - b.forecast_year);
    renderForecastAssumptions(_forecastAssumptions);
    // Scroll the new row into view
    const tbody = document.getElementById("forecast-assumptions-tbody");
    const rows = tbody ? tbody.querySelectorAll("tr") : [];
    rows[rows.length - 1]?.scrollIntoView({ block: "nearest" });
  }

  async function saveForecastYear(tr, year) {
    const errEl = document.getElementById("forecast-assumptions-error");
    const btn = tr.querySelector("button");
    if (btn) { btn.disabled = true; btn.textContent = "Saving…"; }
    if (errEl) errEl.textContent = "";

    // Collect input values from the row
    const inputs = tr.querySelectorAll("input[data-key]");
    const payload = { forecast_year: year };
    for (const inp of inputs) {
      const key = inp.dataset.key;
      const raw = inp.value.trim().replace(/,/g, "");
      if (raw === "") { payload[key] = null; continue; }
      if (inp.dataset.type === "pct") {
        // convert % → bps
        payload[key] = Math.round(parseFloat(raw) * 100);
      } else if (inp.dataset.type === "idr") {
        // IDR display is already in whole IDR (not cents) — convert back to cents
        payload[key] = Math.round(parseFloat(raw) * 100);
      } else {
        payload[key] = parseInt(raw, 10);
      }
    }

    try {
      const resp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/forecast-assumptions`,
        { method: "PUT", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(payload) }
      );
      if (!resp.ok) throw new Error(await responseError(resp));
      const updated = await resp.json();
      // Update local cache
      const idx = _forecastAssumptions.findIndex(r => r.forecast_year === year);
      if (idx >= 0) _forecastAssumptions[idx] = updated;
      else _forecastAssumptions.push(updated);
      _forecastAssumptions.sort((a, b) => a.forecast_year - b.forecast_year);
      renderForecastProjection(_forecastAssumptions);
      if (btn) { btn.disabled = false; btn.textContent = "Saved ✓"; setTimeout(() => { if (btn) btn.textContent = "Save"; }, 2000); }
    } catch (err) {
      if (errEl) errEl.textContent = `Save failed (year ${year}): ${err.message}`;
      if (btn) { btn.disabled = false; btn.textContent = "Save"; }
    }
  }

  function renderForecastProjection(assumptions) {
    const tbody = document.getElementById("forecast-projection-tbody");
    if (!tbody) return;
    const rows = assumptions.filter(r =>
      r.projected_annual_net_yield_bps != null || r.projected_appreciation_bps != null
    );
    if (!rows.length) {
      tbody.innerHTML = `<tr><td colspan="6" style="padding:16px;text-align:center;color:var(--admin-text-muted);">Set net yield % and appreciation % assumptions above to generate a projection.</td></tr>`;
      return;
    }
    tbody.innerHTML = "";
    let cumulative = 100; // normalised to 100
    for (const r of rows) {
      const yieldPct  = (r.projected_annual_net_yield_bps  ?? 0) / 100;
      const appPct    = (r.projected_appreciation_bps      ?? 0) / 100;
      const occPct    = (r.projected_occupancy_bps         ?? 0) / 100;
      const totalPct  = yieldPct + appPct;
      cumulative     *= (1 + totalPct / 100);

      const fmt1 = n => n.toFixed(2) + "%";
      const fmtC = n => n.toFixed(1);

      const tr = document.createElement("tr");
      tr.style.borderBottom = "1px solid var(--admin-border, #e5e7eb)";
      const totalColor = totalPct >= 0 ? "color:var(--admin-success,#059669)" : "color:var(--admin-danger,#D92D20)";
      tr.innerHTML = `
        <td style="padding:7px 10px;font-weight:600;">${r.forecast_year}</td>
        <td style="padding:7px 10px;text-align:right;">${occPct ? fmt1(occPct) : "—"}</td>
        <td style="padding:7px 10px;text-align:right;">${fmt1(yieldPct)}</td>
        <td style="padding:7px 10px;text-align:right;">${fmt1(appPct)}</td>
        <td style="padding:7px 10px;text-align:right;font-weight:600;${totalColor}">${fmt1(totalPct)}</td>
        <td style="padding:7px 10px;text-align:right;font-weight:700;">${fmtC(cumulative)}</td>
      `;
      tbody.appendChild(tr);
    }
  }

  // Cached current-user fetch — reused across 4-eyes hints.
  let _meId = undefined;
  async function currentUserId() {
    if (_meId !== undefined) return _meId;
    try {
      const r = await fetch("/api/me");
      if (!r.ok) { _meId = null; return null; }
      const b = await r.json();
      _meId = b?.id || b?.user?.id || null;
    } catch {
      _meId = null;
    }
    return _meId;
  }

  // ─── Valuations ─────────────────────────────────────────────

  async function loadValuations() {
    const tbody = document.getElementById("valuations-tbody");
    if (!tbody) return;
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/valuations`);
      if (!resp.ok) throw new Error(await responseError(resp));
      const rows = await resp.json();
      renderValuations(rows);
    } catch (err) {
      tbody.innerHTML = `<tr><td colspan="6" style="padding: 16px; color: var(--admin-danger, #dc2626);">Failed to load: ${escapeHtml(err.message)}</td></tr>`;
    }
  }

  function renderValuations(rows) {
    const tbody = document.getElementById("valuations-tbody");
    if (!tbody) return;
    if (!rows.length) {
      tbody.innerHTML = `<tr><td colspan="6" style="padding: 24px; text-align: center; color: var(--admin-text-muted, #6b7280);">No valuations yet — click "+ New valuation" to add the first.</td></tr>`;
      return;
    }
    tbody.innerHTML = "";
    for (const r of rows) {
      const tr = document.createElement("tr");
      tr.style.borderTop = "1px solid var(--admin-border, #e5e7eb)";

      const tdDate = document.createElement("td");
      tdDate.style.padding = "10px 12px";
      tdDate.textContent = r.valuation_date || "—";
      tr.appendChild(tdDate);

      const tdVal = document.createElement("td");
      tdVal.style.padding = "10px 12px";
      tdVal.style.textAlign = "right";
      tdVal.style.fontVariantNumeric = "tabular-nums";
      tdVal.textContent = Number(r.valuation_idr_cents).toLocaleString();
      tr.appendChild(tdVal);

      const tdMethod = document.createElement("td");
      tdMethod.style.padding = "10px 12px";
      tdMethod.textContent = (r.valuation_method || "—").replace(/_/g, " ");
      tr.appendChild(tdMethod);

      const tdAppraiser = document.createElement("td");
      tdAppraiser.style.padding = "10px 12px";
      tdAppraiser.textContent = r.appraiser_name || "—";
      tr.appendChild(tdAppraiser);

      const tdStatus = document.createElement("td");
      tdStatus.style.padding = "10px 12px";
      tdStatus.innerHTML = statusBadge(r.status);
      tr.appendChild(tdStatus);

      const tdAction = document.createElement("td");
      tdAction.style.padding = "10px 12px";
      tdAction.style.textAlign = "right";
      const a = document.createElement("a");
      a.className = "admin-btn admin-btn--secondary";
      a.href = `/admin/villas/${encodeURIComponent(assetId)}/valuations/${r.id}/edit`;
      a.textContent = r.status === "published" ? "Correct" : "Open";
      tdAction.appendChild(a);
      tr.appendChild(tdAction);

      tbody.appendChild(tr);
    }
  }

  function openNewValuation() {
    window.location.href = `/admin/villas/${encodeURIComponent(assetId)}/valuations/new`;
  }

  async function topUp(row) {
    const period = `${row.period_year}-${String(row.period_month).padStart(2, "0")}`;
    if (!confirm(`Top up corrected period ${period}? Each investor gets the positive delta vs what they already received. Negative deltas are absorbed (Q11 lock-in: never claw back).`)) return;
    try {
      const resp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${row.id}/top-up`,
        { method: "POST", headers: csrfHeaders() }
      );
      if (!resp.ok) throw new Error(await responseError(resp));
      const result = await resp.json();
      alert(
        `Top-up for ${period} done.\n` +
        `Topped up: ${result.topped_up_count}\n` +
        `Skipped (no positive delta): ${result.skipped_no_delta}\n` +
        `Total: ${Number(result.topped_up_total_cents).toLocaleString()} ${result.currency} cents`
      );
      await loadOperationsStrip();
    } catch (err) {
      alert(`Top up failed: ${err.message}`);
    }
  }

  async function distribute(row) {
    const period = `${row.period_year}-${String(row.period_month).padStart(2, "0")}`;
    if (!confirm(`Distribute & process payouts for ${period}? Investors' cash wallets will be credited and a wallet_transactions row will appear in their /transactions list. Idempotent.`)) return;
    try {
      const distResp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${row.id}/distribute`,
        { method: "POST", headers: csrfHeaders() }
      );
      if (!distResp.ok) throw new Error(await responseError(distResp));
      const distResult = await distResp.json();

      const procResp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${row.id}/process-payouts`,
        { method: "POST", headers: csrfHeaders() }
      );
      if (!procResp.ok) throw new Error(await responseError(procResp));
      const procResult = await procResp.json();

      alert(
        `Distribute & process for ${period} done.\n` +
        `Scheduled rows created: ${distResult.created} (skipped duplicates: ${distResult.skipped})\n` +
        `Paid: ${procResult.paid_count} (already-paid: ${procResult.skipped_already_paid})\n` +
        `Wallet credit total: ${Number(procResult.paid_total_cents).toLocaleString()} USD cents`
      );
      await loadOperationsStrip();
    } catch (err) {
      alert(`Distribute & process failed: ${err.message}`);
    }
  }

  function openLatestMonthEntry() {
    const now = new Date();
    const y = now.getMonth() === 0 ? now.getFullYear() - 1 : now.getFullYear();
    const m = now.getMonth() === 0 ? 12 : now.getMonth();
    window.location.href = `/admin/villas/${encodeURIComponent(assetId)}/operations/${y}/${m}`;
  }

  // ─── Villa-Returns configuration (PDF §4 master data) ───────

  async function loadVillaConfig() {
    const errEl = document.getElementById("villa-config-error");
    if (!errEl) return; // card not on page
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/config-summary`);
      if (!resp.ok) throw new Error(await responseError(resp));
      fillVillaConfig(await resp.json());
      errEl.textContent = "";
    } catch (err) {
      errEl.textContent = `Failed to load configuration: ${err.message}`;
    }
  }

  function fillVillaConfig(cfg) {
    const setVal = (id, v) => {
      const el = document.getElementById(id);
      if (el) el.value = v == null ? "" : v;
    };
    const setText = (id, v) => {
      const el = document.getElementById(id);
      if (el) el.textContent = v == null ? "—" : v;
    };
    const setChecked = (id, v) => {
      const el = document.getElementById(id);
      if (el) el.checked = !!v;
    };
    setVal("vcfg-tokenized_pct_bps", cfg.tokenized_pct_bps);
    setVal("vcfg-tokens_owner_retained", cfg.tokens_owner_retained);
    setVal("vcfg-tokens_payout_eligible", cfg.tokens_payout_eligible);
    setVal("vcfg-payout_frequency", cfg.payout_frequency);
    setVal("vcfg-payout_currency", cfg.payout_currency);
    setVal("vcfg-distribution_record_day", cfg.distribution_record_day);
    setVal("vcfg-mgmt_fee_bps", cfg.mgmt_fee_bps);
    setVal("vcfg-reserve_pct_bps", cfg.reserve_pct_bps);
    setVal("vcfg-withholding_tax_bps", cfg.withholding_tax_bps);
    setChecked("vcfg-allow_developer_submission", cfg.allow_developer_submission);
    setChecked("vcfg-villa_returns_pilot", cfg.villa_returns_pilot);
    setText("vcfg-tokens_total", cfg.tokens_total);
    setText("vcfg-native_currency_code", cfg.native_currency_code);
    setText("vcfg-poool_split_pct", cfg.poool_split_pct);
  }

  // PUT /config COALESCEs every column, so sending the full current
  // snapshot is a safe partial update — unchanged fields write their
  // own value back. Empty numeric inputs send null (keep existing).
  async function saveVillaConfig() {
    const errEl = document.getElementById("villa-config-error");
    const okEl = document.getElementById("villa-config-status");
    errEl.textContent = "";
    okEl.textContent = "";
    const numOrNull = (id) => {
      const raw = (document.getElementById(id).value || "").trim();
      return raw === "" ? null : parseInt(raw, 10);
    };
    const payload = {
      tokenized_pct_bps: numOrNull("vcfg-tokenized_pct_bps"),
      tokens_owner_retained: numOrNull("vcfg-tokens_owner_retained"),
      tokens_payout_eligible: numOrNull("vcfg-tokens_payout_eligible"),
      reserve_pct_bps: numOrNull("vcfg-reserve_pct_bps"),
      mgmt_fee_bps: numOrNull("vcfg-mgmt_fee_bps"),
      withholding_tax_bps: numOrNull("vcfg-withholding_tax_bps"),
      payout_frequency: document.getElementById("vcfg-payout_frequency").value || null,
      payout_currency:
        document.getElementById("vcfg-payout_currency").value.trim().toUpperCase() || null,
      distribution_record_day: numOrNull("vcfg-distribution_record_day"),
      allow_developer_submission: document.getElementById("vcfg-allow_developer_submission")
        .checked,
      villa_returns_pilot: document.getElementById("vcfg-villa_returns_pilot").checked,
    };
    const btn = document.getElementById("btn-save-villa-config");
    btn.disabled = true;
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/config`, {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
      if (!resp.ok) throw new Error(await responseError(resp));
      fillVillaConfig(await resp.json());
      okEl.textContent = "Configuration saved.";
      setTimeout(() => {
        okEl.textContent = "";
      }, 4000);
    } catch (err) {
      errEl.textContent = `Save failed: ${err.message}`;
    } finally {
      btn.disabled = false;
    }
  }

  // ─── Helpers (inlined to match codebase convention) ──────────

  function csrfHeaders(headers = {}) {
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    const token = parts.length === 2 ? parts.pop().split(";").shift() : null;
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  async function responseError(resp) {
    try {
      const b = await resp.json();
      return b.error || b.message || `HTTP ${resp.status}`;
    } catch {
      return `HTTP ${resp.status}`;
    }
  }

  function formatDate(iso) {
    if (!iso) return "—";
    try { return new Date(iso).toISOString().slice(0, 10); } catch { return iso; }
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }
})();
