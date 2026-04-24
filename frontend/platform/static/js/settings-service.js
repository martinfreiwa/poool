/**
 * SettingsDataService  –  Phase 2: Backend-Logik & Data Fetching
 *
 * Responsibilities:
 *   - All API calls to the settings endpoints
 *   - Input validation (client-side pre-flight)
 *   - Returns typed, normalized response objects
 *
 * Endpoints:
 *   GET  /api/settings                → SettingsResponse
 *   POST /api/settings/profile        → ApiResponse
 *   POST /api/settings/preferences    → ApiResponse
 *   POST /api/settings/email          → ApiResponse
 *   POST /api/settings/password       → ApiResponse
 *   POST /api/settings/phone          → ApiResponse
 *
 * @typedef {Object} SettingsResponse
 * @property {string}      email
 * @property {string|null} first_name
 * @property {string|null} last_name
 * @property {string|null} phone_number
 * @property {string|null} country
 * @property {string}      timezone
 * @property {string}      role
 * @property {string}      language
 * @property {string}      currency
 * @property {string|null} date_of_birth
 * @property {string|null} nationality
 * @property {string|null} address_line_1
 * @property {string|null} address_line_2
 * @property {string|null} city
 * @property {string|null} state_province
 * @property {string|null} postal_code
 * @property {string|null} tax_id
 * @property {string|null} kyc_status
 * @property {boolean}     email_verified
 * @property {boolean}     email_notifications
 * @property {boolean}     push_notifications
 * @property {boolean}     totp_enabled
 *
 * @typedef {Object} ApiResponse
 * @property {boolean} success
 * @property {string}  message
 */
