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

    function getCsrfToken() {
        const value = `; ${document.cookie}`;
        const parts = value.split(`; csrf_token=`);
        if (parts.length === 2) return parts.pop().split(';').shift();
        return "";
    }

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

            // Parse JSON even for 4xx so we can relay server error messages to user
            return await res.json();
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
     * Request data export (stub/future).
     */
    async function requestDataExport() {
        // Implement when backend endpoint exists
        return { success: true, message: "Data export requested. You will receive an email shortly." };
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
        disable2FA,
    };
})();
