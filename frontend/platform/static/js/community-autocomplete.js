/**
 * Community composer autocomplete (Phase 3 task 28).
 *
 * Attaches a small inline popover to #post-content-input that watches for
 * `@`, `#`, and `$` trigger characters. On input it debounces (250ms) and
 * fetches /api/community/mentions/suggest or /hashtags/suggest. Arrow keys
 * navigate, Enter / Tab inserts. Vanilla JS, no framework.
 */
(function () {
  "use strict";

  const DEBOUNCE_MS = 250;
  const TRIGGERS = {
    "@": { endpoint: "/api/community/mentions/suggest", labelKey: "display_name", insertPrefix: "@" },
    "#": { endpoint: "/api/community/hashtags/suggest",  labelKey: "tag",          insertPrefix: "#" },
  };

  function debounce(fn, wait) {
    let t = null;
    return function (...args) {
      if (t) clearTimeout(t);
      t = setTimeout(() => fn.apply(this, args), wait);
    };
  }

  function setup() {
    const textarea = document.getElementById("post-content-input");
    if (!textarea) return;
    if (textarea.dataset.autocompleteBound === "1") return;
    textarea.dataset.autocompleteBound = "1";

    const popover = document.createElement("div");
    popover.className = "community-autocomplete";
    popover.setAttribute("role", "listbox");
    popover.hidden = true;
    document.body.appendChild(popover);

    let activeIndex = 0;
    let suggestions = [];
    let triggerInfo = null; // { char, start, query }

    function close() {
      popover.hidden = true;
      popover.replaceChildren();
      activeIndex = 0;
      suggestions = [];
      triggerInfo = null;
    }

    function positionPopover() {
      const rect = textarea.getBoundingClientRect();
      popover.style.left = `${window.scrollX + rect.left + 12}px`;
      popover.style.top = `${window.scrollY + rect.bottom - 4}px`;
      popover.style.minWidth = `${Math.max(220, rect.width / 2)}px`;
    }

    function render() {
      popover.replaceChildren();
      if (suggestions.length === 0) {
        const empty = document.createElement("div");
        empty.className = "community-autocomplete__empty";
        empty.textContent = "No matches";
        popover.appendChild(empty);
        return;
      }
      suggestions.forEach((s, idx) => {
        const item = document.createElement("button");
        item.type = "button";
        item.className = "community-autocomplete__item" + (idx === activeIndex ? " community-autocomplete__item--active" : "");
        item.setAttribute("role", "option");
        item.setAttribute("aria-selected", idx === activeIndex ? "true" : "false");
        item.textContent = s.label;
        if (s.subtitle) {
          const sub = document.createElement("span");
          sub.className = "community-autocomplete__subtitle";
          sub.textContent = s.subtitle;
          item.appendChild(sub);
        }
        item.addEventListener("mousedown", (event) => {
          event.preventDefault();
          insertSuggestion(idx);
        });
        popover.appendChild(item);
      });
    }

    function insertSuggestion(index) {
      const pick = suggestions[index];
      if (!pick || !triggerInfo) return;
      const before = textarea.value.slice(0, triggerInfo.start);
      const after = textarea.value.slice(textarea.selectionEnd);
      const insertion = `${triggerInfo.char}${pick.value} `;
      textarea.value = before + insertion + after;
      const caret = before.length + insertion.length;
      textarea.setSelectionRange(caret, caret);
      textarea.focus();
      close();
    }

    function detectTrigger() {
      const pos = textarea.selectionStart;
      const upto = textarea.value.slice(0, pos);
      // Walk backwards to find the most recent @/#/$. Bail if we hit a space first.
      let i = upto.length - 1;
      while (i >= 0) {
        const ch = upto[i];
        if (TRIGGERS[ch]) {
          const before = i === 0 ? " " : upto[i - 1];
          if (/\s|[(\[{,;:]/.test(before)) {
            return { char: ch, start: i, query: upto.slice(i + 1) };
          }
          return null;
        }
        if (/\s/.test(ch)) return null;
        i -= 1;
      }
      return null;
    }

    async function fetchSuggestions(info) {
      const conf = TRIGGERS[info.char];
      if (!conf) return [];
      const url = new URL(conf.endpoint, window.location.origin);
      url.searchParams.set("q", info.query);
      try {
        const res = await fetch(url.toString(), { credentials: "same-origin" });
        if (!res.ok) return [];
        const data = await res.json();
        if (info.char === "@") {
          return (data.users || []).map((u) => ({
            label: u.display_name,
            value: u.display_name,
            subtitle: "",
          }));
        }
        if (info.char === "#") {
          return (data.hashtags || []).map((h) => ({
            label: `#${h.tag}`,
            value: h.tag,
            subtitle: h.post_count ? `${h.post_count} posts` : "",
          }));
        }
        return [];
      } catch (err) {
        console.error("autocomplete fetch failed", err);
        return [];
      }
    }

    const refresh = debounce(async () => {
      const info = detectTrigger();
      if (!info || info.query.length < 1) {
        close();
        return;
      }
      triggerInfo = info;
      suggestions = await fetchSuggestions(info);
      if (!triggerInfo) return; // closed while in flight
      activeIndex = 0;
      positionPopover();
      popover.hidden = false;
      render();
    }, DEBOUNCE_MS);

    textarea.addEventListener("input", refresh);
    textarea.addEventListener("keydown", (event) => {
      if (popover.hidden) return;
      if (event.key === "ArrowDown") {
        event.preventDefault();
        activeIndex = Math.min(activeIndex + 1, Math.max(0, suggestions.length - 1));
        render();
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        activeIndex = Math.max(activeIndex - 1, 0);
        render();
      } else if (event.key === "Enter" || event.key === "Tab") {
        if (suggestions.length === 0) return;
        event.preventDefault();
        insertSuggestion(activeIndex);
      } else if (event.key === "Escape") {
        close();
      }
    });
    textarea.addEventListener("blur", () => setTimeout(close, 150));
    window.addEventListener("scroll", () => { if (!popover.hidden) close(); }, true);
  }

  function bindHtmxRehook() {
    if (document.body) {
      document.body.addEventListener("htmx:afterSwap", setup);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      setup();
      bindHtmxRehook();
    });
  } else {
    setup();
    bindHtmxRehook();
  }
})();
