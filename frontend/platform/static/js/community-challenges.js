/**
 * Community Challenges UI (Phase 3 task 22).
 *
 * Backend auto-tracks progress on every action (see challenges.rs
 * increment_progress, fired from post create, review create, circle join,
 * KYC approval, login streak rollups). So the user-facing UX is about
 * surfacing progress and offering a "Take action" deeplink that drops the
 * user into the relevant flow for the challenge's requirement_type.
 *
 * Once a challenge is complete the row shows a completed state with a
 * checkmark + "Reward claimed" hint. XP is awarded server-side by the
 * background completion sweep, no client call required.
 */
(function () {
  "use strict";

  // Map each requirement_type to a deeplink and CTA label. Falls back to a
  // generic "View community" link when we don't have a matching flow.
  const ACTION_MAP = {
    buy_asset:      { label: "Browse commodities", href: "/commodities" },
    write_review:   { label: "Write a review",     href: "/community?tab=feed" },
    join_circle:    { label: "Find a circle",      href: "/community?tab=circle" },
    login_streak:   { label: "Keep logging in",    href: "/dashboard" },
    kyc_approved:   { label: "Complete KYC",       href: "/settings" },
  };

  function normalizeChallenge(raw) {
    const current = Number(raw.current_value ?? raw.user_progress ?? 0);
    const target = Number(raw.requirement_value ?? raw.target_value ?? 0);
    const progressPct = target > 0 ? Math.min(100, Math.max(0, (current / target) * 100)) : 0;

    return {
      id: raw.id,
      title: raw.title || "Untitled challenge",
      description: raw.description || "",
      requirement_type: raw.requirement_type || raw.challenge_type || "general",
      frequency: raw.frequency || "one_time",
      xp_reward: Number(raw.xp_reward || 0),
      badge_reward: raw.badge_reward || null,
      current: current,
      target: target,
      progress_pct: progressPct,
      end_time: raw.end_time || null,
      completed: Boolean(raw.is_completed),
      completed_at: raw.completed_at || null,
      // Submission-type extras (lazy-loaded on toggleSubmissions)
      submissions: [],
      submissionsLoaded: false,
      submissionsOpen: false,
    };
  }

  function csrfHeaders(extra) {
    const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = m ? decodeURIComponent(m[1]) : "";
    const out = extra || {};
    return token ? { ...out, "X-CSRF-Token": token } : out;
  }

  window.communityChallenges = function () {
    return {
      loading: true,
      error: "",
      challenges: [],
      // Submission modal state
      submitModalOpen: false,
      submitChallenge: null,
      submitContent: "",
      submitError: "",
      submitBusy: false,

      async init() {
        await this.loadChallenges();
      },

      async loadChallenges() {
        this.loading = true;
        this.error = "";

        try {
          const res = await fetch("/api/community/challenges", {
            credentials: "same-origin",
            headers: { Accept: "application/json" },
          });

          if (!res.ok) throw new Error("Failed to load challenges.");

          const data = await res.json();
          this.challenges = Array.isArray(data.challenges)
            ? data.challenges.map(normalizeChallenge)
            : [];
        } catch (err) {
          console.error("Failed to load community challenges", err);
          this.error = err && err.message ? err.message : "Failed to load challenges.";
          this.challenges = [];
        } finally {
          this.loading = false;
        }
      },

      challengeIcon(challenge) {
        const type = challenge.requirement_type;
        const svg = (paths) => `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`;
        // Lucide-derived stroke icons, all green via currentColor on the tile.
        if (type === "buy_asset" || type === "investment") {
          return svg('<line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>');
        }
        if (type === "join_circle" || type === "social") {
          return svg('<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>');
        }
        if (type === "write_review" || type === "learning") {
          return svg('<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>');
        }
        if (type === "login_streak") {
          return svg('<path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"/>');
        }
        if (type === "kyc_approved") {
          return svg('<path d="M9 12l2 2 4-4"/><path d="M21 12c0 1-5.5 8-9 8s-9-7-9-8 5.5-8 9-8 9 7 9 8z"/>');
        }
        // Goal / default
        return svg('<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>');
      },

      challengeProgressStyle(challenge) {
        return `width: ${challenge.progress_pct}%;`;
      },

      challengeAction(challenge) {
        return ACTION_MAP[challenge.requirement_type] || { label: "View community", href: "/community" };
      },

      challengeStateLabel(challenge) {
        if (challenge.completed) return "Completed";
        if (challenge.current === 0) return "Not started";
        return "In progress";
      },

      challengeFrequencyLabel(challenge) {
        if (challenge.frequency === "daily") return "Resets daily";
        if (challenge.frequency === "weekly") return "Resets weekly";
        return "One-time";
      },

      // ─── Submission flow (vote-based challenges) ─────────────────────

      async toggleSubmissions(ch) {
        ch.submissionsOpen = !ch.submissionsOpen;
        if (ch.submissionsOpen && !ch.submissionsLoaded) {
          await this.loadSubmissions(ch);
        }
      },

      async loadSubmissions(ch) {
        try {
          const res = await fetch(`/api/community/challenges/${ch.id}/submissions`, {
            credentials: "same-origin",
            headers: { Accept: "application/json" },
          });
          if (!res.ok) throw new Error(`Failed to load entries (${res.status}).`);
          const data = await res.json();
          ch.submissions = Array.isArray(data.submissions) ? data.submissions : [];
          ch.submissionsLoaded = true;
        } catch (err) {
          console.error("Failed to load submissions", err);
          ch.submissions = [];
          if (window.showToast) window.showToast(err.message || "Failed to load entries", "error");
        }
      },

      openSubmitModal(ch) {
        this.submitChallenge = ch;
        this.submitContent = "";
        this.submitError = "";
        this.submitModalOpen = true;
      },

      closeSubmitModal() {
        this.submitModalOpen = false;
        this.submitChallenge = null;
        this.submitContent = "";
        this.submitError = "";
        this.submitBusy = false;
      },

      async submitEntry() {
        const ch = this.submitChallenge;
        if (!ch) return;
        const content = (this.submitContent || "").trim();
        if (content.length < 1) {
          this.submitError = "Entry cannot be empty.";
          return;
        }
        this.submitBusy = true;
        this.submitError = "";
        try {
          const res = await fetch(`/api/community/challenges/${ch.id}/submit`, {
            method: "POST",
            credentials: "same-origin",
            headers: csrfHeaders({ "Content-Type": "application/json" }),
            body: JSON.stringify({ content }),
          });
          const body = await res.json().catch(() => ({}));
          if (!res.ok) throw new Error(body.error || `Submit failed (${res.status})`);
          if (window.showToast) window.showToast("Entry submitted!", "success");
          // Refresh entries so the new/updated submission is visible
          ch.submissionsOpen = true;
          ch.submissionsLoaded = false;
          await this.loadSubmissions(ch);
          this.closeSubmitModal();
        } catch (err) {
          console.error("submitEntry failed", err);
          this.submitError = err.message || "Submit failed.";
        } finally {
          this.submitBusy = false;
        }
      },

      async toggleVote(ch, submission) {
        try {
          const res = await fetch(`/api/community/challenges/submissions/${submission.id}/vote`, {
            method: "POST",
            credentials: "same-origin",
            headers: csrfHeaders({ "Content-Type": "application/json" }),
          });
          const body = await res.json().catch(() => ({}));
          if (!res.ok) throw new Error(body.error || `Vote failed (${res.status})`);
          submission.has_voted = Boolean(body.has_voted);
          submission.vote_count = Number(body.vote_count ?? submission.vote_count);
          // Re-sort so the highest-voted entry rises
          ch.submissions.sort((a, b) => (b.vote_count || 0) - (a.vote_count || 0));
        } catch (err) {
          console.error("toggleVote failed", err);
          if (window.showToast) window.showToast(err.message || "Vote failed", "error");
        }
      },
    };
  };
})();
