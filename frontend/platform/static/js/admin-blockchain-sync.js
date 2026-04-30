/**
 * Web3 Sync & Health Page Controller (8C.5)
 *
 * Fetches data from /api/admin/blockchain/sync and populates:
 * - Event Indexer status KPIs
 * - Settlement worker stats
 * - KYC Whitelist sync queue with "Force Sync" buttons
 * - Live-style event log terminal
 */
document.addEventListener("DOMContentLoaded", () => {
    initBlockchainSync();
});

async function initBlockchainSync() {
    try {
        const response = await fetch("/api/admin/blockchain/sync");

        if (response.status === 401 || response.status === 403) {
            setTerminalLog("[ERROR] Unauthorized. Insufficient permissions.");
            return;
        }

        if (!response.ok) {
            throw new Error(`API Error: ${response.status}`);
        }

        const data = await response.json();
        renderKPIs(data);
        renderWhitelistQueue(data.whitelist_queue || []);
        renderConfigPanel(data.config || {});
        renderTerminalLog(data);
    } catch (e) {
        console.error("Failed to fetch blockchain sync data:", e);
        setTerminalLog(`[ERROR] Failed to load sync data: ${e.message}`);
    }
}

function renderKPIs(data) {
    const ix = data.indexer || {};
    const st = data.settlement || {};

    // Indexer KPIs
    const indexerStatusEl = document.getElementById("kpi-indexer-status");
    if (indexerStatusEl) {
        if (ix.enabled) {
            indexerStatusEl.innerHTML = '<span class="admin-badge admin-badge--success" style="padding:4px 10px;">Active</span>';
        } else {
            indexerStatusEl.innerHTML = '<span class="admin-badge admin-badge--warning" style="padding:4px 10px;">Disabled</span>';
        }
    }

    setText("kpi-last-block", ix.last_synced_block ? ix.last_synced_block.toLocaleString() : "—");
    setText("kpi-last-sync", ix.last_updated_at || "Never");
    setText("kpi-poll-interval", `${ix.poll_interval_secs || 5}s`);
    setText("kpi-confirmation-depth", `${ix.confirmation_depth || 3} blocks`);
    setText("kpi-balance-entries", (ix.total_balance_entries || 0).toLocaleString());

    // Settlement KPIs
    const settlementStatusEl = document.getElementById("kpi-settlement-status");
    if (settlementStatusEl) {
        if (st.enabled) {
            settlementStatusEl.innerHTML = '<span class="admin-badge admin-badge--success" style="padding:4px 10px;">Active</span>';
        } else {
            settlementStatusEl.innerHTML = '<span class="admin-badge admin-badge--warning" style="padding:4px 10px;">Disabled</span>';
        }
    }

    setText("kpi-pending-trades", (st.pending_trades || 0).toLocaleString());
    setText("kpi-submitted-trades", (st.submitted_trades || 0).toLocaleString());
    setText("kpi-confirmed-trades", (st.confirmed_trades || 0).toLocaleString());
    setText("kpi-failed-batches", (st.failed_batches_last_24h || 0).toLocaleString());
    setText("kpi-last-batch", st.last_batch_at || "Never");
    setText("kpi-avg-batch", st.avg_batch_size ? st.avg_batch_size.toFixed(1) : "0");
}

function renderWhitelistQueue(queue) {
    const tbody = document.getElementById("whitelist-tbody");
    if (!tbody) return;

    const badge = document.getElementById("whitelist-count-badge");

    if (!queue || queue.length === 0) {
        if (badge) {
            badge.textContent = "0 pending";
            badge.style.color = "var(--admin-success)";
        }
        tbody.innerHTML = `<tr><td colspan="5" style="text-align:center; padding:40px; color:var(--admin-text-muted);">
            <div style="font-size:14px; font-weight:600; margin-bottom:4px; color:var(--admin-success);">All Clear</div>
            Whitelist Queue Empty - All KYC-approved users have been synced to Polygon.
        </td></tr>`;
        return;
    }

    if (badge) {
        badge.textContent = `${queue.length} pending`;
        badge.style.color = "var(--admin-danger)";
    }

    tbody.replaceChildren(...queue.map(buildWhitelistRow));
}

