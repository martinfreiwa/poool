/**
 * Admin Support Ticket (Single View) JS
 * Deep-dive ticket management, thread, and messaging.
 */

const ticketId = new URLSearchParams(window.location.search).get("id");
let currentTicket = null;
let quillEditor = null;
let attachedFiles = [];

function getCsrfToken() {
  const match = document.cookie.match(/(?:^|; )csrf_token=([^;]+)/);
  return match ? match[1] : "";
}

const CANNED_RESPONSES = {
  greeting:
    "<p>Hi there,</p><p><br></p><p>Thank you for reaching out to POOOL Support. How can we help you today?</p>",
  closing:
    "<p><br></p><p>If you have any other questions, please let us know.</p><p><br></p><p>Best regards,<br>POOOL Support Team</p>",
  investigating:
    "<p>Thanks for the details. We are currently investigating this issue and will get back to you as soon as we have an update.</p>",
  resolved:
    "<p>We're pleased to let you know that this issue has been resolved. Please check your account. Let us know if you need anything else!</p>",
  kyc_request:
    "<p>We need a bit more information to process your request. Could you please provide your latest KYC documents via the portal?</p>",
};

document.addEventListener("DOMContentLoaded", () => {
  if (!ticketId) {
    document.getElementById("ticket-title").textContent = "Ticket not found";
    return;
  }
  loadTicket();

  // Init Quill Editor
  // Wait until Quill is loaded
  const initQuillInterval = setInterval(() => {
    if (window.Quill) {
      clearInterval(initQuillInterval);
      quillEditor = new Quill("#quill-editor", {
        theme: "snow",
        placeholder: "Type your response here... (Tip: type /greeting, /kyc, /closing)",
        modules: {
          toolbar: [
            ["bold", "italic", "underline", "strike"],
            ["blockquote", "code-block"],
            [{ list: "ordered" }, { list: "bullet" }],
            [{ color: [] }, { background: [] }],
            ["link", "clean"],
          ],
        },
      });

      // Add macro slash command shortcuts
      quillEditor.on('text-change', function(delta, oldDelta, source) {
        if (source === 'user') {
          const selection = quillEditor.getSelection();
          if (!selection) return;
          const index = selection.index;
          
          // Get text up to the cursor to see if it ends with a macro command
          const textBeforeCursor = quillEditor.getText(0, index);
          const match = textBeforeCursor.match(/\/(greeting|closing|investigating|resolved|kyc)$/i);
          
          if (match) {
            const command = match[1].toLowerCase();
            const macroMap = {
              'greeting': CANNED_RESPONSES.greeting,
              'closing': CANNED_RESPONSES.closing,
              'investigating': CANNED_RESPONSES.investigating,
              'resolved': CANNED_RESPONSES.resolved,
              'kyc': CANNED_RESPONSES.kyc_request
            };
            const macroHtml = macroMap[command];
            if (macroHtml) {
                // Delete the typed command
                quillEditor.deleteText(index - match[0].length, match[0].length);
                // Insert HTML at that position
                quillEditor.clipboard.dangerouslyPasteHTML(index - match[0].length, macroHtml);
            }
          }
        }
      });
    }
  }, 100);

  // Event Listeners
  document
    .getElementById("btn-send-reply")
    ?.addEventListener("click", sendReply);

  // File Attachments
  document
    .getElementById("file-attachment")
    ?.addEventListener("change", handleFileSelect);
});

function insertCannedResponse(type) {
  if (!type || !quillEditor) return;
  const macroHtml = CANNED_RESPONSES[type] || "";

  // Insert at current cursor index or end of text
  const selection = quillEditor.getSelection(true);
  quillEditor.clipboard.dangerouslyPasteHTML(selection.index, macroHtml);

  // Reset dropdown
  document.getElementById("canned-responses").value = "";
}

function handleFileSelect(event) {
  const files = event.target.files;
  if (!files.length) return;

  for (let i = 0; i < files.length; i++) {
    attachedFiles.push({
      name: files[i].name,
      size: files[i].size,
      file: files[i],
    });
  }

  renderAttachments();
}

function removeAttachment(index) {
  attachedFiles.splice(index, 1);
  renderAttachments();
}

