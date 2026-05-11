/**
 * community-profile.js — WS3.4
 *
 * Powers the /community/me + /community/u/:user_id profile page. Lazy-loads
 * each tab on first activation, paginates with a "Load more" affordance,
 * and reuses helpers from community-feed.js (window.openUserProfile,
 * window.toggleFollow, window.openRelationshipList, etc).
 */
(function () {
  "use strict";

  const PROFILE_ID = window.PROFILE_USER_ID;
  const IS_OWN = window.PROFILE_IS_OWN === true;

  function csrfHeaders(extra) {
    const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = match ? decodeURIComponent(match[1]) : "";
    return token ? Object.assign({}, extra || {}, { "X-CSRF-Token": token }) : Object.assign({}, extra || {});
  }

  function getInitials(name) {
    if (!name) return "?";
    const parts = name.split(" ");
    return (parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2)).toUpperCase();
  }

  function timeAgo(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    const s = Math.floor((Date.now() - d) / 1000);
    if (s < 60) return "just now";
    if (s < 3600) return Math.floor(s / 60) + "m ago";
    if (s < 86400) return Math.floor(s / 3600) + "h ago";
    if (s < 30 * 86400) return Math.floor(s / 86400) + "d ago";
    return d.toLocaleDateString();
  }

  function emptyState(panel, title, desc) {
    panel.innerHTML = "";
    const wrap = document.createElement("div");
    wrap.className = "community-state community-state--empty";
    const icon = document.createElement("div");
    icon.className = "community-state__icon";
    icon.setAttribute("aria-hidden", "true");
    icon.textContent = "🌿";
    const h = document.createElement("h3");
    h.className = "community-state__title";
    h.textContent = title;
    const p = document.createElement("p");
    p.className = "community-state__desc";
    p.textContent = desc;
    wrap.append(icon, h, p);
    panel.appendChild(wrap);
  }

  function errorState(panel, message) {
    panel.innerHTML = "";
    const wrap = document.createElement("div");
    wrap.className = "community-state community-state--error";
    const icon = document.createElement("div");
    icon.className = "community-state__icon";
    icon.textContent = "!";
    const p = document.createElement("p");
    p.className = "community-state__desc";
    p.textContent = message;
    wrap.append(icon, p);
    panel.appendChild(wrap);
  }

  async function fetchJson(url) {
    const res = await fetch(url, { credentials: "same-origin" });
    if (!res.ok) throw new Error("Request failed (" + res.status + ")");
    return res.json();
  }

  // ─── Tab plumbing ───────────────────────────────────────────────
  const loaded = new Set();
  const pageState = new Map(); // tab -> next page to fetch

  function setActiveTab(name) {
    document.querySelectorAll(".community-profile-tab").forEach((b) => {
      const active = b.dataset.tab === name;
      b.classList.toggle("active", active);
      b.setAttribute("aria-selected", active ? "true" : "false");
    });
    document.querySelectorAll(".community-profile-panel").forEach((p) => {
      p.classList.toggle("hidden", p.dataset.panel !== name);
    });
    const url = new URL(window.location.href);
    url.searchParams.set("tab", name);
    window.history.replaceState({}, "", url.toString());
    if (!loaded.has(name)) {
      loaded.add(name);
      pageState.set(name, 1);
      LOADERS[name] && LOADERS[name]();
    }
  }

  // ─── Posts loader ───────────────────────────────────────────────
  function loadPosts() {
    const panel = document.getElementById("community-profile-panel-posts");
    paginatedLoad(panel, "posts", () => {
      const page = pageState.get("posts");
      return fetchJson(`/api/community/profile/${PROFILE_ID}/posts?page=${page}`);
    }, (data) => {
      return (data.posts || []).map(renderPostCard);
    }, "No posts yet.", "Failed to load posts.");
  }

  function renderPostCard(p) {
    const card = document.createElement("article");
    card.className = "feed-post feed-post--client";
    const head = document.createElement("div");
    head.className = "feed-post-header";
    const av = document.createElement("div");
    av.className = "feed-post-avatar-circle";
    if (p.author_avatar) {
      const img = document.createElement("img");
      img.src = p.author_avatar;
      img.alt = "";
      img.className = "feed-post-avatar-circle__img";
      av.replaceChildren(img);
      av.classList.add("feed-post-avatar-circle--photo");
    } else {
      av.textContent = (p.author_name || "U").charAt(0).toUpperCase();
    }
    head.appendChild(av);
    const meta = document.createElement("div");
    meta.className = "feed-post-meta";
    const nm = document.createElement("div");
    nm.className = "feed-post-name";
    nm.textContent = p.author_name || "";
    const tm = document.createElement("div");
    tm.className = "feed-post-time";
    tm.textContent = p.created_at_display || timeAgo(p.created_at);
    meta.append(nm, tm);
    head.appendChild(meta);
    card.appendChild(head);

    const body = document.createElement("div");
    body.className = "feed-post-body";
    const para = document.createElement("p");
    para.textContent = p.content || "";
    body.appendChild(para);
    if (p.image_urls && p.image_urls.length) {
      const grid = document.createElement("div");
      grid.className = "feed-post-image-grid feed-post-image-grid--" + p.image_urls.length;
      p.image_urls.forEach((u) => {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "feed-post-image-grid__item";
        const i = document.createElement("img");
        i.src = u;
        i.alt = "";
        btn.appendChild(i);
        grid.appendChild(btn);
      });
      body.appendChild(grid);
    }
    card.appendChild(body);

    const foot = document.createElement("div");
    foot.className = "feed-post-engagement feed-post-engagement--client";
    const r = document.createElement("span");
    r.className = "feed-post-engagement__stat";
    r.textContent = `🔥 ${p.reaction_count || 0}`;
    const c = document.createElement("span");
    c.className = "feed-post-engagement__stat";
    c.textContent = `💬 ${p.comment_count || 0}`;
    const link = document.createElement("a");
    link.href = "/community/post/" + p.id;
    link.className = "feed-post-engagement__stat";
    link.textContent = "View →";
    foot.append(r, c, link);
    card.appendChild(foot);
    return card;
  }

  // ─── Comments loader ────────────────────────────────────────────
  function loadComments() {
    const panel = document.getElementById("community-profile-panel-comments");
    paginatedLoad(panel, "comments", () => {
      const page = pageState.get("comments");
      return fetchJson(`/api/community/profile/${PROFILE_ID}/comments?page=${page}`);
    }, (data) => (data.comments || []).map(renderCommentRow),
      "No comments yet.", "Failed to load comments.");
  }

  function renderCommentRow(c) {
    const row = document.createElement("a");
    row.className = "community-profile-comment-row ds-card";
    row.href = "/community/post/" + c.post_id + "#comment-" + c.id;
    const text = document.createElement("div");
    text.className = "community-profile-comment-row__text";
    text.textContent = c.content;
    const meta = document.createElement("div");
    meta.className = "community-profile-comment-row__meta";
    meta.textContent = `on "${c.post_snippet || ''}" · ${timeAgo(c.created_at)}`;
    row.append(text, meta);
    return row;
  }

  // ─── Followers / Following loaders ──────────────────────────────
  function loadFollowers() {
    loadRelationship("followers");
  }
  function loadFollowing() {
    loadRelationship("following");
  }
  function loadRelationship(direction) {
    const panel = document.getElementById("community-profile-panel-" + direction);
    paginatedLoad(panel, direction, () => {
      const page = pageState.get(direction);
      return fetchJson(`/api/community/profile/${PROFILE_ID}/${direction}?page=${page}`);
    }, (data) => (data.users || []).map(renderRelationshipRow),
      direction === "followers" ? "No followers yet." : "Not following anyone yet.",
      "Failed to load list.");
  }

  function renderRelationshipRow(u) {
    const row = document.createElement("div");
    row.className = "community-relationship-row";
    const left = document.createElement("button");
    left.type = "button";
    left.className = "community-relationship-row__user";
    left.addEventListener("click", () => {
      window.location.href = "/community/u/" + u.user_id;
    });
    const av = document.createElement("div");
    av.className = "community-relationship-row__avatar";
    if (u.avatar_url) {
      const img = document.createElement("img");
      img.src = u.avatar_url;
      img.alt = "";
      av.appendChild(img);
    } else {
      av.textContent = getInitials(u.display_name);
      av.classList.add("community-relationship-row__avatar--initials");
    }
    const nm = document.createElement("div");
    nm.className = "community-relationship-row__name";
    nm.textContent = u.display_name;
    left.append(av, nm);
    row.appendChild(left);
    if (!u.is_self) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "ds-btn ds-btn--sm " + (u.is_following ? "ds-btn--secondary" : "ds-btn--primary");
      btn.textContent = u.is_following ? "Unfollow" : "Follow";
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        if (typeof window.toggleFollow === "function") {
          window.toggleFollow(u.user_id, u.is_following, btn);
        }
      });
      row.appendChild(btn);
    }
    return row;
  }

  // ─── Media loader ───────────────────────────────────────────────
  function loadMedia() {
    const panel = document.getElementById("community-profile-panel-media");
    paginatedLoad(panel, "media", () => {
      const page = pageState.get("media");
      return fetchJson(`/api/community/profile/${PROFILE_ID}/media?page=${page}`);
    }, (data) => (data.media || []).map(renderMediaTile),
      "No images yet.", "Failed to load media.");
  }

  function renderMediaTile(m) {
    const wrap = document.createElement("a");
    wrap.className = "community-profile-media-tile";
    wrap.href = "/community/post/" + m.post_id;
    const img = document.createElement("img");
    img.src = m.url;
    img.alt = "";
    img.loading = "lazy";
    wrap.appendChild(img);
    return wrap;
  }

  // ─── Activity loader ────────────────────────────────────────────
  function loadActivity() {
    const panel = document.getElementById("community-profile-panel-activity");
    paginatedLoad(panel, "activity", () => {
      const page = pageState.get("activity");
      return fetchJson(`/api/community/profile/${PROFILE_ID}/activity?page=${page}`);
    }, (data) => (data.entries || []).map(renderActivityRow),
      "No activity yet.", "Failed to load activity.");
  }

  function renderActivityRow(entry) {
    const row = document.createElement("div");
    row.className = "community-profile-activity-row";
    const icon = document.createElement("div");
    icon.className = "community-profile-activity-row__icon";
    if (entry.kind === "post") icon.textContent = "📝";
    else if (entry.kind === "comment") icon.textContent = "💬";
    else if (entry.kind === "xp") icon.textContent = "⚡";
    else icon.textContent = "•";
    const body = document.createElement("div");
    body.className = "community-profile-activity-row__body";
    const label = document.createElement("div");
    label.className = "community-profile-activity-row__label";
    if (entry.kind === "post") label.textContent = "Posted: " + (entry.detail || "");
    else if (entry.kind === "comment") label.textContent = "Commented: " + (entry.detail || "");
    else if (entry.kind === "xp") label.textContent = entry.detail || "";
    else label.textContent = entry.detail || entry.kind;
    const ts = document.createElement("div");
    ts.className = "community-profile-activity-row__time";
    ts.textContent = timeAgo(entry.created_at);
    body.append(label, ts);
    row.append(icon, body);
    if (entry.kind === "post" && entry.entity_id) {
      const link = document.createElement("a");
      link.href = "/community/post/" + entry.entity_id;
      link.textContent = "View →";
      link.className = "community-profile-activity-row__link";
      row.appendChild(link);
    }
    return row;
  }

  // ─── Analytics loader (own only) ────────────────────────────────
  async function loadAnalytics() {
    const panel = document.getElementById("community-profile-panel-analytics");
    if (!panel) return;
    try {
      const data = await fetchJson("/api/community/profile/me/analytics");
      panel.replaceChildren(buildAnalytics(data));
    } catch (err) {
      console.error(err);
      errorState(panel, "Failed to load analytics.");
    }
  }

  function buildAnalytics(data) {
    const grid = document.createElement("div");
    grid.className = "community-profile-analytics";
    const cards = [
      { label: "Posts (30d)",     value: data.posts_30d || 0 },
      { label: "Reactions",       value: data.reactions_received_30d || 0 },
      { label: "Comments",        value: data.comments_received_30d || 0 },
      { label: "XP earned",       value: data.xp_earned_30d || 0 },
      { label: "Profile views",   value: data.profile_views_30d || 0 },
    ];
    cards.forEach((c) => {
      const card = document.createElement("div");
      card.className = "ds-card community-profile-analytics-card";
      const v = document.createElement("div");
      v.className = "community-profile-analytics-card__value";
      v.textContent = Number(c.value).toLocaleString();
      const l = document.createElement("div");
      l.className = "community-profile-analytics-card__label";
      l.textContent = c.label;
      card.append(v, l);
      grid.appendChild(card);
    });
    const wrap = document.createDocumentFragment();
    wrap.appendChild(grid);
    if (data.top_post) {
      const top = document.createElement("div");
      top.className = "ds-card community-profile-analytics-top";
      top.innerHTML = "<h3 class='ds-text-md'>Top post (30d)</h3>";
      const snippet = document.createElement("p");
      snippet.textContent = (data.top_post.content_snippet || "") + " — " + (data.top_post.reaction_count || 0) + " reactions";
      const link = document.createElement("a");
      link.href = "/community/post/" + data.top_post.post_id;
      link.className = "ds-btn ds-btn--secondary ds-btn--sm";
      link.textContent = "Open post →";
      top.append(snippet, link);
      wrap.appendChild(top);
    }
    return wrap;
  }

  // ─── Pagination helper ──────────────────────────────────────────
  async function paginatedLoad(panel, key, fetcher, rowFactory, emptyText, errorText) {
    const isFirst = pageState.get(key) === 1;
    if (isFirst) {
      panel.innerHTML = "";
    }
    try {
      const data = await fetcher();
      const rows = rowFactory(data);
      if (rows.length === 0 && isFirst) {
        emptyState(panel, emptyText, "Check back later.");
        return;
      }
      // Remove any previous load-more button.
      const oldBtn = panel.querySelector(".community-profile-load-more");
      if (oldBtn) oldBtn.remove();
      rows.forEach((r) => panel.appendChild(r));
      if (data.has_more) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "ds-btn ds-btn--secondary ds-btn--sm community-profile-load-more";
        btn.textContent = "Load more";
        btn.addEventListener("click", () => {
          pageState.set(key, (data.page || pageState.get(key)) + 1);
          btn.disabled = true;
          btn.textContent = "Loading…";
          paginatedLoad(panel, key, fetcher, rowFactory, emptyText, errorText);
        });
        panel.appendChild(btn);
      }
    } catch (err) {
      console.error(err);
      errorState(panel, errorText);
    }
  }

  const LOADERS = {
    posts: loadPosts,
    comments: loadComments,
    followers: loadFollowers,
    following: loadFollowing,
    media: loadMedia,
    activity: loadActivity,
    analytics: loadAnalytics,
    circle: () => {},
    settings: () => {},
  };

  // ─── Bootstrap ──────────────────────────────────────────────────
  function init() {
    document.querySelectorAll(".community-profile-tab").forEach((tab) => {
      tab.addEventListener("click", () => setActiveTab(tab.dataset.tab));
    });

    // Follow button uses the helper from community-feed.js.
    window.communityProfile = {
      toggleFollow() {
        const btn = document.getElementById("community-profile-follow-btn");
        if (!btn) return;
        const currentlyFollowing = btn.textContent.trim() === "Unfollow";
        if (typeof window.toggleFollow === "function") {
          window.toggleFollow(PROFILE_ID, currentlyFollowing, btn);
        }
      },
    };

    // URL-driven tab activation.
    const urlTab = new URLSearchParams(window.location.search).get("tab");
    const target = urlTab && document.querySelector(`.community-profile-tab[data-tab="${urlTab}"]`)
      ? urlTab
      : "posts";
    setActiveTab(target);

    // Followers / following stat cells become buttons that switch to the
    // matching tab.
    document.querySelectorAll(".community-profile-stat[data-relationship]").forEach((cell) => {
      cell.style.cursor = "pointer";
      cell.addEventListener("click", () => setActiveTab(cell.dataset.relationship));
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