const SettingsDataService = (function () {
    "use strict";

    // ─── Core API Helper ─────────────────────────────────────────

    /**
     * Internal fetch wrapper with auth handling.
     * @param {string} url
     * @param {'GET'|'POST'} method
     * @param {object|null} [body]
     * @returns {Promise<any|null>} parsed JSON or null on network error
     */
    async function apiFetch(url, method, body) {
        const opts = {
            method,
            credentials: "same-origin",
            headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
        };
        if (body) opts.body = JSON.stringify(body);

        try {
            const res = await fetch(url, opts);

            if (res.status === 401) {
                window.location.href = "/auth/login";
                return null;
            }

            // Check if response is JSON before parsing
            const contentType = res.headers.get("content-type") || "";
            if (contentType.includes("application/json")) {
                return await res.json();
            }

            // Non-JSON response (e.g. bare 403/500 status page)
            const text = await res.text();
            console.warn(`Settings API non-JSON response [${method}] ${url}: ${res.status} ${text.substring(0, 200)}`);
            return { success: false, message: `Server error (${res.status}). Please refresh and try again.` };
        } catch (err) {
            console.error(`Settings API error [${method}] ${url}:`, err);
            if (typeof Sentry !== 'undefined') Sentry.captureException(err);
            return { success: false, message: "Network error. Please try again later." };
        }
    }

    /**
     * Multipart upload helper.
     */
    async function apiUpload(url, file) {
        const formData = new FormData();
        formData.append("file", file);

        try {
            const res = await fetch(url, {
                method: "POST",
                credentials: "same-origin",
                headers: { "X-CSRF-Token": getCsrfToken() },
                body: formData,
            });

            if (res.status === 401) {
                window.location.href = "/auth/login";
                return null;
            }

            return await res.json();
        } catch (err) {
            console.error(`Upload error ${url}:`, err);
            return { error: "Network error during upload." };
        }
    }

    // ─── Public Methods ───────────────────────────────────────────

    /**
     * Fetch all settings data for the logged-in user.
     * @returns {Promise<SettingsResponse|null>}
     */
    async function getSettings() {
        return apiFetch("/api/settings", "GET");
    }

    /**
     * Save profile data (name, country, timezone, etc).
     * @param {Object} data Profile data object
     * @returns {Promise<ApiResponse|null>}
     */
    async function saveProfile(data) {
        return apiFetch("/api/settings/profile", "POST", data);
    }

    /**
     * Save notification preferences.
     * @param {{ email_notifications: boolean, push_notifications: boolean }} data
     */
    async function saveNotifications(data) {
        return apiFetch("/api/settings/notifications", "POST", data);
    }

    /**
     * Disable Two-factor authentication.
     */
    async function disable2FA() {
        return apiFetch("/api/settings/2fa/disable", "POST");
    }

    /**
     * Upload avatar image to GCS.
     * @param {File} file
     */
    async function uploadAvatar(file) {
        return apiUpload("/api/upload/avatar", file);
    }

    /**
     * Save preferences (language, currency).
     * @param {{ language: string, currency: string }} data
     * @returns {Promise<ApiResponse|null>}
     */
    async function savePreferences(data) {
        return apiFetch("/api/settings/preferences", "POST", data);
    }

    /**
     * Save leaderboard settings.
     * @param {{ visible: boolean, show_avatar: boolean, display_name: string }} data
     * @returns {Promise<ApiResponse|null>}
     */
    async function saveLeaderboard(data) {
        return apiFetch("/api/settings/leaderboard", "POST", data);
    }

    /**
     * Change the user's email address.
     * Validates fields client-side before sending.
     * @param {string} newEmail
     * @param {string} currentPassword
     * @returns {Promise<{success: boolean, message: string}|null>}
     */
    async function changeEmail(newEmail, currentPassword) {
        if (!newEmail || !currentPassword) {
            return { success: false, message: "Please fill in all fields." };
        }
        if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(newEmail)) {
            return { success: false, message: "Please enter a valid email address." };
        }
        return apiFetch("/api/settings/email", "POST", {
            new_email: newEmail,
            current_password: currentPassword,
        });
    }

    /**
     * Change the user's password with confirmation validation.
     * @param {string} current
     * @param {string} newPw
     * @param {string} confirm
     * @returns {Promise<{success: boolean, message: string}|null>}
     */
    async function changePassword(current, newPw, confirm) {
        if (!current || !newPw || !confirm) {
            return { success: false, message: "Please fill in all fields." };
        }
        if (newPw !== confirm) {
            return { success: false, message: "New passwords do not match." };
        }
        if (newPw.length < 8) {
            return { success: false, message: "Password must be at least 8 characters." };
        }
        return apiFetch("/api/settings/password", "POST", {
            current_password: current,
            new_password: newPw,
            confirm_password: confirm,
        });
    }

    /**
     * Change the user's phone number.
     * @param {string} newPhone
     * @returns {Promise<ApiResponse|null>}
     */
    async function changePhone(newPhone) {
        return apiFetch("/api/settings/phone", "POST", { new_phone: newPhone || "" });
    }

    /**
     * Request a GDPR Art.15/20 data export.
     * @returns {Promise<ApiResponse|null>}
     */
    async function requestDataExport() {
        return apiFetch("/api/settings/export-data", "POST");
    }

    /**
     * Delete the user's account (GDPR Art.17).
     * @param {string} password
     * @param {string} confirmPhrase — must equal "DELETE"
     * @returns {Promise<ApiResponse|null>}
     */
    async function deleteAccount(password, confirmPhrase) {
        if (!password) return { success: false, message: "Password required." };
        if (confirmPhrase !== "DELETE") return { success: false, message: "Type DELETE to confirm." };
        return apiFetch("/api/settings/delete-account", "POST", {
            password,
            confirm: confirmPhrase,
        });
    }

    /**
     * Save social links (personal profile).
     * @param {{twitter?:string, linkedin?:string, instagram?:string, telegram?:string, discord?:string, website?:string}} data
     */
    async function saveSocialLinks(data) {
        return apiFetch("/api/settings/social", "POST", data);
    }

    /**
     * Save developer company profile (developer role only).
     * @param {{company_name?:string, description?:string}} data
     */
    async function saveDeveloperProfile(data) {
        return apiFetch("/api/settings/developer/profile", "POST", data);
    }

    /**
     * Save developer public links (developer role only).
     */
    async function saveDeveloperLinks(data) {
        return apiFetch("/api/settings/developer/links", "POST", data);
    }

    /**
     * Upload developer logo (developer role only).
     */
    async function uploadDeveloperLogo(file) {
        return apiUpload("/api/upload/developer-logo", file);
    }

    /**
     * List active sessions.
     */
    async function listSessions() {
        return apiFetch("/api/settings/sessions", "GET");
    }

    /**
     * Revoke a specific session.
     * @param {string} sessionId
     */
    async function revokeSession(sessionId) {
        return apiFetch(`/api/settings/sessions/${encodeURIComponent(sessionId)}`, "DELETE");
    }

    /**
     * Revoke all sessions except the current one.
     */
    async function revokeOtherSessions() {
        return apiFetch("/api/settings/sessions/revoke-others", "POST");
    }

    /**
     * List OAuth connections.
     */
    async function listOAuthConnections() {
        return apiFetch("/api/settings/oauth", "GET");
    }

    /**
     * Initiate OAuth link flow (returns redirect URL).
     * @param {'google'|'facebook'|'apple'|'github'} provider
     */
    async function linkOAuth(provider) {
        return apiFetch(`/api/settings/oauth/${encodeURIComponent(provider)}/link`, "POST");
    }

    /**
     * Unlink an OAuth connection.
     * @param {string} connectionId
     */
    async function unlinkOAuth(connectionId) {
        return apiFetch(`/api/settings/oauth/${encodeURIComponent(connectionId)}`, "DELETE");
    }

    /**
     * List saved payment methods.
     */
    async function listPaymentMethods() {
        return apiFetch("/api/settings/payment-methods", "GET");
    }

    /**
     * Delete a saved payment method.
     */
    async function deletePaymentMethod(methodId) {
        return apiFetch(`/api/settings/payment-methods/${encodeURIComponent(methodId)}`, "DELETE");
    }

    // ─── Exports ─────────────────────────────────────────────────
    return {
        getSettings,
        saveProfile,
        savePreferences,
        saveNotifications,
        uploadAvatar,
        changeEmail,
        changePassword,
        changePhone,
        requestDataExport,
        deleteAccount,
        disable2FA,
        saveLeaderboard,
        saveSocialLinks,
        saveDeveloperProfile,
        saveDeveloperLinks,
        uploadDeveloperLogo,
        listSessions,
        revokeSession,
        revokeOtherSessions,
        listOAuthConnections,
        linkOAuth,
        unlinkOAuth,
        listPaymentMethods,
        deletePaymentMethod,
    };
})();