function renderAttachments() {
  const list = document.getElementById("attachment-list");
  if (!list) return;

  list.innerHTML = attachedFiles
    .map(
      (f, i) => `
        <div style="display:flex; align-items:center; gap:6px; background:var(--admin-bg-card); border:1px solid var(--admin-border); padding:4px 10px; border-radius:12px; font-size:12px;">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path></svg>
            <span style="color:var(--admin-text-secondary); max-width:120px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${esc(f.name)}</span>
            <button class="admin-btn admin-btn--icon" style="padding:0; border:none; background:transparent; width:16px; height:16px; margin-left:4px;" onclick="removeAttachment(${i})" title="Remove">✕</button>
        </div>
    `,
    )
    .join("");
}

async function loadTicket() {
  try {
    const resp = await fetch(`/api/admin/support/${ticketId}`);
    if (resp.ok) {
      currentTicket = await resp.json();
      renderTicket();
    } else {
      document.getElementById("ticket-title").textContent =
        "Error loading ticket";
    }
  } catch (e) {
    console.error('Failed to load support ticket:', e);
    if (window.Sentry) Sentry.captureException(e);
  }
}

async function renderTicket() {
  try {
    const t = currentTicket;
    if (!t) return;

    // Header and Breadcrumb
    const shortId = t.id ? t.id.slice(-4) : "????";
    document.getElementById("bc-title").textContent = `TKT-${shortId}`;
    document.getElementById("ticket-title").textContent =
      t.subject || "No Subject";

    let timeOpenText = "0m";
    let slaHtml = "";
    if (t.created_at) {
      const createdDate = new Date(t.created_at);
      const endDate = (t.status === "closed" || t.status === "resolved") 
        ? new Date(t.updated_at || new Date()) 
        : new Date();
      
      const diffMs = Math.max(0, endDate - createdDate);
      const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
      const diffHours = Math.floor((diffMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
      const diffMins = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
      
      if (diffDays > 0) {
        timeOpenText = `${diffDays}d ${diffHours}h ${diffMins}m`;
      } else if (diffHours > 0) {
        timeOpenText = `${diffHours}h ${diffMins}m`;
      } else {
        timeOpenText = `${diffMins}m`;
      }

      // SLA warning logic
      if (t.status === "open" || t.status === "in_progress") {
        if (diffHours >= 48 || diffDays >= 2) {
            slaHtml = `&nbsp; <span class="admin-badge admin-badge--danger" style="margin-left: 6px; padding: 2px 6px; font-size: 10px;">SLA Breached</span>`;
        } else if (diffHours >= 24 || diffDays >= 1) {
            slaHtml = `&nbsp; <span class="admin-badge admin-badge--warning" style="margin-left: 6px; padding: 2px 6px; font-size: 10px;">SLA Warning</span>`;
        } else {
            slaHtml = `&nbsp; <span class="admin-badge admin-badge--success" style="margin-left: 6px; padding: 2px 6px; font-size: 10px;">SLA OK</span>`;
        }
      }
    }

    document.getElementById("ticket-id-meta").innerHTML =
      `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 4px; vertical-align: middle;"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect><line x1="16" y1="2" x2="16" y2="6"></line><line x1="8" y1="2" x2="8" y2="6"></line><line x1="3" y1="10" x2="21" y2="10"></line></svg> Ticket #${t.id} &bull; <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-left: 2px; margin-right: 4px; vertical-align: middle;"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect><line x1="16" y1="2" x2="16" y2="6"></line><line x1="8" y1="2" x2="8" y2="6"></line><line x1="3" y1="10" x2="21" y2="10"></line></svg> Created ${fmtDate(t.created_at)} &bull; <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-left: 2px; margin-right: 4px; vertical-align: middle;"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg> Open for: ${timeOpenText}${slaHtml}`;

    // Fetch admins for assignee list
    try {
      const settingsResp = await fetch("/api/admin/settings");
      if (settingsResp.ok) {
        const data = await settingsResp.json();
        if (data.admins && data.admins.length > 0) {
          const assigneeSelect = document.getElementById("sel-assignee");
          assigneeSelect.innerHTML =
            `<option value="">-- Unassigned --</option>` +
            data.admins
              .map(
                (a) =>
                  `<option value="${a.id}">${esc(a.name || a.email)} (${esc(a.role_display || a.role || "Admin")})</option>`,
              )
              .join("");
        }
      }
    } catch (e) {
      console.error('Failed to load admin settings for assignee list:', e);
    }

    // Set selects
    document.getElementById("sel-status").value = t.status || "open";
    document.getElementById("sel-priority").value = t.priority || "normal";
    document.getElementById("sel-assignee").value = t.assigned_to || "";

    // Render Thread
    const threadEl = document.getElementById("ticket-thread");
    const notesEl = document.getElementById("internal-notes-list");
    
    if (notesEl) notesEl.innerHTML = "";

    if (!t.messages || t.messages.length === 0) {
      threadEl.innerHTML =
        '<div style="text-align:center;padding:40px;color:var(--admin-text-muted);">No messages found.</div>';
    } else {
      let chatHtml = "";
      let notesHtml = "";

      t.messages.forEach((m) => {
        const authorName = esc(m.author_name || "Unknown");
        const timeStr = fmtDateTime(m.created_at);

        let content = m.content || "";
        const isRich = (m.author_role === "agent" || m.author_role === "admin" || m.type === "internal_note");

        if (isRich) {
          // Content from Admins (Quill) is HTML. Backend sanitizes this via ammonia.
          // We still strip scripts for defence-in-depth.
          content = content.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "");
        } else {
          // Content from Customers is ALWAYS treated as plain text and escaped.
          content = esc(content).replace(/\n/g, "<br>");
        }

        if (m.type === "internal_note") {
          notesHtml += `
            <div style="background: var(--admin-hover-overlay); border-left: 3px solid var(--admin-warning); padding: 10px 14px; border-radius: 4px;">
              <div style="font-size: 11px; color: var(--admin-warning); font-weight: 600; margin-bottom: 6px; display: flex; align-items: center; gap: 4px;">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>
                ${authorName} &bull; ${timeStr}
              </div>
              <div style="color: var(--admin-text-primary); line-height: 1.5; font-size: 13px; font-family: var(--admin-font);">${content}</div>
            </div>
          `;
        } else if (m.author_role === "agent" || m.author_role === "admin") {
          chatHtml += `
            <div class="msg-bubble msg-bubble--agent">
                <div class="msg-meta">
                    ${authorName} (Support) &bull; ${timeStr}
                </div>
                <div class="msg-content">${content}</div>
            </div>
          `;
        } else {
          chatHtml += `
            <div class="msg-bubble msg-bubble--customer">
                <div class="msg-meta">
                    ${authorName} (Customer) &bull; ${timeStr}
                </div>
                <div class="msg-content">${content}</div>
            </div>
          `;
        }
      });

      threadEl.innerHTML = chatHtml || '<div style="text-align:center;padding:40px;color:var(--admin-text-muted);">No conversation yet.</div>';
      
      if (notesEl) {
        if (notesHtml) {
          notesEl.innerHTML = notesHtml;
        } else {
          notesEl.innerHTML = '<div style="text-align: center; color: var(--admin-text-muted); padding: 20px;">No internal notes yet.</div>';
        }
        setTimeout(() => { notesEl.scrollTop = notesEl.scrollHeight; }, 10);
      }

      // Scroll to bottom
      setTimeout(() => {
        threadEl.scrollTop = threadEl.scrollHeight;
      }, 10);
    }

    // Show reply box if not closed
    const replyContainer = document.getElementById("reply-box-container");
    if (t.status === "closed") {
      replyContainer.style.display = "none";
      threadEl.innerHTML += `<div style="text-align:center;padding:20px 0;margin-top:20px;border-top:1px dashed var(--admin-border);font-size:12px;color:var(--admin-text-muted);">This ticket is closed. Replies are disabled.</div>`;
    } else {
      replyContainer.style.display = "block";
    }

    // Render User Profile
    const userName = t.user_name || "Unknown User";
    const userInitial = userName.charAt(0).toUpperCase();
    document.getElementById("customer-profile-card").innerHTML = `
            <div style="display:flex;align-items:center;gap:12px;margin-bottom:16px;">
                <div style="width:40px;height:40px;border-radius:50%;background:var(--admin-accent-bg);color:var(--admin-accent);display:flex;align-items:center;justify-content:center;font-weight:600;font-size:16px;">
                    ${userInitial}
                </div>
                <div>
                    <a href="/admin/user-details.html?id=${t.user_id}" style="font-weight:600;font-size:14px;color:var(--admin-text-primary);text-decoration:none;">${esc(userName)}</a>
                    <div style="font-size:12px;color:var(--admin-text-muted);">${esc(t.user_email)}</div>
                </div>
            </div>
            <div class="admin-detail-grid">
                <div class="admin-detail-row"><span class="admin-detail-label">Status</span><span class="admin-detail-value"><span class="admin-badge admin-badge--success">Active</span></span></div>
                <div class="admin-detail-row"><span class="admin-detail-label">Member Since</span><span class="admin-detail-value">${fmtDate(t.user_created_at)}</span></div>
                <div class="admin-detail-row"><span class="admin-detail-label">Open Tickets</span><span class="admin-detail-value">${t.user_open_tickets || 0}</span></div>
                <div class="admin-detail-row"><span class="admin-detail-label">Total Invested</span><span class="admin-detail-value">$${((t.user_total_invested_cents || 0) / 100).toLocaleString()}</span></div>
            </div>
            <div style="margin-top:16px;">
                <a href="/admin/user-details.html?id=${t.user_id}" class="admin-btn admin-btn--secondary" style="width:100%;">View Full Profile</a>
            </div>
        `;
  } catch (err) {
    document.getElementById("ticket-thread").innerHTML =
      `<div style="color:var(--admin-danger);padding:20px;">Error rendering details: ${err.message}</div>`;
  }
}

async function updateTicketField(field, value) {
  try {
    const resp = await fetch(`/api/admin/support/${ticketId}`, {
      method: "PATCH",
      headers: { 
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ [field]: value }),
    });
    if (resp.ok) {
      showToast(`Updated ${field} successfully.`, "success");
      if (field === "status" && value === "closed") {
        loadTicket(); // Reload to hide reply box
      }
    } else {
      const err = await resp.json();
      showToast(err.error || `Failed to update ${field}.`, "danger");
    }
  } catch (e) {
    showToast(`Network error updating ${field}.`, "danger");
  }
}

