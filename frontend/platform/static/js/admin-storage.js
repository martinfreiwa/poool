/**
 * Admin Storage Analytics
 * Drives the /admin/storage.html page via Alpine.js.
 * Fetches real data from GET /api/admin/storage.
 */

document.addEventListener("alpine:init", () => {
    Alpine.data("storageAnalytics", () => ({
        loading: true,
        error: null,
        data: null,

        // ── Derived convenience accessors ──────────────────────────────────────
        get bucket() {
            return this.data?.bucket ?? "—";
        },

        // ── Lifecycle ──────────────────────────────────────────────────────────
        async init() {
            await this.load();
        },

        async reload() {
            this.loading = true;
            this.error = null;
            await this.load();
        },

        async load() {
            try {
                const resp = await fetch("/api/admin/storage");
                if (!resp.ok) {
                    const body = await resp.json().catch(() => ({}));
                    throw new Error(body.error || `HTTP ${resp.status}`);
                }
                this.data = await resp.json();
            } catch (e) {
                this.error = e.message || "Failed to load storage analytics.";
            } finally {
                this.loading = false;
            }
        },

        // ── Helpers ────────────────────────────────────────────────────────────

        /** Format bytes to human-readable (KB / MB / GB). */
        formatGB(bytes) {
            if (!bytes) return "0 B";
            if (bytes < 1_048_576) return (bytes / 1024).toFixed(1) + " KB";
            if (bytes < 1_073_741_824) return (bytes / 1_048_576).toFixed(1) + " MB";
            return (bytes / 1_073_741_824).toFixed(3) + " GB";
        },

        /** Return count for a given document status (pending / approved / rejected). */
        getStatus(status) {
            const row = this.data?.breakdown_by_status?.find((r) => r.status === status);
            return row?.count ?? 0;
        },

        /** Bar chart — calculate bar height % relative to max monthly uploads. */
        barHeight(uploads) {
            const maxUploads = Math.max(
                ...(this.data?.monthly_trend ?? []).map((r) => r.uploads),
                1,
            );
            return Math.max((uploads / maxUploads) * 100, 2);
        },

        /** Type breakdown — calculate progress bar width relative to the most common type. */
        typeWidth(count) {
            const maxCount = Math.max(
                ...(this.data?.breakdown_by_type ?? []).map((r) => r.count),
                1,
            );
            return Math.max((count / maxCount) * 100, 3);
        },

        /** Friendly display name for document types. */
        typeName(type) {
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
                asset_expose: "Property Exposé",
                asset_appraisal: "Appraisal",
                asset_financial: "Financial Model",
                asset_floor_plan: "Floor Plan",
                asset_other: "Asset Document",
                other: "Other",
            };
            
            if (names[type]) return names[type];
            
            // Format fallback (e.g. "asset_custom_doc" -> "Asset Custom Doc")
            return type.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
        },

        /** Emoji icon for document type. */
        typeEmoji(type) {
            const icons = {
                passport: "🛂",
                national_id: "🪪",
                driving_licence: "🚗",
                proof_of_address: "🏠",
                asset_image: "🖼️",
                user_avatar: "👤",
                asset_proof_of_title: "📜",
                asset_legal_basis: "⚖️",
                asset_building_permit: "🏗️",
                asset_site_plan: "🗺️",
                asset_tax_npwp: "🧾",
                asset_tax_pbb: "🧾",
                asset_tax_bphtb: "🧾",
                asset_license_nib: "🏢",
                asset_id_card: "🪪",
                asset_owner_npwp: "🧾",
                asset_expose: "📊",
                asset_appraisal: "🔍",
                asset_financial: "📈",
                asset_floor_plan: "📐",
                asset_other: "📄",
                other: "📄",
            };
            return icons[type] ?? "📄";
        },

        /** Color palette for type breakdown bars (cycles). */
        typeColor(index) {
            const palette = [
                "#3b82f6", // blue
                "#8b5cf6", // violet
                "#10b981", // emerald
                "#f59e0b", // amber
                "#ef4444", // red
                "#06b6d4", // cyan
            ];
            return palette[index % palette.length];
        },

        /** Background tint for status icon in recent uploads. */
        statusBgColor(status) {
            if (status === "approved") return "rgba(16, 185, 129, 0.1)";
            if (status === "rejected") return "rgba(239, 68, 68, 0.1)";
            return "rgba(245, 158, 11, 0.1)";
        },
    }));
});
