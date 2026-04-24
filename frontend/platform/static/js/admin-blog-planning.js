(function () {
  "use strict";

  const page = document.body?.dataset.blogPlanningPage;
  if (!page) return;

  const storageKey = `poool.admin.blog.${page}`;
  const byId = (id) => document.getElementById(id);

  function fields() {
    return Array.from(document.querySelectorAll("input[id], textarea[id], select[id]"))
      .filter((el) => !["blog-plan-output"].includes(el.id));
  }

  function readForm() {
    return fields().reduce((data, el) => {
      data[el.id] = el.value;
      return data;
    }, {});
  }

  function writeForm(data) {
    Object.entries(data || {}).forEach(([id, value]) => {
      const el = byId(id);
      if (el) el.value = value;
    });
  }

  function splitLines(value) {
    return String(value || "")
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  }

  function clampTone(value) {
    const parsed = Number.parseFloat(value);
    if (Number.isNaN(parsed)) return 0;
    return Math.max(0, Math.min(1, parsed));
  }

  function personaPayload(data) {
    return {
      name: "poool",
      description: "Trust-led investment voice for fractional property investment in Bali",
      brand: data["persona-brand"] || "POOOL",
      industry: data["persona-industry"] || "",
      audience: data["persona-audience"] || "",
      mission: data["persona-mission"] || "",
      tone_dimensions: {
        funny_serious: clampTone(data["persona-tone-serious"]),
        formal_casual: clampTone(data["persona-tone-casual"]),
        respectful_irreverent: clampTone(data["persona-tone-irreverent"]),
        enthusiastic_matter_of_fact: clampTone(data["persona-tone-matter"]),
      },
      readability: {
        flesch_grade_min: 8,
        flesch_grade_max: 10,
        flesch_ease_min: 50,
        flesch_ease_max: 60,
      },
      style: {
        sentence_length_mean: 17,
        sentence_length_std: 6,
        contraction_frequency: 0.35,
        passive_voice_max_pct: 8,
        vocabulary_tier: data["persona-vocabulary"] || "professional",
        summary_label: data["persona-summary-label"] || "Key Takeaways",
      },
      voice_samples: [],
      do: splitLines(data["persona-do"]),
      dont: splitLines(data["persona-dont"]),
    };
  }

  function strategyMarkdown(data) {
    return [
      "# Blog Strategy: POOOL",
      "",
      "## Positioning",
      data["strategy-positioning"] || "",
      "",
      "## Audience Segments",
      ...splitLines(data["strategy-audiences"]).map((line) => `- ${line}`),
      "",
      "## Content Pillars",
      ...splitLines(data["strategy-pillars"]).map((line) => `- ${line}`),
      "",
      "## Month 1",
      ...splitLines(data["strategy-month-one"]).map((line) => `- ${line}`),
      "",
      "## Month 2",
      ...splitLines(data["strategy-month-two"]).map((line) => `- ${line}`),
      "",
      "## Month 3",
      ...splitLines(data["strategy-month-three"]).map((line) => `- ${line}`),
      "",
      "## Quality Rules",
      ...splitLines(data["strategy-quality"]).map((line) => `- ${line}`),
    ].join("\n");
  }

  function outputValue() {
    const data = readForm();
    if (page === "persona") return JSON.stringify(personaPayload(data), null, 2);
    return strategyMarkdown(data);
  }

  function updateOutput() {
    const output = byId("blog-plan-output");
    if (output) output.value = outputValue();
  }

  function showAlert(message) {
    const alert = byId("blog-plan-alert");
    if (!alert) return;
    alert.textContent = message;
    alert.style.display = "block";
    window.setTimeout(() => {
      alert.style.display = "none";
    }, 2500);
  }

  function saveDraft() {
    window.localStorage.setItem(storageKey, JSON.stringify(readForm()));
    showAlert("Draft saved in this browser.");
  }

  function resetDraft() {
    window.localStorage.removeItem(storageKey);
    window.location.reload();
  }

  async function copyOutput() {
    const value = outputValue();
    try {
      await navigator.clipboard.writeText(value);
      showAlert(page === "persona" ? "Persona JSON copied." : "Strategy Markdown copied.");
    } catch (_error) {
      const output = byId("blog-plan-output");
      if (output) {
        output.focus();
        output.select();
      }
      showAlert("Copy blocked by browser. The output is selected.");
    }
  }

  function init() {
    try {
      const saved = JSON.parse(window.localStorage.getItem(storageKey) || "null");
      if (saved) writeForm(saved);
    } catch (_error) {
      window.localStorage.removeItem(storageKey);
    }

    fields().forEach((el) => {
      el.addEventListener("input", updateOutput);
      el.addEventListener("change", updateOutput);
    });

    const form = byId(page === "persona" ? "blog-persona-form" : "blog-strategy-form");
    form?.addEventListener("submit", (event) => {
      event.preventDefault();
      saveDraft();
      updateOutput();
    });

    byId("blog-plan-reset-btn")?.addEventListener("click", resetDraft);
    byId("blog-plan-copy-btn")?.addEventListener("click", copyOutput);
    updateOutput();
  }

  init();
})();
