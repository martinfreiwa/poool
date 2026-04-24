/**
 * Admin Storage Analytics - CSP-safe renderer for /admin/storage.html.
 */
(function () {
    "use strict";

    const state = {
        loading: true,
        error: null,
        data: null,
    };

    const els = {};

    document.addEventListener("DOMContentLoaded", init);

    function init() {
        els.root = document.getElementById("storage-analytics");
        if (!els.root) return;

        els.view = document.getElementById("storage-view");
        els.bucket = document.getElementById("storage-bucket");
        els.refresh = document.getElementById("storage-refresh");

        if (els.refresh) {
            els.refresh.addEventListener("click", reload);
        }

        load();
    }

    async function reload() {
        state.loading = true;
        state.error = null;
        render();
        await load();
    }

    async function load() {
        try {
            const resp = await fetch("/api/admin/storage");
            if (!resp.ok) {
                const body = await resp.json().catch(() => ({}));
                throw new Error(body.error || `HTTP ${resp.status}`);
            }
            state.data = await resp.json();
            state.error = null;
        } catch (error) {
            console.error("[AdminStorage] load error:", error);
            captureException(error);
            state.error = error.message || "Failed to load storage analytics.";
        } finally {
            state.loading = false;
            render();
        }
    }

    function render() {
        if (!els.view) return;

        if (els.bucket) {
            els.bucket.textContent = state.data?.bucket || "not configured";
        }

        if (state.loading) {
            els.view.innerHTML = renderLoading();
            return;
        }

        if (state.error) {
            els.view.innerHTML = renderError(state.error);
            const retry = els.view.querySelector("[data-action='retry']");
            if (retry) retry.addEventListener("click", reload);
            return;
        }

        if (!state.data) {
            els.view.innerHTML = renderError("No storage data returned.");
            return;
        }

        els.view.innerHTML = [
            renderStatusOverview(state.data),
            renderKpis(state.data),
            renderCharts(state.data),
            renderRecentUploads(state.data),
            renderDisclaimer(state.data),
        ].join("");
    }

    function renderLoading() {
        return `
            <div style="display:flex;flex-direction:column;gap:16px;">
                <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:16px;">
                    ${Array.from({ length: 4 }).map(() => '<div style="height:120px;border-radius:var(--admin-radius-lg);background:var(--admin-bg-card);border:1px solid var(--admin-border);animation:pulse 1.5s infinite;"></div>').join("")}
                </div>
                <div class="admin-section-grid">
                    <div style="height:300px;border-radius:var(--admin-radius-lg);background:var(--admin-bg-card);border:1px solid var(--admin-border);animation:pulse 1.5s infinite;"></div>
                    <div style="height:300px;border-radius:var(--admin-radius-lg);background:var(--admin-bg-card);border:1px solid var(--admin-border);animation:pulse 1.5s infinite;"></div>
                </div>
            </div>`;
    }

    function renderStatusOverview(data) {
        const summary = data.summary || {};
        return `
            <div class="status-overview animate-up delay-1">
                ${statusBadge("success", `${getStatus(data, "approved")} Approved documents`)}
                ${statusBadge("warning", `${getStatus(data, "pending")} Pending approval`)}
                ${statusBadge("danger", `${getStatus(data, "rejected")} Rejected`)}
                ${statusBadge("neutral", `${summary.avatars || 0} User Avatars`, "margin-left:auto;")}
                ${statusBadge("neutral", `${summary.asset_images || 0} Asset Images`)}
                ${statusBadge("neutral", `${(summary.kyc_documents || 0) + (summary.asset_documents || 0)} Real Documents`)}
            </div>`;
    }

    function statusBadge(type, text, extraStyle) {
        return `
            <span class="admin-badge admin-badge--${type}" style="${extraStyle || ""}">
                <span class="admin-badge-dot"></span>
                <span>${escapeHtml(text)}</span>
            </span>`;
    }

    function renderKpis(data) {
        const summary = data.summary || {};
        const cost = data.cost_estimate || {};
        return `
            <div class="admin-kpi-grid animate-up delay-1">
                ${kpiCard("Total GCS Files", number(summary.total_files), "Images & documents")}
                ${kpiCard("Estimated Storage", formatBytes(summary.estimated_storage_bytes), `${toFixed(summary.estimated_storage_gb, 3)} GB projected`)}
                ${kpiCard("Storage Cost / Month", `$${toFixed(cost.storage_per_month_usd, 4)}`, "Standard - $0.020/GB limit")}
                ${kpiCard("Operations Cost / Month", `$${toFixed(cost.operations_per_month_usd, 4)}`, "Class A/B reads & writes")}
            </div>`;
    }

    function kpiCard(label, value, subtext) {
        return `
            <div class="admin-kpi-card">
                <div class="admin-kpi-header">
                    <h3 class="admin-kpi-label">${escapeHtml(label)}</h3>
                    <div class="admin-kpi-icon admin-kpi-icon--neutral"></div>
                </div>
                <div class="admin-kpi-value">${escapeHtml(value)}</div>
                <div class="admin-kpi-subtext">${escapeHtml(subtext)}</div>
            </div>`;
    }

    function renderCharts(data) {
        return `
            <div class="admin-section-grid animate-up delay-2">
                <div class="admin-card">
                    <div class="admin-card-header">
                        <h3 class="admin-card-title">Upload Trend (6 Months)</h3>
                    </div>
                    <div class="admin-card-body">
                        ${renderMonthlyTrend(data.monthly_trend || [])}
                    </div>
                </div>
                <div class="admin-card">
                    <div class="admin-card-header">
                        <h3 class="admin-card-title">Storage by Type</h3>
                    </div>
                    <div class="admin-card-body">
                        ${renderTypeBreakdown(data.breakdown_by_type || [])}
                    </div>
                </div>
            </div>`;
    }

    function renderMonthlyTrend(trend) {
        if (!trend.length) {
            return emptyState("No activity in the last 6 months.");
        }

        const maxUploads = Math.max(...trend.map((row) => Number(row.uploads) || 0), 1);
        return `
            <div class="bar-chart">
                ${trend.map((bar) => {
                    const uploads = Number(bar.uploads) || 0;
                    const height = Math.max((uploads / maxUploads) * 100, 2);
                    return `
                        <div class="bar-col">
                            <div class="bar-label-top">${escapeHtml(number(uploads))}</div>
                            <div class="bar-fill" style="height:${height}%"></div>
                            <div class="bar-label-bottom">${escapeHtml(bar.month || "")}</div>
                        </div>`;
                }).join("")}
            </div>`;
    }

    function renderTypeBreakdown(types) {
        if (!types.length) {
            return emptyState("No documents stored yet.");
        }

        const maxCount = Math.max(...types.map((row) => Number(row.count) || 0), 1);
        return `
            <div>
                ${types.map((type, index) => {
                    const count = Number(type.count) || 0;
                    const width = Math.max((count / maxCount) * 100, 3);
                    return `
                        <div class="type-bar-row">
                            <div class="admin-kpi-icon admin-kpi-icon--neutral" style="width:24px;height:24px;font-size:10px;"></div>
                            <div class="type-bar-name">${escapeHtml(typeName(type.type))}</div>
                            <div class="type-bar-track">
                                <div class="type-bar-fill" style="width:${width}%;background:${typeColor(index)}"></div>
                            </div>
                            <div class="type-bar-count">${escapeHtml(number(count))}</div>
                            <div class="type-bar-mb">${escapeHtml(String(type.estimated_mb || 0))} MB</div>
                        </div>`;
                }).join("")}
            </div>`;
    }

    function renderRecentUploads(data) {
        const uploads = data.recent_uploads || [];
        const body = uploads.length ? `
            <div style="overflow-x:auto;">
                <table class="admin-table">
                    <thead>
                        <tr>
                            <th>Type</th>
                            <th>Document ID</th>
                            <th>User Account</th>
                            <th>Status</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${uploads.map(renderUploadRow).join("")}
                    </tbody>
                </table>
            </div>` : emptyState("No recent uploads found.");

        return `
            <div class="admin-card animate-up delay-3">
                <div class="admin-card-header">
                    <h3 class="admin-card-title">Recent Document Uploads</h3>
                    <div class="admin-card-actions">
                        <a href="/admin/kyc.html" class="admin-btn admin-btn--secondary admin-btn--sm">View KYC Queue</a>
                    </div>
                </div>
                <div class="admin-card-body admin-card-body--flush">${body}</div>
            </div>`;
    }

    function renderUploadRow(upload) {
        const isAsset = String(upload.document_type || "").startsWith("asset_");
        const status = upload.status || "pending";
        const statusClass = status === "approved"
            ? "admin-badge--success"
            : status === "rejected"
                ? "admin-badge--danger"
                : "admin-badge--warning";

        return `
            <tr>
                <td>
                    <div style="display:flex;align-items:center;gap:8px;font-weight:500;">
                        <span>${escapeHtml(typeName(upload.document_type))}</span>
                    </div>
                </td>
                <td style="font-family:monospace;color:var(--admin-text-secondary);">${escapeHtml(upload.id)}</td>
                <td>${escapeHtml(upload.user_email || "unknown")}</td>
                <td>
                    <span class="admin-badge ${statusClass}">
                        <span class="admin-badge-dot"></span>
                        <span style="text-transform:capitalize;">${escapeHtml(status)}</span>
                    </span>
                </td>
                <td>
                    ${isAsset
                        ? '<span style="color:var(--admin-text-muted);font-size:12px;">Auto-approved</span>'
                        : '<a href="/admin/kyc.html" class="admin-btn admin-btn--secondary admin-btn--sm" style="padding:4px 8px;font-size:12px;">Review</a>'}
                </td>
            </tr>`;
    }

    function renderDisclaimer(data) {
        const note = data.cost_estimate?.pricing_note || "";
        if (!note) return "";

        return `
            <div class="note-banner animate-up delay-3">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="flex-shrink:0;margin-top:1px;">
                    <circle cx="12" cy="12" r="10"></circle>
                    <path d="M12 8v4m0 4v.01"></path>
                </svg>
                <span>${escapeHtml(note)}</span>
            </div>`;
    }

    function renderError(message) {
        return `
            <div style="text-align:center;padding:60px;color:var(--admin-text-muted);">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" style="margin:0 auto 16px;display:block;opacity:0.4;">
                    <circle cx="12" cy="12" r="10"></circle>
                    <path d="M12 8v4m0 4v.01"></path>
                </svg>
                <p style="font-size:14px;color:var(--admin-danger);">${escapeHtml(message)}</p>
                <button class="admin-btn admin-btn--primary" style="margin-top:16px;" data-action="retry">Try again</button>
            </div>`;
    }

    function emptyState(message) {
        return `<div style="text-align:center;padding:40px;color:var(--admin-text-muted);font-size:13px;">${escapeHtml(message)}</div>`;
    }

    function getStatus(data, status) {
        const row = (data.breakdown_by_status || []).find((item) => item.status === status);
        return row?.count || 0;
    }

    function formatBytes(bytes) {
        const value = Number(bytes) || 0;
        if (value < 1_048_576) return (value / 1024).toFixed(1) + " KB";
        if (value < 1_073_741_824) return (value / 1_048_576).toFixed(1) + " MB";
        return (value / 1_073_741_824).toFixed(3) + " GB";
    }

    function typeName(type) {
        const names = {
            passport: "Passport",
            national_id: "National ID",
            driving_licence: "Driving Licence",
            proof_of_address: "Proof of Address",
            asset_image: "Asset Image",
            user_avatar: "User Avatar",
            asset_proof_of_title: "Proof of Title",
            asset_legal_basis: "Legal Basis",
            asset_building_permit: "Building Permit",
            asset_site_plan: "Site Plan",
            asset_tax_npwp: "Tax NPWP",
            asset_tax_pbb: "Tax PBB",
            asset_tax_bphtb: "Tax BPHTB",
            asset_license_nib: "License NIB",
            asset_id_card: "ID Card",
            asset_owner_npwp: "Owner NPWP",
            asset_expose: "Property Expose",
            asset_appraisal: "Appraisal",
            asset_financial: "Financial Model",
            asset_floor_plan: "Floor Plan",
            asset_other: "Asset Document",
            other: "Other",
        };

        if (names[type]) return names[type];
        return String(type || "other")
            .split("_")
            .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
            .join(" ");
    }

    function typeColor(index) {
        const palette = ["#3b82f6", "#8b5cf6", "#10b981", "#f59e0b", "#ef4444", "#06b6d4"];
        return palette[index % palette.length];
    }

    function number(value) {
        return Number(value || 0).toLocaleString();
    }

    function toFixed(value, decimals) {
        return Number(value || 0).toFixed(decimals);
    }

    function escapeHtml(value) {
        return String(value == null ? "" : value)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    function captureException(error) {
        if (typeof window.Sentry !== "undefined") window.Sentry.captureException(error);
    }
})();
