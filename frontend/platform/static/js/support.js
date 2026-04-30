/**
 * User-Facing Support Page JS
 * Loads the user's own tickets, allows creating new ones, viewing details,
 * replying to tickets, reopening resolved tickets, and searching FAQ.
 */
(function () {
  "use strict";

  let allTickets = [];
  let currentFilter = "";
  let currentSearch  = "";
  let visibleCount   = 10;
  let faqDebounceTimeout;
  let pollTimer = null;

  document.addEventListener("DOMContentLoaded", () => {
    loadMyTickets();
    initSupportNav();
    initClipboardPaste();
    initPolling();
    clearLegacyDraft();
    loadDraft();
    updateCharacterCounts();

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
      ticketSubject.addEventListener("input", () => {
        updateCharacterCounts();
        saveDraft();
      });
    }

    const ticketMessage = document.getElementById("ticket-message");
    if (ticketMessage) {
      ticketMessage.addEventListener("input", () => {
        updateCharacterCounts();
        saveDraft();
      });
    }

    // FAQ accordion toggle
    document.querySelectorAll(".faq-question").forEach((q) => {
      q.setAttribute("aria-expanded", "false");
      q.addEventListener("click", () => {
        const item = q.closest(".faq-item");
        const isOpen = item.classList.toggle("open");
        q.setAttribute("aria-expanded", String(isOpen));
      });
    });

    // Priority time update
    const prioritySelect = document.getElementById("ticket-priority");
    if (prioritySelect) {
      prioritySelect.addEventListener("change", (e) => {
        const val = e.target.value;
        updateResponseTime(val);
        saveDraft();
      });
      // Initial trigger
      updateResponseTime(prioritySelect.value);
    }

    document.getElementById("ticket-category")?.addEventListener("change", saveDraft);
    document.getElementById("clear-ticket-draft-btn")?.addEventListener("click", clearDraft);

    // Ticket list search
    document.getElementById("tickets-search")?.addEventListener("input", (e) => {
      currentSearch = e.target.value.toLowerCase();
      visibleCount = 10;
      renderTickets();
    });

    // Drag-and-drop file upload
    initDragDropUpload();
  });

  function initSupportNav() {
    const links = Array.from(document.querySelectorAll('.developer-lb-topbar a[href^="#"]'));
    if (!links.length) return;
    const sections = links
      .map((link) => document.querySelector(link.getAttribute("href")))
      .filter(Boolean);

    links.forEach((link) => {
      link.addEventListener("click", () => {
        links.forEach((item) => item.classList.remove("active"));
        link.classList.add("active");
      });
    });

    window.addEventListener("scroll", () => {
      let activeId = sections[0]?.id;
      sections.forEach((section) => {
        if (section.getBoundingClientRect().top <= 140) {
          activeId = section.id;
        }
      });
      links.forEach((link) => {
        link.classList.toggle("active", link.getAttribute("href") === `#${activeId}`);
      });
    }, { passive: true });
  }

  function updateResponseTime(val) {
        const timeEl = document.getElementById("expected-response-time");
        const overviewEl = document.getElementById("support-overview-response");
        if(!timeEl) return;
        let html = `Response: <span style="font-weight: 500; color: #10b981;">~4 hours</span>`;
        let plain = "~4 hours";
        if(val === 'urgent') {
          html = `Response: <span style="font-weight: 500; color: #f43f5e;">&lt; 1 hour</span>`;
          plain = "< 1 hour";
        } else if (val === 'high') {
          html = `Response: <span style="font-weight: 500; color: #f59e0b;">~2 hours</span>`;
          plain = "~2 hours";
        } else if (val === 'normal') {
          html = `Response: <span style="font-weight: 500; color: #10b981;">~4 hours</span>`;
          plain = "~4 hours";
        } else {
          html = `Response: <span style="font-weight: 500; color: #6b7280;">~24 hours</span>`;
          plain = "~24 hours";
        }
        timeEl.innerHTML = html;
        if (overviewEl) overviewEl.textContent = plain;
  }

  function updateCharacterCounts() {
    const subject = document.getElementById("ticket-subject");
    const message = document.getElementById("ticket-message");
    setEl("ticket-subject-count", String(subject?.value.length || 0));
    setEl("ticket-message-count", String(message?.value.length || 0));
    const hint = document.getElementById("ticket-message-hint");
    if (hint && message) {
      const remaining = Math.max(0, 20 - message.value.trim().length);
      hint.textContent = remaining > 0 ? `${remaining} more characters needed.` : "Ready to submit.";
    }
  }

  var DRAFT_KEY = "poool:support-draft-v2";

  function clearLegacyDraft() {
    window.localStorage?.removeItem("poool:support-ticket-draft");
  }

  function saveDraft() {
    try {
      const draft = {
        subject:  document.getElementById("ticket-subject")?.value  || "",
        message:  document.getElementById("ticket-message")?.value  || "",
        category: document.getElementById("ticket-category")?.value || "general",
        priority: document.getElementById("ticket-priority")?.value || "normal",
      };
      localStorage.setItem(DRAFT_KEY, JSON.stringify(draft));
      setDraftStatus("Draft saved");
      clearTimeout(saveDraft._clearTimer);
      saveDraft._clearTimer = setTimeout(() => setDraftStatus(""), 2000);
    } catch (_) {}
  }

  function loadDraft() {
    try {
      const raw = localStorage.getItem(DRAFT_KEY);
      if (!raw) return;
      const draft = JSON.parse(raw);
      const subjectEl  = document.getElementById("ticket-subject");
      const messageEl  = document.getElementById("ticket-message");
      const categoryEl = document.getElementById("ticket-category");
      const priorityEl = document.getElementById("ticket-priority");
      if (subjectEl  && draft.subject)  subjectEl.value  = draft.subject;
      if (messageEl  && draft.message)  messageEl.value  = draft.message;
      if (categoryEl && draft.category) categoryEl.value = draft.category;
      if (priorityEl && draft.priority) priorityEl.value = draft.priority;
      updateCharacterCounts();
      if (priorityEl) updateResponseTime(priorityEl.value);
      if (draft.subject || draft.message) setDraftStatus("Draft restored");
    } catch (_) {}
  }

  function clearDraft() {
    clearLegacyDraft();
    localStorage.removeItem(DRAFT_KEY);
    const form = document.getElementById("support-form");
    form?.reset();
    resetFileInfo(document.getElementById("drop-zone-file-info"), document.getElementById("drop-zone"));
    updateCharacterCounts();
    updateResponseTime(document.getElementById("ticket-priority")?.value || "normal");
    setDraftStatus("");
  }

  function setDraftStatus(text) {
    const status = document.getElementById("support-draft-status");
    if (status) status.textContent = text;
  }

  function initDragDropUpload() {
    const dropZone = document.getElementById("drop-zone");
    const fileInput = document.getElementById("ticket-attachment");
    const fileInfo = document.getElementById("drop-zone-file-info");
    const trigger = dropZone?.querySelector(".drop-zone-content");
    if (!dropZone || !fileInput) return;

    trigger?.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        fileInput.click();
      }
    });

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
        const file = e.dataTransfer.files[0];
        if (!validateAttachment(file)) return;
        fileInput.files = e.dataTransfer.files;
        updateFileInfo(file, fileInfo, dropZone);
      }
    });

    fileInput.addEventListener("change", () => {
      if (fileInput.files.length > 0) {
        const file = fileInput.files[0];
        if (!validateAttachment(file)) {
          fileInput.value = "";
          resetFileInfo(fileInfo, dropZone);
          return;
        }
        updateFileInfo(file, fileInfo, dropZone);
      } else {
        resetFileInfo(fileInfo, dropZone);
      }
    });
  }

  function validateAttachment(file) {
    const maxSize = 5 * 1024 * 1024;
    const allowedTypes = ["image/png", "image/jpeg", "application/pdf"];
    if (file.size > maxSize) {
      showToast("Attachment must be 5MB or smaller.", "error");
      return false;
    }
    if (!allowedTypes.includes(file.type)) {
      showToast("Attachment must be a JPG, PNG, or PDF.", "error");
      return false;
    }
    return true;
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
        updateTicketSummary();
        renderTickets();
      } else if (resp.status === 401) {
        window.location.href = "/auth/login";
      } else {
        const err = await resp.json().catch(() => ({}));
        renderTicketsError(err.error || "Could not load your tickets.");
      }
    } catch (e) {
      console.error("Error loading tickets", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      renderTicketsError("Network error while loading tickets.");
    }
  }

  function isOpenStatus(status) {
    return status === "open" || status === "in_progress" || status === "waiting_on_customer";
  }

  function isClosedStatus(status) {
    return status === "resolved" || status === "closed";
  }



  function renderTickets() {
    const container = document.getElementById("tickets-list");
    if (!container) return;

    let filtered = allTickets;
    if (currentFilter === "open")
      filtered = allTickets.filter((t) => isOpenStatus(t.status));
    if (currentFilter === "resolved")
      filtered = allTickets.filter((t) => isClosedStatus(t.status));

    // Search filter
    if (currentSearch) {
      filtered = filtered.filter(
        (t) =>
          (t.subject || "").toLowerCase().includes(currentSearch) ||
          (t.message || "").toLowerCase().includes(currentSearch) ||
          (t.category || "").toLowerCase().includes(currentSearch)
      );
    }

    if (filtered.length === 0) {
      container.innerHTML = `
                <div class="support-empty">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#D0D5DD" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
                    </svg>
                    <p style="font-size: 16px; font-weight: 600; color: #344054; margin-top: 12px;">${currentSearch ? "No tickets match your search" : `No tickets ${currentFilter ? `with "${currentFilter}" status` : "yet"}`}</p>
                    <p class="muted">${currentSearch ? "Try different keywords." : "Submit a ticket above and we'll get back to you shortly."}</p>
                </div>`;
      return;
    }

    const page = filtered.slice(0, visibleCount);
    const hasMore = filtered.length > visibleCount;

    container.innerHTML = page
      .map(
        (t) => `
            <div class="support-ticket-card ticket-priority-border--${esc(t.priority || "normal")}" data-ticket-id="${esc(t.id)}">
                <button type="button" class="ticket-card-header" data-ticket-toggle="${esc(t.id)}" aria-expanded="false" aria-controls="detail-${esc(t.id)}">
                    <div class="ticket-card-left">
                        <div class="ticket-card-meta">
                            <span class="ticket-priority ticket-priority--${esc(t.priority || "normal")}">${priorityIcon(t.priority || "normal")}${esc((t.priority || "normal").toUpperCase())}</span>
                            ${t.category && t.category !== 'general' ? `<span class="ticket-category-tag">${esc(t.category)}</span>` : ''}
                        </div>
                        <h3 class="ticket-card-subject">${esc(t.subject)}</h3>
                    </div>
                    <div class="ticket-card-right">
                        <span class="ticket-status ticket-status--${esc(t.status)}">${statusIcon(t.status)}${formatStatus(t.status)}</span>
                        <span class="ticket-card-date">${formatDate(t.created_at)}</span>
                    </div>
                </button>
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
                                        <span class="reply-author">${r.is_admin ? SVG_ADMIN_AVATAR + " Support Team" : SVG_USER_AVATAR + " You"}</span>
                                        <span class="reply-date">${formatDate(r.created_at)}</span>
                                    </div>
                                    <div class="reply-body">${r.is_admin ? sanitizeAdminContent(r.message) : esc(r.message).replace(/\n/g, "<br>")}</div>
                                    ${renderAttachments(r.attachments_json)}
                                </div>
                            `,
              )
              .join("")}
                        </div>
                    `
            : ""
          }
                    ${renderCsat(t)}
                    <div class="ticket-actions">
                        ${isOpenStatus(t.status)
            ? `
                            <form class="ticket-reply-form" data-ticket-id="${esc(t.id)}">
                                <textarea class="ticket-reply-input" placeholder="Type your reply…" required rows="2"></textarea>
                                <div class="ticket-reply-footer">
                                    <label class="ticket-reply-attach-btn" for="reply-attach-${esc(t.id)}" title="Attach screenshot or file (JPG, PNG, PDF — max 5 MB)">
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/></svg>
                                        Attach
                                    </label>
                                    <input type="file" id="reply-attach-${esc(t.id)}" class="reply-attach-input" accept="image/png,image/jpeg,application/pdf" />
                                    <span class="reply-attach-name" id="reply-attach-name-${esc(t.id)}"></span>
                                    <button type="submit" class="ds-btn ds-btn--primary ticket-reply-btn">Send Reply</button>
                                </div>
                            </form>
                        `
            : ""
          }
                        ${isClosedStatus(t.status)
            ? `
                            <button type="button" class="ticket-reopen-btn" data-ticket-id="${esc(t.id)}">Reopen Ticket</button>
                        `
            : ""
          }
                    </div>
                </div>
            </div>
        `,
      )
      .join("");

    if (hasMore) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "support-load-more";
      btn.textContent = `Show more (${filtered.length - visibleCount} remaining)`;
      btn.addEventListener("click", () => {
        visibleCount += 10;
        renderTickets();
      });
      container.appendChild(btn);
    }

    bindTicketActions(container);
  }

  function renderTicketsError(message) {
    const container = document.getElementById("tickets-list");
    if (!container) return;
    container.innerHTML = `
      <div class="support-empty support-error" role="alert">
        <p style="font-size: 16px; font-weight: 600; color: #344054; margin-top: 12px;">${esc(message)}</p>
        <button type="button" class="ds-btn ds-btn--secondary" id="support-retry-btn">Retry</button>
      </div>`;
    document.getElementById("support-retry-btn")?.addEventListener("click", loadMyTickets);
  }

  function bindTicketActions(container) {
    container.querySelectorAll("[data-ticket-toggle]").forEach((button) => {
      button.addEventListener("click", () => {
        const detail = document.getElementById(`detail-${button.dataset.ticketToggle}`);
        if (!detail) return;
        const isOpen = detail.classList.toggle("open");
        button.setAttribute("aria-expanded", String(isOpen));
      });
    });

    container.querySelectorAll(".ticket-reply-form").forEach((form) => {
      form.addEventListener("submit", (event) => submitReply(event, form.dataset.ticketId));
      const fileInput = form.querySelector(".reply-attach-input");
      if (fileInput) {
        fileInput.addEventListener("change", () => {
          const nameEl = form.querySelector(".reply-attach-name");
          if (nameEl) {
            const f = fileInput.files[0];
            nameEl.textContent = f ? f.name : "";
            nameEl.title = f ? f.name : "";
          }
        });
      }
    });

    container.querySelectorAll(".ticket-reopen-btn").forEach((button) => {
      button.addEventListener("click", () => reopenTicket(button.dataset.ticketId));
    });
  }

  function updateTicketSummary() {
    const openCount = allTickets.filter((t) => isOpenStatus(t.status)).length;
    const resolvedCount = allTickets.filter((t) => isClosedStatus(t.status)).length;
    setEl("support-open-count", String(openCount));
    setEl("support-resolved-count", String(resolvedCount));
    setEl("support-tab-count-all", String(allTickets.length));
    setEl("support-tab-count-open", String(openCount));
    setEl("support-tab-count-resolved", String(resolvedCount));
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

  async function submitReply(e, ticketId) {
    e.preventDefault();
    if (!ticketId) return;
    const form = e.target;
    const textarea = form.querySelector("textarea");
    const message = textarea?.value?.trim();
    if (!message) return;

    const fileInput = form.querySelector(".reply-attach-input");
    const attachment = fileInput?.files[0];
    if (attachment && attachment.size > 5 * 1024 * 1024) {
      showToast("Attachment must be under 5 MB.", "error");
      return;
    }

    const btn = form.querySelector(".ticket-reply-btn");
    if (btn) { btn.disabled = true; btn.textContent = "Sending…"; }

    try {
      const formData = new FormData();
      formData.append("message", message);
      if (attachment) formData.append("attachment", attachment);

      const resp = await fetch(`/api/support/tickets/${ticketId}/reply`, {
        method: "POST",
        headers: { "X-CSRF-Token": getCookie("csrf_token") || "" },
        body: formData,
      });
      if (resp.ok) {
        textarea.value = "";
        if (fileInput) { fileInput.value = ""; }
        const nameEl = form.querySelector(".reply-attach-name");
        if (nameEl) nameEl.textContent = "";
        showToast("Reply sent!", "success");
        loadMyTickets();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to send reply.", "error");
      }
    } catch (err) {
      showToast("Network error. Please try again.", "error");
    } finally {
      if (btn) { btn.disabled = false; btn.textContent = "Send Reply"; }
    }
  }

  function sanitizeAdminContent(html) {
    // Strip scripts only — backend already sanitizes via ammonia before storing.
    return (html || "").replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "");
  }

  async function reopenTicket(ticketId) {
    if (!ticketId) return;
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
  }

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
        clearLegacyDraft();
        localStorage.removeItem(DRAFT_KEY);
        setDraftStatus("");
        updateCharacterCounts();
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

    document.querySelectorAll(".faq-item").forEach((item) => {
      const text = item.textContent.toLowerCase();
      const matchesSearch = text.includes(q);

      item.style.display = matchesSearch ? "" : "none";
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
        let html = `<div class="faq-suggestion-header">Suggested answers</div>`;
        matches.forEach(m => {
          html += `<div class="faq-suggestion-item" onclick="document.getElementById('faq')?.setAttribute('open', ''); document.getElementById('faq-search')?.focus(); document.getElementById('faq')?.scrollIntoView({ behavior: 'smooth', block: 'start' });">
            <svg width="14" height="14" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 2v16M2 10h16"></path></svg>
            <span>${esc(m.title)}</span>
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

  function priorityIcon(p) {
    switch (p) {
      case 'urgent':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>';
      case 'high':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><polyline points="18 15 12 9 6 15"/></svg>';
      case 'low':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><polyline points="6 9 12 15 18 9"/></svg>';
      default:
        return '<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" aria-hidden="true" style="flex-shrink:0;border-radius:50%"><circle cx="4" cy="4" r="4"/></svg>';
    }
  }

  function statusIcon(s) {
    switch (s) {
      case 'open':
        return '<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" aria-hidden="true" style="flex-shrink:0"><circle cx="4" cy="4" r="4"/></svg>';
      case 'in_progress':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><circle cx="12" cy="12" r="9"/><polyline points="12 7 12 12 15 15"/></svg>';
      case 'waiting_on_customer':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>';
      case 'resolved':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><polyline points="20 6 9 17 4 12"/></svg>';
      case 'closed':
        return '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="flex-shrink:0"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';
      default:
        return '';
    }
  }

  var SVG_ADMIN_AVATAR = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>';
  var SVG_USER_AVATAR  = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>';

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
