/**
 * User-Facing Support Page JS
 * Loads the user's own tickets, allows creating new ones, viewing details,
 * replying to tickets, reopening resolved tickets, and searching FAQ.
 */
(function () {
  "use strict";

  let allTickets = [];
  let currentFilter = "";
  let faqDebounceTimeout;

  document.addEventListener("DOMContentLoaded", () => {
    loadMyTickets();

    // New ticket form
    document
      .getElementById("support-form")
      ?.addEventListener("submit", handleSubmit);

    // Filter tabs
    document.querySelectorAll(".support-tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        document
          .querySelectorAll(".support-tab")
          .forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");
        currentFilter = tab.dataset.filter || "";
        renderTickets();
      });
    });

    // FAQ search
    const faqSearch = document.getElementById("faq-search");
    if (faqSearch) {
      faqSearch.addEventListener("input", filterFAQ);
    }

    // Dynamic FAQ suggestion on ticket subject
    const ticketSubject = document.getElementById("ticket-subject");
    if (ticketSubject) {
      ticketSubject.addEventListener("input", suggestFAQ);
    }

    // FAQ accordion toggle
    document.querySelectorAll(".faq-question").forEach((q) => {
      q.addEventListener("click", () => {
        const item = q.closest(".faq-item");
        item.classList.toggle("open");
      });
    });

    // FAQ category filter tabs
    document.querySelectorAll(".faq-cat-tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        document
          .querySelectorAll(".faq-cat-tab")
          .forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");
        filterFAQ();
      });
    });

    // Priority time update
    const prioritySelect = document.getElementById("ticket-priority");
    if (prioritySelect) {
      prioritySelect.addEventListener("change", (e) => {
        const val = e.target.value;
        const timeEl = document.getElementById("expected-response-time");
        if(!timeEl) return;
        
        if(val === 'urgent') {
          timeEl.innerHTML = `Response: <span style="font-weight: 500; color: #f43f5e;">&lt; 1 hour</span>`;
        } else if (val === 'high') {
          timeEl.innerHTML = `Response: <span style="font-weight: 500; color: #f59e0b;">~2 hours</span>`;
        } else if (val === 'normal') {
          timeEl.innerHTML = `Response: <span style="font-weight: 500; color: #10b981;">~4 hours</span>`;
        } else {
          timeEl.innerHTML = `Response: <span style="font-weight: 500; color: #6b7280;">~24 hours</span>`;
        }
      });
      // Initial trigger
      prioritySelect.dispatchEvent(new Event("change"));
    }
  });

  async function loadMyTickets() {
    try {
      const resp = await fetch("/api/support/tickets");
      if (resp.ok) {
        const data = await resp.json();
        allTickets = data.tickets || data || [];
        renderTickets();
        updateStats();
      } else if (resp.status === 401) {
        window.location.href = "/auth/login";
      }
    } catch (e) {
      console.error("Error loading tickets", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
    }
  }

  function updateStats() {
    const open = allTickets.filter(
      (t) => t.status === "open" || t.status === "in_progress",
    ).length;
    const resolved = allTickets.filter(
      (t) => t.status === "resolved" || t.status === "closed",
    ).length;
    setEl("stat-open-count", open);
    setEl("stat-resolved-count", resolved);
    setEl("stat-total-count", allTickets.length);
  }

  function renderTickets() {
    const container = document.getElementById("tickets-list");
    if (!container) return;

    let filtered = allTickets;
    if (currentFilter === "open")
      filtered = allTickets.filter(
        (t) => t.status === "open" || t.status === "in_progress",
      );
    if (currentFilter === "resolved")
      filtered = allTickets.filter(
        (t) => t.status === "resolved" || t.status === "closed",
      );

    if (filtered.length === 0) {
      container.innerHTML = `
                <div class="support-empty">
                    <svg width="40" height="40" viewBox="0 0 20 20" fill="none" stroke="var(--support-muted)" stroke-width="1.5">
                        <path d="M18 10c0 4.418-3.582 8-8 8a8.07 8.07 0 01-3.2-.66L2 18l.66-4.8A8.07 8.07 0 012 10c0-4.418 3.582-8 8-8s8 3.582 8 8z" />
                    </svg>
                    <p>No tickets ${currentFilter ? `with "${currentFilter}" status` : "yet"}.</p>
                    <p class="muted">Submit a ticket above and we'll get back to you within 24 hours.</p>
                </div>`;
      return;
    }

    container.innerHTML = filtered
      .map(
        (t) => `
            <div class="support-ticket-card" onclick="document.getElementById('detail-${esc(t.id)}')?.classList.toggle('open')">
                <div class="ticket-card-header">
                    <div class="ticket-card-left">
                        <span class="ticket-priority ticket-priority--${esc(t.priority || "normal")}">${esc((t.priority || "normal").toUpperCase())}</span>
                        <h3 class="ticket-card-subject">${esc(t.subject)}</h3>
                    </div>
                    <div class="ticket-card-right">
                        <span class="ticket-status ticket-status--${esc(t.status)}">${formatStatus(t.status)}</span>
                        <span class="ticket-card-date">${formatDate(t.created_at)}</span>
                    </div>
                </div>
                <div class="ticket-card-detail" id="detail-${esc(t.id)}">
                    <div class="ticket-card-message">${esc(t.message)}</div>
                    ${t.replies && t.replies.length
            ? `
                        <div class="ticket-replies">
                            <h4>Conversation</h4>
                            ${t.replies
              .map(
                (r) => `
                                <div class="ticket-reply ${r.is_admin ? "reply-admin" : "reply-user"}">
                                    <div class="reply-header">
                                        <span class="reply-author">${r.is_admin ? "🛡️ Support Team" : "👤 You"}</span>
                                        <span class="reply-date">${formatDate(r.created_at)}</span>
                                    </div>
                                    <div class="reply-body">${esc(r.message)}</div>
                                    ${r.attachments_json && r.attachments_json.length > 0 ? `
                                        <div class="reply-attachments" style="margin-top: 8px; display: flex; gap: 8px; flex-wrap: wrap;">
                                            ${r.attachments_json.map(a => {
                                                // Skip gs:// URLs that browsers can't open directly
                                                const url = a.file_url && !a.file_url.startsWith('gs://') ? esc(a.file_url) : '#';
                                                const isDisabled = url === '#' ? 'style="display: inline-flex; align-items: center; gap: 4px; padding: 4px 8px; background: rgba(0,0,0,0.05); border-radius: 4px; font-size: 12px; text-decoration: none; color: #9ca3af; cursor: not-allowed;" title="File not available for preview"' : 'style="display: inline-flex; align-items: center; gap: 4px; padding: 4px 8px; background: rgba(0,0,0,0.05); border-radius: 4px; font-size: 12px; text-decoration: none; color: #535862;"';
                                                return `
                                                <a href="${url}" target="_blank" class="attachment-link" ${isDisabled}>
                                                    <svg width="12" height="12" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2"><path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"></path></svg>
                                                    Attachment
                                                </a>`;
                                            }).join('')}
                                        </div>
                                    ` : ''}
                                </div>
                            `,
              )
              .join("")}
                        </div>
                    `
            : ""
          }
                    <div class="ticket-actions" onclick="event.stopPropagation()">
                        ${t.status === "open" || t.status === "in_progress"
            ? `
                            <form class="ticket-reply-form" onsubmit="window._submitReply(event, '${esc(t.id)}')">
                                <textarea class="ticket-reply-input" placeholder="Type your reply…" required rows="2"></textarea>
                                <button type="submit" class="ds-btn ds-btn--primary ticket-reply-btn">Send Reply</button>
                            </form>
                        `
            : ""
          }
                        ${t.status === "resolved" || t.status === "closed"
            ? `
                            <button class="ticket-reopen-btn" onclick="window._reopenTicket('${esc(t.id)}')">Reopen Ticket</button>
                        `
            : ""
          }
                    </div>
                </div>
            </div>
        `,
      )
      .join("");
  }

  // Expose reply and reopen to window for inline handlers
  window._submitReply = async function (e, ticketId) {
    e.preventDefault();
    const form = e.target;
    const textarea = form.querySelector("textarea");
    const message = textarea?.value?.trim();
    if (!message) return;

    const btn = form.querySelector("button");
    if (btn) {
      btn.disabled = true;
      btn.textContent = "Sending…";
    }

    try {
      const resp = await fetch(`/api/support/tickets/${ticketId}/reply`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCookie("csrf_token") || ""
        },
        body: JSON.stringify({ message }),
      });
      if (resp.ok) {
        showToast("Reply sent!", "success");
        loadMyTickets();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to send reply.", "error");
      }
    } catch (e) {
      showToast("Network error. Please try again.", "error");
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = "Send Reply";
      }
    }
  };

  window._reopenTicket = async function (ticketId) {
    if (!confirm("Reopen this ticket?")) return;
    try {
      const resp = await fetch(`/api/support/tickets/${ticketId}/reopen`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCookie("csrf_token") || ""
        },
      });
      if (resp.ok) {
        showToast("Ticket reopened.", "success");
        loadMyTickets();
      } else {
        showToast("Failed to reopen ticket.", "error");
      }
    } catch (e) {
      showToast("Network error.", "error");
    }
  };

  async function handleSubmit(e) {
    e.preventDefault();
    const subject = document.getElementById("ticket-subject")?.value?.trim();
    const message = document.getElementById("ticket-message")?.value?.trim();
    const priority = document.getElementById("ticket-priority")?.value || "normal";
    const category = document.getElementById("ticket-category")?.value || "general";
    const attachment = document.getElementById("ticket-attachment")?.files[0];

    if (!subject || !message) return;

    const btn = document.getElementById("submit-ticket-btn");
    if (btn) {
      btn.disabled = true;
      btn.textContent = "Submitting…";
    }

    // Build context
    const navContext = {
      userAgent: navigator.userAgent,
      language: navigator.language,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      url: window.location.href
    };

    const formData = new FormData();
    formData.append("subject", subject);
    formData.append("message", message);
    formData.append("priority", priority);
    formData.append("category", category);
    formData.append("context", JSON.stringify(navContext));
    if (attachment) {
      formData.append("attachment", attachment);
    }

    try {
      const resp = await fetch("/api/support/tickets", {
        method: "POST",
        headers: {
          "X-CSRF-Token": getCookie("csrf_token") || ""
        },
        body: formData,
      });
      if (resp.ok) {
        document.getElementById("ticket-subject").value = "";
        document.getElementById("ticket-message").value = "";
        const fileInput = document.getElementById("ticket-attachment");
        if (fileInput) fileInput.value = "";
        const suggestionBox = document.getElementById("faq-suggestion-container");
        if (suggestionBox) suggestionBox.style.display = "none";

        showToast(
          "Ticket submitted! We'll respond within 24 hours.",
          "success",
        );
        loadMyTickets();
        // Switch to All tickets tab
        document
          .querySelectorAll(".support-tab")
          .forEach((t) => t.classList.remove("active"));
        document
          .querySelector('.support-tab[data-filter=""]')
          ?.classList.add("active");
        currentFilter = "";
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to submit ticket.", "error");
      }
    } catch (e) {
      showToast("Network error. Please try again.", "error");
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = "Submit Ticket";
      }
    }
  }

  function filterFAQ() {
    const q = document.getElementById("faq-search")?.value?.toLowerCase() || "";
    const activeTab = document.querySelector(".faq-cat-tab.active");
    const activeCat = activeTab ? activeTab.dataset.category : "";

    document.querySelectorAll(".faq-item").forEach((item) => {
      const text = item.textContent.toLowerCase();
      const itemCat = item.dataset.category || "";
      const matchesSearch = text.includes(q);
      const matchesCategory = !activeCat || itemCat === activeCat;

      item.style.display = (matchesSearch && matchesCategory) ? "" : "none";
    });
  }

  function suggestFAQ() {
    clearTimeout(faqDebounceTimeout);
    faqDebounceTimeout = setTimeout(() => {
      const q = document.getElementById("ticket-subject")?.value?.toLowerCase() || "";
      const container = document.getElementById("faq-suggestion-container");
      if (!container) return;

      if (q.length < 5) {
        container.style.display = "none";
        return;
      }

      let matches = [];
      document.querySelectorAll(".faq-item").forEach((item) => {
        const text = item.textContent.toLowerCase();
        if (text.includes(q)) {
          const question = item.querySelector('.faq-question')?.textContent?.trim();
          // Remove the svg arrow text if present
          const cleanQuestion = question ? question.replace('How', 'How').trim() : null;
          if (cleanQuestion) matches.push({ title: cleanQuestion, node: item });
        }
      });

      if (matches.length > 0) {
        matches = matches.slice(0, 2); // Show top 2
        let html = `<div style="font-weight: 600; font-size: 13px; color: #344054; margin-bottom: 6px;">Suggested Answers:</div>`;
        matches.forEach(m => {
          html += `<div style="font-size: 13px; color: #2E2EF9; cursor: pointer; margin-bottom: 4px; display: flex; align-items: center; gap: 6px;" onclick="document.getElementById('faq-search').focus(); window.scrollTo({top: document.body.scrollHeight, behavior: 'smooth'});">
            <svg width="14" height="14" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 2v16M2 10h16"></path></svg>
            <span>Did you mean: ${m.title}</span>
          </div>`;
        });
        container.innerHTML = html;
        container.style.display = "block";
      } else {
        container.style.display = "none";
      }
    }, 400);
  }

  function formatStatus(s) {
    const m = {
      open: "Open",
      in_progress: "In Progress",
      resolved: "Resolved",
      closed: "Closed",
    };
    return m[s] || s;
  }

  function formatDate(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    return (
      d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
      " " +
      d.toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })
    );
  }

  function setEl(id, val) {
    const el = document.getElementById(id);
    if (el) el.textContent = val;
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function showToast(msg, type) {
    const colors = { success: "#22c55e", error: "#ef4444", info: "#6366f1" };
    const t = document.createElement("div");
    t.style.cssText = `position:fixed;bottom:24px;right:24px;z-index:9999;padding:14px 22px;border-radius:12px;font-size:14px;font-weight:500;background:${colors[type] || colors.info};color:#fff;box-shadow:0 8px 30px rgba(0,0,0,0.2);animation:fadeIn 0.25s ease;`;
    t.textContent = msg;
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 4000);
  }

  function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
    return null;
  }
})();
