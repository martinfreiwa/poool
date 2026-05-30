/**
 * Passkey / WebAuthn helpers for POOOL.
 *
 * Exposes two entry points:
 *   Passkeys.register(name)  — register a new passkey (authenticated user)
 *   Passkeys.login()         — sign in with a passkey (discoverable)
 *
 * Both return a Promise that resolves on success or rejects with an Error.
 */
(function (global) {
  "use strict";

  // ─── Base64url helpers ─────────────────────────────────────────

  function b64urlToBuffer(b64url) {
    const pad = b64url.length % 4 === 0 ? "" : "====".slice(b64url.length % 4);
    const b64 = (b64url + pad).replace(/-/g, "+").replace(/_/g, "/");
    const bin = atob(b64);
    const buf = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
    return buf.buffer;
  }

  function bufferToB64url(buf) {
    const bytes = new Uint8Array(buf);
    let bin = "";
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
  }

  // ─── Decode creation options from server ──────────────────────

  function prepareCreationOptions(opts) {
    const pk = opts.publicKey;
    pk.challenge = b64urlToBuffer(pk.challenge);
    pk.user.id = b64urlToBuffer(pk.user.id);
    if (pk.excludeCredentials) {
      pk.excludeCredentials = pk.excludeCredentials.map((c) => ({
        ...c,
        id: b64urlToBuffer(c.id),
      }));
    }
    return opts;
  }

  // ─── Decode request options from server ───────────────────────

  function prepareRequestOptions(opts, forceModal) {
    const pk = opts.publicKey;
    pk.challenge = b64urlToBuffer(pk.challenge);
    if (pk.allowCredentials) {
      pk.allowCredentials = pk.allowCredentials.map((c) => ({
        ...c,
        id: b64urlToBuffer(c.id),
      }));
    }
    // Server returns mediation:"conditional" for the discoverable flow.
    // For a button-click we want "optional" so the browser shows a modal.
    if (forceModal) opts.mediation = "optional";
    return opts;
  }

  // ─── Encode registration credential for server ────────────────

  function encodeRegistrationCredential(cred) {
    return {
      id: cred.id,
      rawId: bufferToB64url(cred.rawId),
      type: cred.type,
      response: {
        attestationObject: bufferToB64url(cred.response.attestationObject),
        clientDataJSON: bufferToB64url(cred.response.clientDataJSON),
        transports: cred.response.getTransports
          ? cred.response.getTransports()
          : [],
      },
    };
  }

  // ─── Encode authentication credential for server ──────────────

  function encodeAuthenticationCredential(cred) {
    const encoded = {
      id: cred.id,
      rawId: bufferToB64url(cred.rawId),
      type: cred.type,
      response: {
        authenticatorData: bufferToB64url(cred.response.authenticatorData),
        clientDataJSON: bufferToB64url(cred.response.clientDataJSON),
        signature: bufferToB64url(cred.response.signature),
      },
    };
    if (cred.response.userHandle) {
      encoded.response.userHandle = bufferToB64url(cred.response.userHandle);
    }
    return encoded;
  }

  // ─── API helpers ───────────────────────────────────────────────

  async function apiFetch(url, body) {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify(body),
    });
    const json = await res.json();
    if (!res.ok) throw new Error(json.error || "Request failed");
    return json;
  }

  // ─── Public API ────────────────────────────────────────────────

  const Passkeys = {
    supported() {
      return (
        typeof window !== "undefined" &&
        typeof window.PublicKeyCredential !== "undefined"
      );
    },

    /**
     * Register a new passkey for the currently logged-in user.
     * @param {string} [name] - Optional human-readable label, e.g. "My iPhone".
     */
    async register(name) {
      if (!this.supported()) throw new Error("Passkeys not supported in this browser.");

      // 1. Start
      const start = await apiFetch("/auth/passkey/register/start", {});
      const options = prepareCreationOptions(start.options);

      // 2. Browser prompt
      const cred = await navigator.credentials.create(options);
      if (!cred) throw new Error("Passkey creation cancelled.");

      // 3. Finish
      await apiFetch("/auth/passkey/register/finish", {
        challenge_id: start.challenge_id,
        credential: encodeRegistrationCredential(cred),
        name: name || null,
      });
    },

    /**
     * Sign in with a discoverable passkey.
     * On success the server sets the session cookie; caller should redirect.
     * @returns {Promise<{redirect: string}>}
     */
    async login() {
      if (!this.supported()) throw new Error("Passkeys not supported in this browser.");

      // 1. Start
      const start = await apiFetch("/auth/passkey/login/start", {});
      const options = prepareRequestOptions(start.options, true);

      // 2. Browser prompt
      const cred = await navigator.credentials.get(options);
      if (!cred) throw new Error("Passkey sign-in cancelled.");

      // 3. Finish
      return await apiFetch("/auth/passkey/login/finish", {
        challenge_id: start.challenge_id,
        credential: encodeAuthenticationCredential(cred),
      });
    },

    /**
     * Fetch the list of registered passkeys.
     * @returns {Promise<Array<{id, name, created_at}>>}
     */
    async list() {
      const res = await fetch("/auth/passkey/list", { credentials: "same-origin" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Failed to load passkeys");
      return json.passkeys || [];
    },

    /**
     * Delete a registered passkey by ID.
     * @param {string} id - UUID of the passkey_credentials row.
     */
    async delete(id) {
      const res = await fetch(`/auth/passkey/${encodeURIComponent(id)}`, {
        method: "DELETE",
        credentials: "same-origin",
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Failed to delete passkey");
    },
  };

  global.Passkeys = Passkeys;
})(window);
