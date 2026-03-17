document.addEventListener("alpine:init", () => {
  Alpine.data("tierData", () => ({
    data: {
      tier_name: "Intro",
      invested_12m: 0,
      progress_pct: 0,
      tier_target: "Plus",
      tier_target_amount: 4000,
    },
    tiers: [],

    async initTier() {
      try {
        // Fetch user rewards overview
        const rewardsRes = await fetch("/api/rewards");
        if (rewardsRes.ok) {
          const rewardsData = await rewardsRes.json();
          this.data = {
            ...this.data,
            ...rewardsData,
          };
        }

        // Fetch all tiers definition
        const tiersRes = await fetch("/api/rewards/tiers");
        if (tiersRes.ok) {
          this.tiers = await tiersRes.json();
        }
      } catch (err) {
        console.error('Failed to load tier data:', err);
        if (window.Sentry) Sentry.captureException(err);
      }
    },

    getTierOrder(tierName) {
      const tiersList = ["Intro", "Plus", "Pro", "Elite", "Premium"];
      let idx = tiersList.indexOf(tierName);
      // Case-insensitive fallback
      if (idx === -1) {
        const lower = tierName.toLowerCase();
        idx = tiersList.findIndex((t) => t.toLowerCase() === lower);
      }
      return idx === -1 ? 0 : idx;
    },

    getStepperState(stepName) {
      const currentOrd = this.getTierOrder(this.data.tier_name);
      const stepOrd = this.getTierOrder(stepName);

      if (currentOrd === stepOrd) return "active";
      if (currentOrd > stepOrd) return "past";
      return "inactive";
    },

    getStepperIconState(stepName) {
      const state = this.getStepperState(stepName);
      if (state === "active") return "active";
      if (state === "past") return "past";
      return "";
    },

    getTierCardClass(tierName) {
      const mapping = {
        Intro: "intro-style",
        Plus: "plus-style",
        Pro: "pro-style",
        Elite: "elite-style",
        Premium: "premium-style",
      };
      const lower = tierName.toLowerCase();
      const key = Object.keys(mapping).find((k) => k.toLowerCase() === lower);
      return mapping[key] || "intro-style";
    },

    getTierBadgeClass(tierName) {
      const mapping = {
        Intro: "intro",
        Plus: "plus",
        Pro: "pro",
        Elite: "elite",
        Premium: "premium",
      };
      const lower = tierName.toLowerCase();
      const key = Object.keys(mapping).find((k) => k.toLowerCase() === lower);
      return mapping[key] || "intro";
    },
  }));
});
