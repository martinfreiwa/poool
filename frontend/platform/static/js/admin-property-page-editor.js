/**
 * admin-property-page-editor.js
 *
 * Shared editor for the "Property Page Content" + "Project Milestones" cards.
 * Used by both:
 *   - /admin/developer-submission-review
 *   - /admin/asset-details
 *
 * Public API:
 *   PropertyPageEditor.init({ assetId, asset, milestones })
 *
 *   - assetId    UUID string of the asset.
 *   - asset      Object containing the editable fields (location_description,
 *                investment_type, leasing_items, info_badges, etc.).
 *   - milestones Array of milestone rows ({id, title, description,
 *                milestone_date, month_index, is_completed}).
 *
 * Hits:
 *   PATCH  /api/admin/assets/:id/page-content
 *   POST   /api/admin/assets/:id/milestones
 *   PATCH  /api/admin/assets/:id/milestones/:milestoneId
 *   DELETE /api/admin/assets/:id/milestones/:milestoneId
 */
(function () {
  const ICON_FALLBACKS = [
    "/static/images/prop-details/ID.webp",
    "/static/images/prop-details/house.webp",
    "/static/images/prop-details/coins-stacked-02.webp",
    "/static/images/prop-details/line-chart-up-02.webp",
  ];

  const PropertyPageEditor = {
    _assetId: null,

    init({ assetId, asset, milestones }) {
      if (!assetId) {
        console.warn("PropertyPageEditor.init: missing assetId");
        return;
      }
      this._assetId = assetId;
      this._wireSave();
      this._wireAddRowButtons();
      this._wireMilestoneAdd();
      this._wireMilestoneSaveAll();
      this._hydrateForm(asset || {});
      this._hydrateInfoBadges(asset?.info_badges || []);
      this._hydrateLeasingItems(asset?.leasing_items || []);
      this._hydrateRiskNotifications(asset?.risk_notification_items, asset?.risk_notification);
      this._renderMilestones(milestones || []);
    },

    // ─── Top-level form ────────────────────────────────────────────────────
    _hydrateForm(asset) {
      const set = (id, v) => {
        const el = document.getElementById(id);
        if (el) el.value = v == null ? "" : v;
      };
      set("pc-location-description", asset.location_description);
      set("pc-investment-type", asset.investment_type);
      set("pc-investment-type-description", asset.investment_type_description);
      set("pc-leasing-strategy-type", asset.leasing_strategy_type);
      set("pc-leasing-strategy-description", asset.leasing_strategy_description);
      set(
        "pc-default-investment-amount",
        asset.default_investment_amount_cents != null
          ? Math.round(asset.default_investment_amount_cents / 100)
          : ""
      );
      set(
        "pc-default-value-growth",
        asset.default_value_growth_bps != null ? asset.default_value_growth_bps / 100 : ""
      );
      set(
        "pc-default-rental-yield",
        asset.default_rental_yield_bps != null ? asset.default_rental_yield_bps / 100 : ""
      );
      set("pc-developer-name", asset.developer_name);
      set("pc-developer-logo-url", asset.developer_logo_url);
      set("pc-developer-description", asset.developer_description);
      set("pc-developer-website", asset.developer_website);
      set("pc-developer-facebook", asset.developer_facebook);
      set("pc-developer-instagram", asset.developer_instagram);
      set("pc-developer-youtube", asset.developer_youtube);
      set("pc-google-maps-url", asset.google_maps_url);
    },

    _wireSave() {
      const btn = document.getElementById("btn-save-page-content");
      const status = document.getElementById("page-content-status");
      if (!btn) return;
      btn.onclick = async () => {
        const body = this._collectPayload();
        btn.disabled = true;
        if (status) status.textContent = "Saving…";
        try {
          const res = await fetch(
            `/api/admin/assets/${this._assetId}/page-content`,
            {
              method: "PATCH",
              headers: {
                "Content-Type": "application/json",
                "X-CSRF-Token": (typeof getCsrfToken === "function") ? getCsrfToken() : "",
              },
              body: JSON.stringify(body),
            }
          );
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || `HTTP ${res.status}`);
          }
          const data = await res.json();
          if (status) status.textContent = `Saved ${data.fields_updated?.length ?? 0} fields`;
          this._toast("Property page content saved", "success");
        } catch (e) {
          if (status) status.textContent = "";
          this._toast(`Save failed: ${e.message}`, "error");
        } finally {
          btn.disabled = false;
        }
      };
    },

    _collectPayload() {
      const form = document.getElementById("page-content-form");
      const out = {};
      if (form) {
        form.querySelectorAll("[data-field]").forEach((el) => {
          const field = el.dataset.field;
          const unit = el.dataset.unit;
          const raw = el.value;
          if (unit === "cents") {
            out[field] = raw === "" ? null : Math.round(Number(raw) * 100);
          } else if (unit === "bps") {
            out[field] = raw === "" ? null : Math.round(Number(raw) * 100);
          } else {
            out[field] = raw.trim() === "" ? null : raw.trim();
          }
        });
      }
      out.info_badges = this._collectInfoBadges();
      out.leasing_items = this._collectLeasingItems();
      out.risk_notification_items = this._collectRiskNotifications();
      // Legacy text field is superseded by structured items; clear so the
      // template prefers the new array.
      out.risk_notification = null;
      return out;
    },

    // ─── Info Badges list ─────────────────────────────────────────────────
    _hydrateInfoBadges(items) {
      const list = document.getElementById("pc-info-badges-list");
      if (!list) return;
      list.innerHTML = "";
      this._asArray(items).forEach((b) => list.appendChild(this._infoBadgeRow(b)));
    },

    _infoBadgeRow(b) {
      const row = document.createElement("div");
      row.className = "pc-list-row";
      row.dataset.kind = "info-badges";
      row.innerHTML = `
        <input class="pc-input" data-key="icon_url" placeholder="Icon URL" maxlength="512" />
        <input class="pc-input" data-key="title" placeholder="Title (e.g. ID, Bali)" />
        <input class="pc-input" data-key="subtitle" placeholder="Subtitle" />
        <button type="button" class="pc-row-delete" title="Remove">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/></svg>
        </button>
      `;
      row.querySelector('[data-key="icon_url"]').value = b?.icon_url ?? "";
      row.querySelector('[data-key="title"]').value = b?.title ?? "";
      row.querySelector('[data-key="subtitle"]').value = b?.subtitle ?? "";
      row.querySelector(".pc-row-delete").onclick = () => row.remove();
      return row;
    },

    _collectInfoBadges() {
      const list = document.getElementById("pc-info-badges-list");
      if (!list) return null;
      const rows = Array.from(list.querySelectorAll('[data-kind="info-badges"]'));
      if (rows.length === 0) return null;
      return rows
        .map((r, i) => {
          const get = (k) => r.querySelector(`[data-key="${k}"]`)?.value?.trim() || "";
          const icon_url = get("icon_url") || ICON_FALLBACKS[i % ICON_FALLBACKS.length];
          const title = get("title");
          const subtitle = get("subtitle");
          if (!title && !subtitle) return null; // skip empty rows
          return { icon_url, title, subtitle };
        })
        .filter(Boolean);
    },

    // ─── Leasing Items list ────────────────────────────────────────────────
    _hydrateLeasingItems(items) {
      const list = document.getElementById("pc-leasing-items-list");
      if (!list) return;
      list.innerHTML = "";
      this._asArray(items).forEach((b) => list.appendChild(this._leasingItemRow(b)));
    },

    _leasingItemRow(b) {
      const row = document.createElement("div");
      row.className = "pc-list-row";
      row.dataset.kind = "leasing-items";
      row.innerHTML = `
        <input class="pc-input" data-key="title" placeholder="Title (e.g. Professional management)" />
        <textarea class="pc-input" data-key="description" rows="2" placeholder="Description"></textarea>
        <button type="button" class="pc-row-delete" title="Remove">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/></svg>
        </button>
      `;
      row.querySelector('[data-key="title"]').value = b?.title ?? "";
      row.querySelector('[data-key="description"]').value = b?.description ?? "";
      row.querySelector(".pc-row-delete").onclick = () => row.remove();
      return row;
    },

    _collectLeasingItems() {
      const list = document.getElementById("pc-leasing-items-list");
      if (!list) return null;
      const rows = Array.from(list.querySelectorAll('[data-kind="leasing-items"]'));
      if (rows.length === 0) return null;
      return rows
        .map((r) => {
          const get = (k) => r.querySelector(`[data-key="${k}"]`)?.value?.trim() || "";
          const title = get("title");
          const description = get("description");
          if (!title && !description) return null;
          return { title, description };
        })
        .filter(Boolean);
    },

    // ─── Risk Notifications list ──────────────────────────────────────────
    _hydrateRiskNotifications(items, legacyText) {
      const list = document.getElementById("pc-risk-notifications-list");
      if (!list) return;
      list.innerHTML = "";
      let rows = [];
      if (Array.isArray(items)) {
        rows = items
          .map((it) => ({
            title: String(it?.title ?? "").trim(),
            body: String(it?.body ?? "").trim(),
          }))
          .filter((r) => r.title || r.body);
      }
      if (rows.length === 0 && legacyText) {
        rows = String(legacyText)
          .split(/\n+/)
          .map((line) => line.replace(/^[-*]\s+/, "").trim())
          .filter(Boolean)
          .map((body) => ({ title: "", body }));
      }
      if (rows.length === 0) {
        list.appendChild(this._riskNotificationRow({}));
        return;
      }
      rows.forEach((r) => list.appendChild(this._riskNotificationRow(r)));
    },

    _riskNotificationRow(item) {
      const row = document.createElement("div");
      row.className = "pc-list-row";
      row.dataset.kind = "risk-notifications";
      row.innerHTML = `
        <input class="pc-input" data-key="title" maxlength="255" placeholder="Title (e.g. Developer Issues)" />
        <textarea class="pc-input" data-key="body" rows="2" maxlength="8000" placeholder="Risk description shown on the property page."></textarea>
        <button type="button" class="pc-row-delete" title="Remove">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/></svg>
        </button>
      `;
      row.querySelector('[data-key="title"]').value = item?.title || "";
      row.querySelector('[data-key="body"]').value = item?.body || "";
      row.querySelector(".pc-row-delete").onclick = () => {
        row.remove();
        const list = document.getElementById("pc-risk-notifications-list");
        if (list && list.children.length === 0) list.appendChild(this._riskNotificationRow({}));
      };
      return row;
    },

    _collectRiskNotifications() {
      const list = document.getElementById("pc-risk-notifications-list");
      if (!list) return null;
      const rows = Array.from(list.querySelectorAll('[data-kind="risk-notifications"]'));
      const items = rows
        .map((row) => ({
          title: row.querySelector('[data-key="title"]')?.value?.trim() || "",
          body: row.querySelector('[data-key="body"]')?.value?.trim() || "",
        }))
        .filter((r) => r.title || r.body);
      return items.length === 0 ? null : items;
    },

    _wireAddRowButtons() {
      document.querySelectorAll(".pc-add-row-btn").forEach((btn) => {
        btn.type = "button";
        btn.onclick = (event) => {
          event.preventDefault();
          const which = btn.dataset.list;
          if (which === "info-badges") {
            document.getElementById("pc-info-badges-list")?.appendChild(this._infoBadgeRow({}));
          } else if (which === "leasing-items") {
            document.getElementById("pc-leasing-items-list")?.appendChild(this._leasingItemRow({}));
          } else if (which === "risk-notifications") {
            document.getElementById("pc-risk-notifications-list")?.appendChild(this._riskNotificationRow(""));
          }
        };
      });
    },

    // ─── Milestones ────────────────────────────────────────────────────────
    _renderMilestones(milestones) {
      const tbody = document.getElementById("milestones-container");
      if (!tbody) return;
      if (!milestones || milestones.length === 0) {
        tbody.innerHTML = `<tr><td colspan="6" style="padding:20px 28px;text-align:center;color:var(--admin-text-muted);font-size:12px;">
          No milestones yet. Click "Add milestone" to create the project roadmap.
        </td></tr>`;
        return;
      }
      tbody.innerHTML = milestones.map((m) => this._milestoneRowHtml(m)).join("");
      tbody.querySelectorAll("[data-milestone-id]").forEach((row) => this._wireMilestoneRow(row));
    },

    _milestoneRowHtml(m) {
      const id = this._esc(m.id || `new-${Date.now()}`);
      const month = m.month_index != null ? m.month_index : "";
      const dateVal = m.milestone_date ? this._toDateInputValue(m.milestone_date) : "";
      const isNew = !m.id;
      return `
        <tr data-milestone-id="${id}" ${isNew ? 'data-new-milestone="true"' : ""}>
          <td><input class="pc-input ms-month" type="number" min="0" step="1" value="${this._esc(String(month))}" style="width:60px;padding:6px 8px;font-size:12px;" /></td>
          <td><input class="pc-input ms-title" value="${this._esc(m.title || "")}" maxlength="255" style="font-size:13px;font-weight:600;" /></td>
          <td><textarea class="pc-input ms-desc" rows="2" style="font-size:12px;">${this._esc(m.description || "")}</textarea></td>
          <td><input class="pc-input ms-date" type="date" value="${this._esc(dateVal)}" style="font-size:12px;" /></td>
          <td style="text-align:center;"><input type="checkbox" class="ms-done" ${m.is_completed ? "checked" : ""} style="width:16px;height:16px;cursor:pointer;" /></td>
          <td style="text-align:right;">
            <button type="button" class="admin-btn admin-btn--secondary ms-save-btn" style="padding:4px 8px;font-size:11px;margin-right:4px;${isNew ? "" : "display:none;"}">${isNew ? "Create" : "Save"}</button>
            <button type="button" class="ms-delete-btn" title="Delete" style="background:transparent;border:none;color:var(--admin-danger);cursor:pointer;padding:4px;">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>
            </button>
          </td>
        </tr>
      `;
    },

    _wireMilestoneRow(row) {
      const id = row.dataset.milestoneId;
      const isNew = row.dataset.newMilestone === "true";
      const saveBtn = row.querySelector(".ms-save-btn");
      row.dataset.dirty = isNew ? "true" : "false";
      const showSave = () => {
        row.dataset.dirty = "true";
        if (saveBtn) saveBtn.style.display = "inline-block";
      };
      row.querySelectorAll("input, textarea").forEach((inp) => {
        inp.addEventListener("input", showSave);
        inp.addEventListener("change", showSave);
      });
      if (saveBtn) {
        saveBtn.addEventListener("click", async () => {
          try {
            await this._saveMilestoneRow(row, { showToast: true });
          } catch (e) {
            this._toast(`Failed to save milestone: ${e.message}`, "error");
          }
        });
      }
      const deleteBtn = row.querySelector(".ms-delete-btn");
      if (deleteBtn) {
        deleteBtn.addEventListener("click", async () => {
          if (isNew) {
            row.remove();
            return;
          }
          if (!confirm("Delete this milestone?")) return;
          try {
            const res = await fetch(
              `/api/admin/assets/${this._assetId}/milestones/${id}`,
              {
                method: "DELETE",
                headers: { "X-CSRF-Token": (typeof getCsrfToken === "function") ? getCsrfToken() : "" },
              }
            );
            if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
            row.remove();
            this._toast("Milestone deleted", "success");
          } catch (e) {
            this._toast(`Failed to delete: ${e.message}`, "error");
          }
        });
      }
    },

    _collectMilestoneInputs(row) {
      const month = row.querySelector(".ms-month").value.trim();
      const title = row.querySelector(".ms-title").value.trim();
      const desc = row.querySelector(".ms-desc").value.trim();
      const date = row.querySelector(".ms-date").value;
      const done = row.querySelector(".ms-done").checked;
      return {
        title: title || null,
        description: desc === "" ? null : desc,
        month_index: month === "" ? null : Number(month),
        milestone_date: date ? new Date(date).toISOString() : null,
        is_completed: done,
      };
    },

    async _saveMilestoneRow(row, { showToast = false } = {}) {
      const id = row.dataset.milestoneId;
      const isNew = row.dataset.newMilestone === "true";
      const saveBtn = row.querySelector(".ms-save-btn");
      const payload = this._collectMilestoneInputs(row);
      if (!payload.title) {
        row.querySelector(".ms-title")?.focus();
        throw new Error("Milestone title is required");
      }

      if (saveBtn) saveBtn.disabled = true;
      try {
        const endpoint = isNew
          ? `/api/admin/assets/${this._assetId}/milestones`
          : `/api/admin/assets/${this._assetId}/milestones/${id}`;
        const res = await fetch(endpoint, {
          method: isNew ? "POST" : "PATCH",
          headers: {
            "Content-Type": "application/json",
            "X-CSRF-Token": (typeof getCsrfToken === "function") ? getCsrfToken() : "",
          },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);

        if (isNew) {
          const created = await res.json();
          row.outerHTML = this._milestoneRowHtml(created);
          const newRow = document.querySelector(`[data-milestone-id="${created.id}"]`);
          if (newRow) this._wireMilestoneRow(newRow);
          if (showToast) this._toast("Milestone added", "success");
          return created;
        }

        row.dataset.dirty = "false";
        if (saveBtn) saveBtn.style.display = "none";
        if (showToast) this._toast("Milestone updated", "success");
        return null;
      } finally {
        if (saveBtn) saveBtn.disabled = false;
      }
    },

    async _saveAllMilestones() {
      const status = document.getElementById("milestone-save-status");
      const btn = document.getElementById("btn-milestone-save-all");
      const tbody = document.getElementById("milestones-container");
      if (!tbody) return;

      const rows = Array.from(tbody.querySelectorAll("[data-milestone-id]"))
        .filter((row) => row.dataset.newMilestone === "true" || row.dataset.dirty === "true");
      if (rows.length === 0) {
        if (status) status.textContent = "No milestone changes";
        return;
      }

      if (btn) btn.disabled = true;
      if (status) status.textContent = `Saving ${rows.length} milestone${rows.length === 1 ? "" : "s"}...`;
      try {
        for (const row of rows) {
          await this._saveMilestoneRow(row);
        }
        if (status) status.textContent = `Saved ${rows.length} milestone${rows.length === 1 ? "" : "s"}`;
        this._toast("Milestones saved", "success");
      } catch (e) {
        if (status) status.textContent = "";
        this._toast(`Failed to save milestones: ${e.message}`, "error");
      } finally {
        if (btn) btn.disabled = false;
      }
    },

    _wireMilestoneAdd() {
      const btn = document.getElementById("btn-milestone-add");
      if (!btn) return;
      btn.type = "button";
      btn.onclick = (event) => {
        event.preventDefault();
        const tbody = document.getElementById("milestones-container");
        if (!tbody) return;
        if (tbody.querySelector("td[colspan]")) tbody.innerHTML = "";
        const draft = { title: "", description: "", month_index: null, milestone_date: null, is_completed: false };
        tbody.insertAdjacentHTML("beforeend", this._milestoneRowHtml(draft));
        const row = tbody.querySelector("tr[data-new-milestone]:last-child");
        if (row) {
          this._wireMilestoneRow(row);
          row.querySelector(".ms-title")?.focus();
        }
      };
    },

    _wireMilestoneSaveAll() {
      const btn = document.getElementById("btn-milestone-save-all");
      if (!btn) return;
      btn.type = "button";
      btn.onclick = (event) => {
        event.preventDefault();
        this._saveAllMilestones();
      };
    },

    // ─── Helpers ───────────────────────────────────────────────────────────
    _toast(msg, kind) {
      if (typeof showToast === "function") {
        showToast(msg, kind);
      } else {
        console.log(`[${kind}] ${msg}`);
      }
    },

    _esc(s) {
      return String(s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
    },

    _toDateInputValue(iso) {
      const m = String(iso).match(/^(\d{4}-\d{2}-\d{2})/);
      return m ? m[1] : "";
    },

    _asArray(value) {
      if (Array.isArray(value)) return value;
      if (value == null || value === "") return [];
      if (typeof value === "string") {
        try {
          const parsed = JSON.parse(value);
          return Array.isArray(parsed) ? parsed : [];
        } catch (_) {
          return [];
        }
      }
      if (typeof value === "object" && Array.isArray(value.items)) return value.items;
      return [];
    },
  };

  window.PropertyPageEditor = PropertyPageEditor;
})();
