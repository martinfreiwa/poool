/**
 * community-amas.js - Expert AMAs tab logic.
 * Targets the current community_ama.html fragment contract.
 */
(function () {
  "use strict";

  // Lightweight toast helper — falls back to window.alert only if toast.js
  // failed to load. Default kind is 'error'.
  const toast = (msg, kind) => (typeof window.showToast === 'function')
    ? window.showToast(msg, kind || 'error')
    : window.alert(msg);

  const STATUS_LABELS = {
    live: "LIVE NOW",
    accepting_questions: "QUESTIONS OPEN",
    scheduled: "UPCOMING",
    closed: "CLOSED",
    archived: "ARCHIVED",
  };

  function byId(id) {
    return document.getElementById(id);
  }

  function setText(id, value) {
    const el = byId(id);
    if (el) el.textContent = value == null || value === "" ? "-" : String(value);
  }

  function show(el, display) {
    if (el) el.style.display = display;
  }

  function timeAgo(dateStr) {
    const date = new Date(dateStr).getTime();
    if (!Number.isFinite(date)) return "";
    const diff = Math.floor((Date.now() - date) / 1000);
    if (diff < 60) return "just now";
    if (diff < 3600) return Math.floor(diff / 60) + "m ago";
    if (diff < 86400) return Math.floor(diff / 3600) + "h ago";
    if (diff < 604800) return Math.floor(diff / 86400) + "d ago";
    return new Date(dateStr).toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  function formatDateTime(dateStr) {
    if (!dateStr) return "Date TBA";
    const date = new Date(dateStr);
    if (Number.isNaN(date.getTime())) return "Date TBA";
    return date.toLocaleDateString("en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    }) + " - " + date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      timeZoneName: "short",
    });
  }

  function showToast(message) {
    if (window.showToast) {
      window.showToast(message);
      return;
    }

    const toast = document.createElement("div");
    toast.className = "community-toast";
    toast.setAttribute("role", "status");
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(function () { toast.remove(); }, 4000);
  }

  window.initCommunityAmas = function () {
    const loading = byId("ama-loading");
    const empty = byId("ama-empty");
    const content = byId("ama-content");
    const questionsList = byId("ama-questions-list");
    const questionInput = byId("ama-question-input");
    const submitBtn = byId("ama-question-submit-btn");

    if (!loading || !empty || !content || !questionsList || !questionInput || !submitBtn) return;

    let activeAmaId = null;

    function setLoading() {
      show(loading, "block");
      show(empty, "none");
      show(content, "none");
      submitBtn.disabled = true;
    }

    function setEmpty(message) {
      show(loading, "none");
      show(content, "none");
      show(empty, "block");
      const messageEl = empty.querySelector("p");
      if (messageEl && message) messageEl.textContent = message;
      activeAmaId = null;
      submitBtn.disabled = true;
    }

    function setContent() {
      show(loading, "none");
      show(empty, "none");
      show(content, "block");
    }

    function renderQuestion(q) {
      const card = document.createElement("div");
      card.className = "ds-card community-ama-question";

      const topRow = document.createElement("div");
      topRow.className = "community-ama-question__top";

      const question = document.createElement("p");
      question.className = "community-ama-question__text";
      question.textContent = q.question || "";
      topRow.appendChild(question);

      if (q.is_featured) {
        const badge = document.createElement("span");
        badge.className = "ds-badge community-ama-question__featured";
        badge.textContent = "Featured";
        topRow.appendChild(badge);
      }

      card.appendChild(topRow);

      if (q.answer) {
        const answer = document.createElement("div");
        answer.className = "community-ama-question__answer";

        const answerLabel = document.createElement("div");
        answerLabel.className = "community-ama-question__answer-label";
        answerLabel.textContent = "Expert Answer";
        answer.appendChild(answerLabel);

        const answerText = document.createElement("p");
        answerText.className = "community-ama-question__answer-text";
        answerText.textContent = q.answer;
        answer.appendChild(answerText);

        card.appendChild(answer);
      }

      const bottom = document.createElement("div");
      bottom.className = "community-ama-question__bottom";

      const upvoteBtn = document.createElement("button");
      upvoteBtn.type = "button";
      upvoteBtn.className = "community-ama-question__upvote" + (q.user_has_upvoted ? " community-ama-question__upvote--active" : "");
      upvoteBtn.setAttribute("aria-pressed", q.user_has_upvoted ? "true" : "false");
      upvoteBtn.textContent = "Upvote " + Number(q.upvote_count || 0);
      upvoteBtn.addEventListener("click", function () {
        handleUpvote(q.id);
      });
      bottom.appendChild(upvoteBtn);

      const time = document.createElement("span");
      time.className = "community-ama-question__time";
      time.textContent = timeAgo(q.created_at);
      bottom.appendChild(time);

      card.appendChild(bottom);
      return card;
    }

    function renderQuestions(questions) {
      questionsList.replaceChildren();
      if (!questions.length) {
        const emptyQuestions = document.createElement("div");
        emptyQuestions.className = "community-loading-state";
        emptyQuestions.textContent = "No questions submitted yet. Be the first to ask.";
        questionsList.appendChild(emptyQuestions);
        return;
      }

      questions.forEach(function (q) {
        questionsList.appendChild(renderQuestion(q));
      });
    }

    function renderHero(ama) {
      const status = ama.status || "scheduled";
      setText("ama-status-badge", STATUS_LABELS[status] || "UPCOMING");
      setText("ama-title", ama.title);
      setText("ama-description", ama.description || "");
      setText("ama-date-time", formatDateTime(ama.scheduled_at));
      setText("ama-expert-name", ama.expert_name || "Expert");
      setText("ama-expert-title", ama.expert_title || "Expert");

      const avatar = byId("ama-expert-avatar");
      if (avatar) {
        const name = ama.expert_name || "Expert";
        avatar.textContent = name.trim().charAt(0).toUpperCase() || "E";
      }

      const acceptsQuestions = status === "accepting_questions" || status === "live";
      submitBtn.disabled = !acceptsQuestions;
      submitBtn.textContent = acceptsQuestions ? "Submit" : "Questions Closed";
      questionInput.disabled = !acceptsQuestions;
      questionInput.placeholder = acceptsQuestions ? "Type your question..." : "Questions are not open for this AMA.";
    }

    async function loadAmaDetail(amaId) {
      try {
        const res = await fetch("/api/community/amas/" + encodeURIComponent(amaId), {
          credentials: "same-origin",
          headers: { Accept: "application/json" },
        });
        if (!res.ok) throw new Error("Failed to load AMA questions.");

        const data = await res.json();
        renderQuestions(Array.isArray(data.questions) ? data.questions : []);
      } catch (err) {
        console.error("Failed to load AMA detail", err);
        renderQuestions([]);
      }
    }

    async function loadAmas() {
      setLoading();
      try {
        const res = await fetch("/api/community/amas", {
          credentials: "same-origin",
          headers: { Accept: "application/json" },
        });

        if (!res.ok) throw new Error("Failed to load AMAs.");

        const data = await res.json();
        const amas = Array.isArray(data.amas) ? data.amas : [];
        if (!amas.length) {
          setEmpty();
          return;
        }

        const activeStatuses = ["live", "accepting_questions", "scheduled"];
        const active = amas.find(function (ama) {
          return activeStatuses.includes(ama.status);
        }) || amas[0];

        activeAmaId = active.id;
        setContent();
        renderHero(active);
        await loadAmaDetail(active.id);
      } catch (err) {
        console.error("Failed to load AMAs", err);
        setEmpty("We could not load Expert AMAs right now. Please try again later.");
      }
    }

    async function handleUpvote(questionId) {
      if (!activeAmaId || !questionId) return;

      try {
        const res = await fetch(
          "/api/community/amas/" + encodeURIComponent(activeAmaId) + "/questions/" + encodeURIComponent(questionId) + "/upvote",
          {
            method: "POST",
            credentials: "same-origin",
            headers: { "Content-Type": "application/json" },
          }
        );
        if (!res.ok) throw new Error(await res.text());
        await loadAmaDetail(activeAmaId);
      } catch (err) {
        console.error("AMA upvote failed", err);
        toast("Failed to upvote question: " + (err && err.message ? err.message : "Unknown error"));
      }
    }

    window.submitQuestion = async function () {
      const question = questionInput.value.trim();

      if (question.length < 10) {
        toast("Your question must be at least 10 characters.");
        return;
      }
      if (question.length > 500) {
        toast("Question is too long. Max 500 characters.");
        return;
      }
      if (!activeAmaId) {
        toast("No active AMA to submit to.");
        return;
      }

      const originalText = submitBtn.textContent;
      submitBtn.disabled = true;
      submitBtn.textContent = "Submitting...";

      try {
        const res = await fetch("/api/community/amas/" + encodeURIComponent(activeAmaId) + "/questions", {
          method: "POST",
          credentials: "same-origin",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ question }),
        });

        if (!res.ok) throw new Error(await res.text());

        questionInput.value = "";
        await loadAmaDetail(activeAmaId);
        showToast("Question submitted. The community can upvote it now.");
      } catch (err) {
        toast("Failed to submit question: " + (err && err.message ? err.message : "Unknown error"));
      } finally {
        submitBtn.disabled = false;
        submitBtn.textContent = originalText === "Submitting..." ? "Submit" : originalText;
      }
    };

    window.loadAmas = loadAmas;
    loadAmas();
  };

  document.addEventListener("DOMContentLoaded", window.initCommunityAmas);
  document.body.addEventListener("htmx:afterSwap", function (event) {
    if (event.target && event.target.id === "community-content-area") {
      window.initCommunityAmas();
    }
  });
})();
