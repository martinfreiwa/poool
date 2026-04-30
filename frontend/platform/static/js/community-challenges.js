(function () {
  "use strict";

  function normalizeChallenge(raw) {
    const current = Number(raw.current_value ?? raw.user_progress ?? 0);
    const target = Number(raw.requirement_value ?? raw.target_value ?? 0);
    const progressPct = target > 0 ? Math.min(100, Math.max(0, (current / target) * 100)) : 0;

    return {
      id: raw.id,
      title: raw.title || "Untitled challenge",
      description: raw.description || "",
      challenge_type: raw.challenge_type || raw.requirement_type || "general",
      xp_reward: Number(raw.xp_reward || 0),
      user_progress: current,
      target_value: target,
      progress_pct: progressPct,
      end_time: raw.end_time || null,
      is_completed: Boolean(raw.is_completed),
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

          if (!res.ok) {
            throw new Error("Failed to load challenges.");
          }

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
        const type = challenge.challenge_type;
        if (type === "buy_asset" || type === "investment") return "USD";
        if (type === "join_circle" || type === "social") return "Group";
        if (type === "write_review" || type === "learning") return "Read";
        if (type === "login_streak") return "Streak";
        return "Goal";
      },

      challengeProgressStyle(challenge) {
        return `width: ${challenge.progress_pct}%; background: #0000FF; height: 100%; border-radius: 4px;`;
      },
    };
  };
})();
