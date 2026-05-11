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
    };
  }

  window.communityChallenges = function () {
    return {
      loading: true,
      error: "",
      challenges: [],

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
        if (type === "buy_asset" || type === "investment") return "USD";
        if (type === "join_circle" || type === "social") return "Group";
        if (type === "write_review" || type === "learning") return "Read";
        if (type === "login_streak") return "Streak";
        if (type === "kyc_approved") return "Verify";
        return "Goal";
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
    };
  };
})();