function buildWhitelistRow(user) {
    const row = document.createElement("tr");

    const userCell = document.createElement("td");
    const userWrap = document.createElement("div");
    userWrap.style.display = "flex";
    userWrap.style.flexDirection = "column";
    userWrap.style.gap = "2px";

    const email = document.createElement("span");
    email.style.fontWeight = "600";
    email.style.color = "var(--admin-text-primary)";
    email.textContent = user.email || "Unknown user";

    const id = document.createElement("span");
    id.style.fontSize = "11px";
    id.style.color = "var(--admin-text-muted)";
    id.style.fontFamily = "'SF Mono', monospace";
    id.textContent = `ID: ${String(user.user_id || "").split("-")[0]}...`;

    userWrap.append(email, id);
    userCell.appendChild(userWrap);

    const kycCell = document.createElement("td");
    const status = document.createElement("span");
    status.className = "admin-badge admin-badge--success";
    status.textContent = user.kyc_status || "approved";
    kycCell.appendChild(status);

    const verifiedCell = document.createElement("td");
    verifiedCell.style.fontSize = "12px";
    verifiedCell.style.color = "var(--admin-text-muted)";
    verifiedCell.textContent = user.verified_at || "—";

    const walletCell = document.createElement("td");
    walletCell.style.color = "var(--admin-danger)";
    walletCell.style.fontSize = "12px";
    walletCell.style.fontWeight = "500";
    walletCell.textContent = "Missing wallet address";

    const actionCell = document.createElement("td");
    const button = document.createElement("button");
    button.className = "admin-btn admin-btn--primary";
    button.type = "button";
    button.style.padding = "6px 14px";
    button.style.fontSize = "12px";
    button.style.fontWeight = "600";
    button.textContent = "Force Sync";
    button.addEventListener("click", () => forceKycSync(user.user_id, user.email || "this user", button));
    actionCell.appendChild(button);

    row.append(userCell, kycCell, verifiedCell, walletCell, actionCell);
    return row;
}

function renderConfigPanel(config) {
    setText("cfg-network", config.network || "—");
    setText("cfg-chain-id", config.chain_id || "—");
    setText("cfg-factory", truncAddr(config.factory_address || ""));
    setText("cfg-registry", truncAddr(config.identity_registry || ""));
    setText("cfg-settlement", truncAddr(config.settlement_address || ""));
    setText("cfg-rpc", config.rpc_url || "—");
}

