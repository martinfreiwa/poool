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

    // Drag-and-drop file upload
    initDragDropUpload();
  });

  function initDragDropUpload() {
    const dropZone = document.getElementById("drop-zone");
    const fileInput = document.getElementById("ticket-attachment");
    const fileInfo = document.getElementById("drop-zone-file-info");
    if (!dropZone || !fileInput) return;

    dropZone.addEventListener("click", () => fileInput.click());

    dropZone.addEventListener("dragover", (e) => {
      e.preventDefault();
      dropZone.classList.add("drag-over");
    });

    dropZone.addEventListener("dragleave", () => {
      dropZone.classList.remove("drag-over");
    });

    dropZone.addEventListener("drop", (e) => {
      e.preventDefault();
      dropZone.classList.remove("drag-over");
      if (e.dataTransfer.files.length > 0) {
        fileInput.files = e.dataTransfer.files;
        updateFileInfo(fileInput.files[0], fileInfo, dropZone);
      }
    });

    fileInput.addEventListener("change", () => {
      if (fileInput.files.length > 0) {
        updateFileInfo(fileInput.files[0], fileInfo, dropZone);
      } else {
        resetFileInfo(fileInfo, dropZone);
      }
    });
  }

  function updateFileInfo(file, infoEl, dropZone) {
    if (!infoEl) return;
    const sizeMB = (file.size / (1024 * 1024)).toFixed(2);
    infoEl.innerHTML = `<strong>${esc(file.name)}</strong> (${sizeMB} MB)
      <button type="button" class="drop-zone-remove" aria-label="Remove file">&times;</button>`;
    infoEl.style.display = "block";
    dropZone.classList.add("has-file");

    infoEl.querySelector(".drop-zone-remove")?.addEventListener("click", (e) => {
      e.stopPropagation();
      const input = document.getElementById("ticket-attachment");
      if (input) input.value = "";
      resetFileInfo(infoEl, dropZone);
    });
  }

  function resetFileInfo(infoEl, dropZone) {
    if (infoEl) {
      infoEl.textContent = "";
      infoEl.style.display = "none";
    }
    if (dropZone) dropZone.classList.remove("has-file");
  }

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

  function isOpenStatus(status) {
    return status === "open" || status === "in_progress" || status === "waiting_on_customer";
  }

  function isClosedStatus(status) {
    return status === "resolved" || status === "closed";
  }

  function updateStats() {
    const open = allTickets.filter((t) => isOpenStatus(t.status)).length;
    const resolved = allTickets.filter((t) => isClosedStatus(t.status)).length;
    setEl("stat-open-count", open);
    setEl("stat-resolved-count", resolved);
    setEl("stat-total-count", allTickets.length);
  }

  function renderTickets() {
    const container = document.getElementById("tickets-list");
    if (!container) return;

    let filtered = allTickets;
    if (currentFilter === "open")
      filtered = allTickets.filter((t) => isOpenStatus(t.status));
    if (currentFilter === "resolved")
      filtered = allTickets.filter((t) => isClosedStatus(t.status));

    if (filtered.length === 0) {
      container.innerHTML = `
                <div class="support-empty">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#D0D5DD" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
                    </svg>
                    <p style="font-size: 16px; font-weight: 600; color: #344054; margin-top: 12px;">No tickets ${currentFilter ? `with "${currentFilter}" status` : "yet"}</p>
                    <p class="muted">Submit a ticket above and we'll get back to you shortly.</p>
                </div>`;
      return;
    }

    container.innerHTML = filtered
      .map(
        (t) => `
            <div class="support-ticket-card ticket-priority-border--${esc(t.priority || "normal")}" data-ticket-id="${esc(t.id)}">
                <div class="ticket-card-header" onclick="document.getElementById('detail-${esc(t.id)}')?.classList.toggle('open')">
                    <div class="ticket-card-left">
                        <div class="ticket-card-meta">
                            <span class="ticket-priority ticket-priority--${esc(t.priority || "normal")}">${esc((t.priority || "normal").toUpperCase())}</span>
                            ${t.category && t.category !== 'general' ? `<span class="ticket-category-tag">${esc(t.category)}</span>` : ''}
                        </div>
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
                                    ${renderAttachments(r.attachments_json)}
                                </div>
                            `,
              )
              .join("")}
                        </div>
                    `
            : ""
          }
                    <div class="ticket-actions" onclick="event.stopPropagation()">
                        ${isOpenStatus(t.status)
            ? `
                            <form class="ticket-reply-form" onsubmit="window._submitReply(event, '${esc(t.id)}')">
                                <textarea class="ticket-reply-input" placeholder="Type your reply…" required rows="2"></textarea>
                                <button type="submit" class="ds-btn ds-btn--primary ticket-reply-btn">Send Reply</button>
                            </form>
                        `
            : ""
          }
                        ${isClosedStatus(t.status)
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

  function renderAttachments(attachmentsJson) {
    // Null-safe: handle null, undefined, and non-array values
    if (!attachmentsJson || !Array.isArray(attachmentsJson) || attachmentsJson.length === 0) {
      return '';
    }
    return `<div class="reply-attachments">
      ${attachmentsJson.map(a => {
        const url = a.file_url && !a.file_url.startsWith('gs://') ? esc(a.file_url) : '#';
        const isDisabled = url === '#';
        return `<a href="${url}" target="_blank" class="attachment-link ${isDisabled ? 'attachment-disabled' : ''}" 
                   ${isDisabled ? 'title="File not available for preview" onclick="event.preventDefault()"' : ''}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
                  </svg>
                  Attachment
                </a>`;
      }).join('')}
    </div>`;
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
    if (!await pooolConfirm({ title: 'Reopen ticket', message: 'This will reopen the ticket and notify our support team.', confirmText: 'Reopen', type: 'default' })) return;
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
        const dropZone = document.getElementById("drop-zone");
        const fileInfo = document.getElementById("drop-zone-file-info");
        resetFileInfo(fileInfo, dropZone);
        const suggestionBox = document.getElementById("faq-suggestion-container");
        if (suggestionBox) suggestionBox.style.display = "none";

        showToast(
          "Ticket submitted! We'll respond shortly.",
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
          const questionBtn = item.querySelector('.faq-question');
          // Get text content excluding the SVG arrow
          const cleanQuestion = questionBtn ? questionBtn.childNodes[0]?.textContent?.trim() : null;
          if (cleanQuestion) matches.push({ title: cleanQuestion, node: item });
        }
      });

      if (matches.length > 0) {
        matches = matches.slice(0, 2); // Show top 2
        let html = `<div class="faq-suggestion-header">💡 Suggested Answers:</div>`;
        matches.forEach(m => {
          html += `<div class="faq-suggestion-item" onclick="document.getElementById('faq-search').focus(); window.scrollTo({top: document.body.scrollHeight, behavior: 'smooth'});">
            <svg width="14" height="14" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 2v16M2 10h16"></path></svg>
            <span>Did you mean: ${esc(m.title)}</span>
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
      waiting_on_customer: "Awaiting Reply",
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
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}

  function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
    return null;
  }
})();