async function sendReply() {
  if (!quillEditor) return;

  // Check if it's virtually empty
  const textContent = quillEditor.getText().trim();
  if (textContent.length === 0) return;

  // We send Rich HTML content
  const content = quillEditor.root.innerHTML;
  const isNote = false;

  const btn = document.getElementById("btn-send-reply");
  btn.disabled = true;
  btn.textContent = "Sending...";

  // In a full production env, you would first upload attachedFiles to a bucket
  // and append attachment links to the content or send array of URLs.
  // For this demonstration, we'll append a text log of attachments to the reply.
  let finalContent = content;
  if (attachedFiles.length > 0) {
    const attachmentHtml = attachedFiles
      .map(
        (f) => `<li><a href="#" style="color:#6366f1;">${esc(f.name)}</a></li>`,
      )
      .join("");
    finalContent += `<div style="margin-top:10px; padding-top:10px; border-top:1px solid #e2e8f0; font-size:12px;"><strong>Attachments:</strong><ul>${attachmentHtml}</ul></div>`;
  }

  try {
    const resp = await fetch(`/api/admin/support/${ticketId}/messages`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        content: finalContent,
        type: "reply",
      }),
    });
    if (resp.ok) {
      quillEditor.setText(""); // Clear
      attachedFiles = [];
      renderAttachments();
      loadTicket();
      showToast(
        isNote ? "Internal note added." : "Reply sent to customer.",
        "success",
      );
    } else {
      const err = await resp.json();
      showToast(err.error || "Failed to send reply.", "danger");
    }
  } catch (e) {
    showToast("Network error sending reply.", "danger");
  }

  btn.disabled = false;
  btn.textContent = "Send Reply";
}

function showToast(msg, type = "info") {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
function fmtDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
function fmtDateTime(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  return (
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
    " " +
    d.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" })
  );
}

async function addInternalNote() {
  const noteInput = document.getElementById("note-input");
  if (!noteInput) return;
  const textContent = noteInput.value.trim();
  if (!textContent) return;

  const btn = document.getElementById("btn-add-note");
  if (btn) {
    btn.disabled = true;
    btn.textContent = "Saving...";
  }

  // Convert simple newlines to br tags
  const htmlContent = textContent.replace(/\n/g, "<br>");

  try {
    const resp = await fetch(`/api/admin/support/${ticketId}/messages`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        content: htmlContent,
        type: "internal_note",
      }),
    });
    if (resp.ok) {
      noteInput.value = "";
      loadTicket();
      showToast("Internal note added.", "success");
    } else {
      const err = await resp.json();
      showToast(err.error || "Failed to add note.", "danger");
    }
  } catch (e) {
    showToast("Network error saving note.", "danger");
  }

  if (btn) {
    btn.disabled = false;
    btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 4px;"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg> Save Note';
  }
}