function renderTerminalLog(data) {
    const logEl = document.getElementById("event-log-terminal");
    if (!logEl) return;

    const ix = data.indexer || {};
    const st = data.settlement || {};
    const now = new Date().toISOString().replace("T", " ").substring(0, 19) + "Z";
    const lines = [];

    // Header
    lines.push(`<span style="color:#a3e635;">[POOOL Web3 Monitor]</span> System report at ${now}`);
    lines.push("");

    // Indexer section
    lines.push(`<span style="color:#38bdf8;">[EVENT INDEXER]</span>`);
    if (ix.enabled) {
        lines.push(`  Status: <span style="color:#4ade80;">ACTIVE</span> (poll every ${ix.poll_interval_secs}s, depth=${ix.confirmation_depth})`);
        lines.push(`  Last synced block: <span style="color:#fbbf24;">${ix.last_synced_block || 0}</span>`);
        if (ix.last_updated_at) {
            lines.push(`  Last update: ${ix.last_updated_at}`);
        }
        lines.push(`  On-chain balance entries: ${ix.total_balance_entries}`);
    } else {
        lines.push(`  Status: <span style="color:#f87171;">DISABLED</span> — Set chain_indexer_enabled=true in platform_settings`);
    }

    lines.push("");

    // Settlement section
    lines.push(`<span style="color:#38bdf8;">[SETTLEMENT WORKER]</span>`);
    if (st.enabled) {
        lines.push(`  Status: <span style="color:#4ade80;">ACTIVE</span>`);
    } else {
        lines.push(`  Status: <span style="color:#f87171;">DISABLED</span> — Set CHAIN_SETTLEMENT_ENABLED=true`);
    }
    lines.push(`  Pending trades: ${st.pending_trades || 0}`);
    lines.push(`  Submitted (in-flight): ${st.submitted_trades || 0}`);
    lines.push(`  Confirmed: <span style="color:#4ade80;">${st.confirmed_trades || 0}</span>`);
    if (st.failed_batches_last_24h > 0) {
        lines.push(`  Failed batches (24h): <span style="color:#f87171;">${st.failed_batches_last_24h}</span> ⚠️`);
    }
    if (st.last_batch_at) {
        lines.push(`  Last batch: ${st.last_batch_at}`);
    }
    lines.push(`  Avg batch size: ${st.avg_batch_size ? st.avg_batch_size.toFixed(1) : "N/A"} trades`);

    lines.push("");

    // Whitelist section
    const queueCount = (data.whitelist_queue || []).length;
    lines.push(`<span style="color:#38bdf8;">[KYC WHITELIST]</span>`);
    if (queueCount === 0) {
        lines.push(`  Queue: <span style="color:#4ade80;">EMPTY ✓</span> — All users synced`);
    } else {
        lines.push(`  Queue: <span style="color:#fbbf24;">${queueCount} users pending</span>`);
    }

    lines.push("");
    lines.push(`<span style="color:#64748b;">─── End of report ───</span>`);

    logEl.innerHTML = lines.join("<br>");
}

// ── Force KYC Sync ──

async function forceKycSync(userId, email, btn) {
    if (!confirm(`Force-sync KYC whitelist for ${email}?\n\nThis will generate a chain wallet address and mark the user for on-chain registration.`)) {
        return;
    }

    btn.disabled = true;
    btn.textContent = "Syncing...";

    try {
        const resp = await fetch(`/api/admin/blockchain/force-kyc-sync/${userId}`, {
            method: "POST",
        });

        if (!resp.ok) {
            const err = await resp.json().catch(() => ({}));
            throw new Error(err.error || `HTTP ${resp.status}`);
        }

        const result = await resp.json();
        btn.textContent = "✅ Synced";
        btn.classList.replace("admin-btn--primary", "admin-btn--success");
        btn.style.pointerEvents = "none";

        // Update the error column in the same row
        const row = btn.closest("tr");
        if (row) {
            const cells = row.querySelectorAll("td");
            if (cells.length >= 4) {
                const shortAddr = result.wallet_address
                    ? `${result.wallet_address.substring(0, 10)}...${result.wallet_address.substring(34)}`
                    : "—";
                cells[3].style.color = "var(--admin-success)";
                cells[3].style.fontSize = "12px";
                cells[3].style.fontFamily = "'SF Mono', monospace";
                cells[3].textContent = shortAddr;
            }
        }
    } catch (e) {
        console.error("Force sync failed:", e);
        btn.textContent = "Failed ✗";
        btn.classList.replace("admin-btn--primary", "admin-btn--danger");
        alert(`Force sync failed: ${e.message}`);
        setTimeout(() => {
            btn.disabled = false;
            btn.textContent = "Retry";
            btn.classList.replace("admin-btn--danger", "admin-btn--primary");
        }, 3000);
    }
}

// ── Utilities ──

function setText(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
}

function setTerminalLog(msg) {
    const el = document.getElementById("event-log-terminal");
    if (el) el.textContent = msg;
}

function truncAddr(addr) {
    if (!addr || addr.length < 20 || addr === "Not configured") return addr;
    return `${addr.substring(0, 10)}...${addr.substring(addr.length - 6)}`;
}
