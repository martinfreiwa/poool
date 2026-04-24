(function () {
  "use strict";

  const state = {
    articles: [],
    categories: [],
    authors: [],
    overview: {},
    filteredArticles: [],
    selectedArticle: null,
    studioUrl: "",
    loading: false,
    isEditorPage: false,
    editorDirty: false,
  };

  document.addEventListener("DOMContentLoaded", () => {
    state.isEditorPage = document.body?.dataset.blogEditorPage === "true";
    bindTabs();
    bindFilters();
    bindEditor();
    bindTaxonomy();
    bindImport();
    loadDashboard();
  });

  function bindTabs() {
    document.querySelectorAll("[data-blog-tab]").forEach((button) => {
      button.addEventListener("click", () => activateTab(button.dataset.blogTab));
    });
  }

  function bindFilters() {
    ["blog-search", "blog-category-filter", "blog-author-filter", "blog-status-filter", "blog-translation-filter"].forEach((id) => {
      const el = byId(id);
      if (!el) return;
      el.addEventListener("input", loadArticles);
      el.addEventListener("change", loadArticles);
    });
    byId("blog-refresh-btn")?.addEventListener("click", loadDashboard);
    byId("blog-clear-filters-btn")?.addEventListener("click", clearFilters);
  }

  function bindEditor() {
    byId("blog-editor-form")?.addEventListener("submit", async (event) => {
      event.preventDefault();
      await saveDraft();
    });
    byId("blog-title")?.addEventListener("input", maybeSuggestSlug);
    ["blog-title", "blog-slug", "blog-excerpt", "blog-body", "blog-author", "blog-category", "blog-cover-url", "blog-tags", "blog-status", "blog-published-at", "blog-meta-title", "blog-meta-description"].forEach((id) => {
      byId(id)?.addEventListener("input", () => {
        markEditorDirty();
        renderEditorQuality();
      });
      byId(id)?.addEventListener("change", () => {
        markEditorDirty();
        renderEditorQuality();
      });
    });
    byId("blog-body")?.addEventListener("input", renderPreview);
    byId("blog-cover-file")?.addEventListener("change", uploadCoverImage);
    document.querySelectorAll("[data-format]").forEach((button) => {
      button.addEventListener("click", () => applyBodyFormat(button.dataset.format));
    });
    byId("blog-publish-btn")?.addEventListener("click", () => confirmArticleAction("publish"));
    byId("blog-unpublish-btn")?.addEventListener("click", () => confirmArticleAction("unpublish"));
    byId("blog-archive-btn")?.addEventListener("click", () => confirmArticleAction("archive"));
    byId("blog-restore-btn")?.addEventListener("click", () => confirmArticleAction("restore"));
    window.addEventListener("beforeunload", (event) => {
      if (!state.editorDirty) return;
      event.preventDefault();
      event.returnValue = "";
    });
  }

  function bindTaxonomy() {
    byId("blog-author-form")?.addEventListener("submit", async (event) => {
      event.preventDefault();
      await saveAuthor();
    });
    byId("blog-category-form")?.addEventListener("submit", async (event) => {
      event.preventDefault();
      await saveCategory();
    });
  }

  function bindImport() {
    byId("blog-import-dry-run-btn")?.addEventListener("click", () => runImport(true));
    byId("blog-import-run-btn")?.addEventListener("click", () => {
      if (window.confirm("Import legacy DB articles into Sanity now? This can create or update CMS documents.")) {
        runImport(false);
      }
    });
  }

  async function loadDashboard() {
    clearAlert();
    setLoading(true);
    try {
      const [overviewResp, articlesResp, authorsResp, categoriesResp] = await Promise.all([
        apiGet("/api/admin/blog/overview"),
        apiGet("/api/admin/blog/articles?per_page=100"),
        apiGet("/api/admin/blog/authors"),
        apiGet("/api/admin/blog/categories"),
      ]);

      state.overview = overviewResp.overview || {};
      state.articles = articlesResp.articles || [];
      state.authors = authorsResp.authors || articlesResp.authors || [];
      state.categories = categoriesResp.categories || articlesResp.categories || [];
      state.studioUrl = articlesResp.studio_url || state.overview.studio_url || "";

      renderAll();
      if (state.isEditorPage) {
        const id = new URLSearchParams(window.location.search).get("id");
        if (id) {
          await selectArticle(id);
        } else if (!state.selectedArticle) {
          selectNewArticle();
        }
      }
    } catch (error) {
      showAlert(error.message || "Failed to load Blog CMS.", "error");
      renderEmptyTable("blog-article-table", 8, "Unable to load Sanity articles.");
    } finally {
      setLoading(false);
    }
  }

  async function loadArticles() {
    const params = new URLSearchParams();
    params.set("per_page", "100");
    const search = byId("blog-search")?.value.trim();
    const category = byId("blog-category-filter")?.value;
    const status = byId("blog-status-filter")?.value;
    if (search) params.set("q", search);
    if (category) params.set("category", category);
    if (status) params.set("status", status);

    try {
      const data = await apiGet(`/api/admin/blog/articles?${params.toString()}`);
      state.articles = data.articles || [];
      applyLocalArticleFilters();
      renderArticles();
    } catch (error) {
      showAlert(error.message || "Failed to filter articles.", "error");
    }
  }

  function renderAll() {
    renderOverview();
    renderFilterOptions();
    renderSelectOptions();
    applyLocalArticleFilters();
    renderArticles();
    renderAuthors();
    renderCategories();
    configureTopLinks();
  }

  function renderOverview() {
    const published = state.articles.filter((a) => a.status === "published").length;
    const draftish = state.articles.filter((a) => ["draft", "changes_pending"].includes(a.status)).length;
    const missingTranslations = state.articles.filter(hasMissingTranslation).length;
    setText("blog-kpi-published", formatCount(state.overview.published_count ?? published));
    setText("blog-kpi-drafts", formatCount(state.overview.draft_count ?? draftish));
    setText("blog-kpi-categories", formatCount(state.overview.category_count ?? state.categories.length));
    setText("blog-kpi-translations", missingTranslations ? `${missingTranslations} articles missing i18n` : "Translations complete");
    setText("blog-kpi-writes", state.overview.writes_enabled ? "Enabled" : "Token needed");
    setText("blog-kpi-project", `Project ${state.overview.project_id || "--"} / ${state.overview.dataset || "--"}`);
  }

  function renderFilterOptions() {
    const select = byId("blog-category-filter");
    if (select) {
      const current = select.value;
      replaceOptions(select, [{ value: "", label: "All categories" }].concat(
        state.categories.map((c) => ({ value: c.slug || "", label: c.name || c.slug || "Untitled" })),
      ));
      select.value = current;
      rebuildPooolDropdown(select);
    }

    const authorSelect = byId("blog-author-filter");
    if (authorSelect) {
      const current = authorSelect.value;
      replaceOptions(authorSelect, [{ value: "", label: "All authors" }].concat(
        state.authors.map((a) => ({ value: a.id || "", label: a.name || a.slug || "Untitled" })),
      ));
      authorSelect.value = current;
      rebuildPooolDropdown(authorSelect);
    }

    rebuildPooolDropdown(byId("blog-status-filter"));
    rebuildPooolDropdown(byId("blog-translation-filter"));
  }

  function renderSelectOptions() {
    replaceOptions(byId("blog-author"), [{ value: "", label: "No author" }].concat(
      state.authors.map((a) => ({ value: a.id, label: a.name || a.slug || "Untitled" })),
    ));
    replaceOptions(byId("blog-category"), [{ value: "", label: "No category" }].concat(
      state.categories.map((c) => ({ value: c.id, label: c.name || c.slug || "Untitled" })),
    ));
    rebuildPooolDropdown(byId("blog-author"));
    rebuildPooolDropdown(byId("blog-category"));
  }

  function renderArticles() {
    const tbody = byId("blog-article-table");
    if (!tbody) return;
    tbody.replaceChildren();

    const articles = state.filteredArticles.length || anyFilterActive() ? state.filteredArticles : state.articles;
    updateCountLabel(articles.length, state.articles.length);

    if (articles.length === 0) {
      renderEmptyTable("blog-article-table", 8, "No matching Sanity articles found.");
      return;
    }

    articles.forEach((article) => {
      const tr = document.createElement("tr");
      tr.appendChild(titleCell(article));
      const status = document.createElement("td");
      status.appendChild(statusBadge(article.status));
      tr.appendChild(status);
      tr.appendChild(textCell(article.category?.name || "Uncategorized"));
      tr.appendChild(textCell(article.author?.name || "Unknown"));
      tr.appendChild(translationCell(article.translationStatus || article.translation_status || []));
      tr.appendChild(textCell(formatDate(article.publishedAt || article.published_at)));
      tr.appendChild(textCell(article.slug || ""));

      const actions = document.createElement("td");
      actions.className = "blog-cms-actions-cell";
      const edit = document.createElement("a");
      edit.className = "admin-btn admin-btn--primary admin-btn--sm";
      edit.href = `/admin/blog-editor.html?id=${encodeURIComponent(article.id)}`;
      edit.textContent = "Edit";
      actions.appendChild(edit);
      if (article.slug && article.status === "published") {
        const live = document.createElement("a");
        live.className = "admin-btn admin-btn--secondary admin-btn--sm";
        live.href = `/blog/${encodeURIComponent(article.slug)}`;
        live.textContent = "Live";
        actions.appendChild(live);
      }
      tr.appendChild(actions);
      tbody.appendChild(tr);
    });
  }

  function applyLocalArticleFilters() {
    const author = byId("blog-author-filter")?.value;
    const translation = byId("blog-translation-filter")?.value;
    state.filteredArticles = state.articles.filter((article) => {
      if (author && article.author?.id !== author) return false;
      if (translation === "missing" && !hasMissingTranslation(article)) return false;
      if (translation === "complete" && hasMissingTranslation(article)) return false;
      return true;
    });
  }

  function clearFilters() {
    setValue("blog-search", "");
    setValue("blog-category-filter", "");
    setValue("blog-author-filter", "");
    setValue("blog-status-filter", "");
    setValue("blog-translation-filter", "");
    syncFilterUi();
    loadArticles();
  }

  function anyFilterActive() {
    return Boolean(
      byId("blog-search")?.value.trim()
      || byId("blog-category-filter")?.value
      || byId("blog-author-filter")?.value
      || byId("blog-status-filter")?.value
      || byId("blog-translation-filter")?.value,
    );
  }

  function hasMissingTranslation(article) {
    const translations = article.translationStatus || article.translation_status || [];
    return ["id", "de", "ru"].some((code) => !translations.find((item) => item.code === code && item.present));
  }

  function titleCell(article) {
    const td = document.createElement("td");
    const wrap = document.createElement("div");
    wrap.className = "blog-cms-title-cell";
    const title = document.createElement("span");
    title.textContent = article.title || "Untitled";
    title.style.fontWeight = "600";
    wrap.appendChild(title);
    const meta = document.createElement("div");
    meta.className = "blog-cms-title-meta";
    meta.appendChild(document.createTextNode(`${article.readingTimeMinutes || article.reading_time_minutes || 5} min read`));
    if (article.featured) {
      const featured = document.createElement("span");
      featured.className = "blog-cms-featured-pill";
      featured.textContent = "Featured";
      meta.appendChild(featured);
    }
    wrap.appendChild(meta);
    td.appendChild(wrap);
    return td;
  }

  function translationCell(translations) {
    const td = document.createElement("td");
    const wrap = document.createElement("div");
    wrap.className = "blog-cms-translations";
    const expected = [
      { code: "id", label: "Indonesian" },
      { code: "de", label: "German" },
      { code: "ru", label: "Russian" },
    ];
    expected.forEach((expectedTranslation) => {
      const translation = translations.find((item) => item.code === expectedTranslation.code) || expectedTranslation;
      const badge = document.createElement("span");
      const present = Boolean(translation.present);
      badge.className = `blog-cms-translation-badge blog-cms-translation-badge--${present ? "present" : "missing"}`;
      badge.textContent = expectedTranslation.code.toUpperCase();
      const status = translation.status ? ` (${humanStatus(translation.status)})` : "";
      badge.title = `${expectedTranslation.label}: ${present ? `available${status}` : "missing"}`;
      wrap.appendChild(badge);
    });
    td.appendChild(wrap);
    return td;
  }

  function renderAuthors() {
    const tbody = byId("blog-author-table");
    if (!tbody) return;
    tbody.replaceChildren();
    if (!state.authors.length) {
      renderEmptyTable("blog-author-table", 3, "No authors found.");
      return;
    }
    state.authors.forEach((author) => {
      const tr = document.createElement("tr");
      tr.appendChild(textCell(author.name || "Untitled", "600"));
      tr.appendChild(textCell(author.slug || ""));
      tr.appendChild(textCell(formatCount(author.article_count)));
      tr.addEventListener("click", () => fillAuthorForm(author));
      tbody.appendChild(tr);
    });
  }

  function renderCategories() {
    const tbody = byId("blog-category-table");
    if (!tbody) return;
    tbody.replaceChildren();
    if (!state.categories.length) {
      renderEmptyTable("blog-category-table", 3, "No categories found.");
      return;
    }
    state.categories.forEach((category) => {
      const tr = document.createElement("tr");
      tr.appendChild(textCell(category.name || "Untitled", "600"));
      tr.appendChild(textCell(category.slug || ""));
      tr.appendChild(textCell(formatCount(category.article_count)));
      tr.addEventListener("click", () => fillCategoryForm(category));
      tbody.appendChild(tr);
    });
  }

  async function selectArticle(id) {
    try {
      const data = await apiGet(`/api/admin/blog/articles/${encodeURIComponent(id)}`);
      fillEditor(data.article);
    } catch (error) {
      showAlert(error.message || "Failed to load article.", "error");
    }
  }

  function selectNewArticle() {
    state.selectedArticle = null;
    setText("blog-editor-title", "Create Article");
    setText("blog-editor-subtitle", "New articles are saved as Sanity drafts.");
    byId("blog-editor-form")?.reset();
    state.editorDirty = false;
    setValue("blog-article-id", "");
    setValue("blog-article-revision", "");
    setValue("blog-status", "draft");
    setValue("blog-schema-type", "BlogPosting");
    setValue("blog-cover-asset-ref", "");
    configureArticleLinks(null);
    renderEditorQuality();
    renderPreview();
  }

  function fillEditor(article) {
    state.selectedArticle = article;
    state.editorDirty = false;
    setText("blog-editor-title", article.title || "Untitled");
    setText("blog-editor-subtitle", `Status: ${humanStatus(article.status)}`);
    setValue("blog-article-id", article.id || "");
    setValue("blog-article-revision", article.revision || "");
    setValue("blog-title", article.title || "");
    setValue("blog-slug", article.slug || "");
    setValue("blog-author", article.author?.id || "");
    setValue("blog-category", article.category?.id || "");
    setValue("blog-excerpt", article.excerpt || "");
    setValue("blog-body", article.bodyText || article.body_text || "");
    setValue("blog-cover-url", article.coverImageUrl || article.cover_image_url || "");
    setValue("blog-cover-asset-ref", article.coverImageAssetRef || article.cover_image_asset_ref || "");
    const shareLinks = article.shareLinks || article.share_links || {};
    setValue("blog-share-whatsapp", shareLinks.whatsappUrl || shareLinks.whatsapp_url || "");
    setValue("blog-share-facebook", shareLinks.facebookUrl || shareLinks.facebook_url || "");
    setValue("blog-share-x", shareLinks.xUrl || shareLinks.x_url || "");
    setValue("blog-share-instagram", shareLinks.instagramUrl || shareLinks.instagram_url || "");
    setValue("blog-share-linkedin", shareLinks.linkedinUrl || shareLinks.linkedin_url || "");
    setValue("blog-tags", (article.tags || []).join(", "));
    setValue("blog-status", normalizeEditableStatus(article.status));
    setValue("blog-published-at", toLocalDateTime(article.publishedAt || article.published_at));
    setValue("blog-schema-type", article.schemaType || article.schema_type || "BlogPosting");
    setValue("blog-meta-title", article.metaTitle || article.meta_title || "");
    setValue("blog-meta-description", article.metaDescription || article.meta_description || "");
    byId("blog-featured").checked = Boolean(article.featured);
    configureArticleLinks(article);
    renderEditorQuality();
    renderEditorTranslations(article);
    renderPreview();
  }

  async function saveDraft() {
    clearAlert();
    const id = byId("blog-article-id")?.value;
    const payload = editorPayload();
    const method = id ? "PUT" : "POST";
    const url = id ? `/api/admin/blog/articles/${encodeURIComponent(id)}` : "/api/admin/blog/articles";

    try {
      const data = await apiJson(url, method, payload);
      fillEditor(data.article);
      state.editorDirty = false;
      if (state.isEditorPage) {
        const nextUrl = `/admin/blog-editor.html?id=${encodeURIComponent(data.article.id)}`;
        if (window.location.pathname.endsWith("/blog-editor.html") && window.location.search !== `?id=${encodeURIComponent(data.article.id)}`) {
          window.history.replaceState(null, "", nextUrl);
        }
      }
      showAlert("Draft saved.", "success");
      await loadDashboard();
      fillEditor(data.article);
    } catch (error) {
      showAlert(error.message || "Failed to save draft.", "error");
    }
  }

  async function articleAction(action) {
    const id = byId("blog-article-id")?.value;
    if (!id) {
      showAlert("Save the article as a draft first.", "error");
      return;
    }
    clearAlert();
    try {
      const data = await apiJson(`/api/admin/blog/articles/${encodeURIComponent(id)}/${action}`, "POST", {});
      fillEditor(data.article);
      showAlert(`Article ${humanStatus(action)}.`, "success");
      await loadDashboard();
      fillEditor(data.article);
    } catch (error) {
      showAlert(error.message || `Failed to ${action} article.`, "error");
    }
  }

  async function confirmArticleAction(action) {
    const messages = {
      publish: "Publish this article to the public blog?",
      unpublish: "Take this article down from the public blog?",
      archive: "Archive this article and hide it from normal editorial lists?",
      restore: "Restore this article as an editable draft?",
    };
    if (!window.confirm(messages[action] || `Run ${humanStatus(action)} on this article?`)) {
      return;
    }
    await articleAction(action);
  }

  function editorPayload() {
    const bodyText = byId("blog-body")?.value || "";
    return {
      title: byId("blog-title")?.value.trim() || "",
      slug: slugify(byId("blog-slug")?.value || byId("blog-title")?.value || ""),
      excerpt: byId("blog-excerpt")?.value.trim() || "",
      bodyText,
      body: portableTextFromText(bodyText),
      authorId: byId("blog-author")?.value || null,
      categoryId: byId("blog-category")?.value || null,
      tags: splitTags(byId("blog-tags")?.value || ""),
      status: byId("blog-status")?.value || "draft",
      publishedAt: fromLocalDateTime(byId("blog-published-at")?.value),
      schemaType: byId("blog-schema-type")?.value || "BlogPosting",
      metaTitle: byId("blog-meta-title")?.value.trim() || null,
      metaDescription: byId("blog-meta-description")?.value.trim() || null,
      coverImageUrl: byId("blog-cover-url")?.value.trim() || null,
      coverImageAssetRef: byId("blog-cover-asset-ref")?.value || null,
      shareLinks: {
        whatsappUrl: byId("blog-share-whatsapp")?.value.trim() || null,
        facebookUrl: byId("blog-share-facebook")?.value.trim() || null,
        xUrl: byId("blog-share-x")?.value.trim() || null,
        instagramUrl: byId("blog-share-instagram")?.value.trim() || null,
        linkedinUrl: byId("blog-share-linkedin")?.value.trim() || null,
      },
      featured: Boolean(byId("blog-featured")?.checked),
      revision: byId("blog-article-revision")?.value || null,
    };
  }

  async function uploadCoverImage() {
    const file = byId("blog-cover-file")?.files?.[0];
    if (!file) return;
    const form = new FormData();
    form.append("file", file);
    try {
      const resp = await fetch("/api/admin/blog/assets", { method: "POST", body: form });
      const data = await parseResponse(resp);
      const asset = data.asset?.document || data.asset;
      if (asset?._id) setValue("blog-cover-asset-ref", asset._id);
      if (asset?.url) setValue("blog-cover-url", asset.url);
      showAlert("Cover image uploaded.", "success");
    } catch (error) {
      showAlert(error.message || "Image upload failed.", "error");
    }
  }

  async function saveAuthor() {
    const payload = {
      id: byId("taxonomy-author-id")?.value || null,
      name: byId("taxonomy-author-name")?.value.trim() || "",
      slug: slugify(byId("taxonomy-author-slug")?.value || byId("taxonomy-author-name")?.value || ""),
      bio: byId("taxonomy-author-bio")?.value.trim() || null,
      avatarUrl: byId("taxonomy-author-avatar")?.value.trim() || null,
      websiteUrl: byId("taxonomy-author-website")?.value.trim() || null,
      twitterHandle: normalizeHandle(byId("taxonomy-author-twitter")?.value || ""),
      linkedinUrl: byId("taxonomy-author-linkedin")?.value.trim() || null,
      facebookUrl: byId("taxonomy-author-facebook")?.value.trim() || null,
      instagramUrl: byId("taxonomy-author-instagram")?.value.trim() || null,
      whatsapp: normalizePhone(byId("taxonomy-author-whatsapp")?.value || ""),
      expertise: splitTags(byId("taxonomy-author-expertise")?.value || ""),
    };
    try {
      await apiJson("/api/admin/blog/authors", "POST", payload);
      byId("blog-author-form")?.reset();
      await loadDashboard();
      showAlert("Author saved.", "success");
    } catch (error) {
      showAlert(error.message || "Failed to save author.", "error");
    }
  }

  async function saveCategory() {
    const payload = {
      id: byId("taxonomy-category-id")?.value || null,
      name: byId("taxonomy-category-name")?.value.trim() || "",
      slug: slugify(byId("taxonomy-category-slug")?.value || byId("taxonomy-category-name")?.value || ""),
      description: byId("taxonomy-category-description")?.value.trim() || null,
      color: byId("taxonomy-category-color")?.value.trim() || null,
      sortOrder: parseInt(byId("taxonomy-category-order")?.value || "0", 10),
    };
    try {
      await apiJson("/api/admin/blog/categories", "POST", payload);
      byId("blog-category-form")?.reset();
      await loadDashboard();
      showAlert("Category saved.", "success");
    } catch (error) {
      showAlert(error.message || "Failed to save category.", "error");
    }
  }

  async function runImport(dryRun) {
    const target = byId("blog-import-result");
    if (target) target.textContent = dryRun ? "Running dry run..." : "Importing...";
    try {
      const path = dryRun
        ? "/api/admin/blog/import/db-to-sanity/dry-run"
        : "/api/admin/blog/import/db-to-sanity";
      const data = await apiJson(path, "POST", {});
      if (target) target.textContent = JSON.stringify(data.import, null, 2);
      if (!dryRun) await loadDashboard();
    } catch (error) {
      if (target) target.textContent = error.message || "Import failed.";
    }
  }

  function fillAuthorForm(author) {
    setValue("taxonomy-author-id", author.id || "");
    setValue("taxonomy-author-name", author.name || "");
    setValue("taxonomy-author-slug", author.slug || "");
    setValue("taxonomy-author-bio", author.bio || "");
    setValue("taxonomy-author-avatar", author.avatar_url || "");
    setValue("taxonomy-author-website", author.website_url || author.websiteUrl || "");
    setValue("taxonomy-author-twitter", author.twitter_handle || author.twitterHandle || "");
    setValue("taxonomy-author-linkedin", author.linkedin_url || author.linkedinUrl || "");
    setValue("taxonomy-author-facebook", author.facebook_url || author.facebookUrl || "");
    setValue("taxonomy-author-instagram", author.instagram_url || author.instagramUrl || "");
    setValue("taxonomy-author-whatsapp", author.whatsapp || "");
    setValue("taxonomy-author-expertise", (author.expertise || []).join(", "));
  }

  function fillCategoryForm(category) {
    setValue("taxonomy-category-id", category.id || "");
    setValue("taxonomy-category-name", category.name || "");
    setValue("taxonomy-category-slug", category.slug || "");
    setValue("taxonomy-category-description", category.description || "");
    setValue("taxonomy-category-color", category.color || "");
    setValue("taxonomy-category-order", category.sort_order || "");
  }

  function renderPreview() {
    const target = byId("blog-preview");
    if (!target) return;
    target.replaceChildren();
    portableTextFromText(byId("blog-body")?.value || "").forEach((block) => {
      const text = block.children?.map((child) => child.text || "").join("") || "";
      const tag = block.style === "h2" ? "h2" : block.style === "h3" ? "h3" : block.style === "blockquote" ? "blockquote" : "p";
      const el = document.createElement(tag);
      if (block.listItem) {
        el.textContent = `${block.listItem === "number" ? "1." : "•"} ${text}`;
        target.appendChild(el);
        return;
      }
      el.textContent = text;
      target.appendChild(el);
    });
    renderEditorQuality();
  }

  function renderEditorQuality() {
    if (!state.isEditorPage) return;
    const title = byId("blog-title")?.value.trim() || "";
    const excerpt = byId("blog-excerpt")?.value.trim() || "";
    const body = byId("blog-body")?.value || "";
    const metaTitle = byId("blog-meta-title")?.value.trim() || "";
    const metaDescription = byId("blog-meta-description")?.value.trim() || "";
    const wordCount = countWords(body);
    const readingMinutes = Math.max(0, Math.ceil(wordCount / 220));

    setText("blog-excerpt-counter", `${excerpt.length} characters`);
    setText("blog-body-counter", `${wordCount.toLocaleString()} words`);
    setText("blog-reading-time", `${readingMinutes} min read`);
    setText("blog-meta-title-counter", `${metaTitle.length} / 70`);
    setText("blog-meta-description-counter", `${metaDescription.length} / 160`);
    setText("blog-editor-status-summary", humanStatus(byId("blog-status")?.value || state.selectedArticle?.status || "draft"));
    setText("blog-editor-length-summary", `${wordCount.toLocaleString()} words`);

    updateMeter("blog-meta-title-meter", metaTitle.length, 70, metaTitle.length > 60);
    updateMeter("blog-meta-description-meter", metaDescription.length, 160, metaDescription.length > 155);

    const seoReady = metaTitle.length >= 45 && metaTitle.length <= 60 && metaDescription.length >= 120 && metaDescription.length <= 155;
    setText("blog-editor-seo-summary", seoReady ? "Ready" : "Needs work");

    const checks = [
      { label: "Title is present", ok: title.length >= 8 },
      { label: "Slug is present", ok: Boolean(byId("blog-slug")?.value.trim()) },
      { label: "Excerpt is usable", ok: excerpt.length >= 80, warn: excerpt.length >= 40 },
      { label: "Body has enough depth", ok: wordCount >= 600, warn: wordCount >= 300 },
      { label: "Author and category selected", ok: Boolean(byId("blog-author")?.value && byId("blog-category")?.value) },
      { label: "Cover image configured", ok: Boolean(byId("blog-cover-url")?.value.trim() || byId("blog-cover-asset-ref")?.value) },
      { label: "SEO metadata in range", ok: seoReady, warn: metaTitle.length > 0 || metaDescription.length > 0 },
    ];
    renderHealthChecks(checks);
  }

  function renderEditorTranslations(article) {
    const target = byId("blog-editor-translation-list");
    if (!target) return;
    const translations = article?.translationStatus || article?.translation_status || [];
    target.replaceChildren();
    [
      { code: "id", label: "Indonesian" },
      { code: "de", label: "German" },
      { code: "ru", label: "Russian" },
    ].forEach((expected) => {
      const translation = translations.find((item) => item.code === expected.code);
      const row = document.createElement("div");
      row.className = "blog-editor-translation-row";
      const label = document.createElement("span");
      label.textContent = expected.label;
      const badge = document.createElement("span");
      const present = Boolean(translation?.present);
      badge.className = `admin-badge ${present ? "admin-badge--success" : "admin-badge--warning"}`;
      badge.textContent = present ? humanStatus(translation.status || "available") : "Missing";
      row.append(label, badge);
      target.appendChild(row);
    });
  }

  function renderHealthChecks(checks) {
    const target = byId("blog-editor-health-list");
    if (!target) return;
    target.replaceChildren();
    checks.forEach((check) => {
      const item = document.createElement("li");
      item.className = `blog-editor-health-item ${check.ok ? "is-ok" : check.warn ? "is-warn" : "is-danger"}`;
      const dot = document.createElement("span");
      dot.className = "blog-editor-health-dot";
      const text = document.createElement("span");
      text.textContent = check.label;
      item.append(dot, text);
      target.appendChild(item);
    });
  }

  function updateMeter(id, value, max, warning) {
    const meter = byId(id);
    if (!meter) return;
    const pct = Math.min(100, Math.round((value / max) * 100));
    meter.style.width = `${pct}%`;
    meter.classList.toggle("is-danger", value > max);
    meter.classList.toggle("is-warn", !value || warning);
  }

  function applyBodyFormat(format) {
    const textarea = byId("blog-body");
    if (!textarea) return;
    const start = textarea.selectionStart || 0;
    const end = textarea.selectionEnd || 0;
    const selected = textarea.value.slice(start, end);
    const fallback = selected || "Text";
    let replacement = fallback;

    if (format === "h2") replacement = prefixLines(fallback, "## ");
    if (format === "h3") replacement = prefixLines(fallback, "### ");
    if (format === "quote") replacement = prefixLines(fallback, "> ");
    if (format === "bullet") replacement = prefixLines(fallback, "- ");
    if (format === "number") replacement = prefixLines(fallback, "1. ");
    if (format === "bold") replacement = `**${fallback}**`;
    if (format === "italic") replacement = `*${fallback}*`;
    if (format === "code") replacement = selected.includes("\n") ? `\n\`\`\`\n${fallback}\n\`\`\`\n` : `\`${fallback}\``;
    if (format === "link") {
      const href = window.prompt("URL");
      if (!href) return;
      replacement = `[${fallback}](${href})`;
    }

    textarea.setRangeText(replacement, start, end, "end");
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
    textarea.focus();
  }

  function configureTopLinks() {
    const href = state.studioUrl || "https://www.sanity.io/";
    byId("blog-studio-link")?.setAttribute("href", href);
  }

  function configureArticleLinks(article) {
    const live = byId("blog-view-live-link");
    const sanity = byId("blog-open-sanity-link");
    if (live) {
      live.href = article?.slug ? `/blog/${encodeURIComponent(article.slug)}` : "#";
      live.style.pointerEvents = article?.slug ? "" : "none";
    }
    if (sanity) {
      sanity.href = article?.studioUrl || article?.studio_url || state.studioUrl || "https://www.sanity.io/";
    }
  }

  async function apiGet(url) {
    const resp = await fetch(url);
    return parseResponse(resp);
  }

  async function apiJson(url, method, payload) {
    const resp = await fetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: method === "GET" ? undefined : JSON.stringify(payload || {}),
    });
    return parseResponse(resp);
  }

  async function parseResponse(resp) {
    const data = await resp.json().catch(() => ({}));
    if (!resp.ok) {
      throw new Error(data.error || `Request failed (${resp.status})`);
    }
    return data;
  }

  function portableTextFromText(text) {
    return text
      .split(/\n+/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        let style = "normal";
        let listItem = null;
        let content = line;
        if (line.startsWith("#### ")) {
          style = "h4";
          content = line.slice(5);
        } else if (line.startsWith("### ")) {
          style = "h3";
          content = line.slice(4);
        } else if (line.startsWith("## ")) {
          style = "h2";
          content = line.slice(3);
        } else if (line.startsWith("> ")) {
          style = "blockquote";
          content = line.slice(2);
        } else if (line.startsWith("- ")) {
          listItem = "bullet";
          content = line.slice(2);
        } else if (/^\d+\.\s+/.test(line)) {
          listItem = "number";
          content = line.replace(/^\d+\.\s+/, "");
        }
        const parsed = parseInlineMarks(content);
        return {
          _type: "block",
          _key: crypto.randomUUID ? crypto.randomUUID().replaceAll("-", "") : String(Date.now() + Math.random()),
          style,
          ...(listItem ? { listItem, level: 1 } : {}),
          markDefs: parsed.markDefs,
          children: parsed.children,
        };
      });
  }

  function parseInlineMarks(text) {
    const children = [];
    const markDefs = [];
    const pattern = /\[([^\]]+)\]\((https?:\/\/[^\s)]+|mailto:[^\s)]+|tel:[^\s)]+)\)|\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`/g;
    let index = 0;
    let match;
    while ((match = pattern.exec(text)) !== null) {
      if (match.index > index) {
        children.push(span(text.slice(index, match.index), []));
      }
      if (match[1] && match[2]) {
        const key = `link${markDefs.length + 1}`;
        markDefs.push({ _key: key, _type: "link", href: match[2], blank: true });
        children.push(span(match[1], [key]));
      } else if (match[3]) {
        children.push(span(match[3], ["strong"]));
      } else if (match[4]) {
        children.push(span(match[4], ["em"]));
      } else if (match[5]) {
        children.push(span(match[5], ["code"]));
      }
      index = pattern.lastIndex;
    }
    if (index < text.length) {
      children.push(span(text.slice(index), []));
    }
    return { markDefs, children: children.length ? children : [span(text, [])] };
  }

  function span(text, marks) {
    return {
      _type: "span",
      _key: crypto.randomUUID ? crypto.randomUUID().replaceAll("-", "") : String(Date.now() + Math.random()),
      text,
      marks,
    };
  }

  function maybeSuggestSlug() {
    const slug = byId("blog-slug");
    if (!slug || slug.value || state.selectedArticle) return;
    slug.value = slugify(byId("blog-title")?.value || "");
  }

  function splitTags(value) {
    return value.split(",").map((tag) => tag.trim()).filter(Boolean);
  }

  function prefixLines(text, prefix) {
    return text.split("\n").map((line) => line.startsWith(prefix) ? line : `${prefix}${line}`).join("\n");
  }

  function countWords(text) {
    return String(text || "").trim().split(/\s+/).filter(Boolean).length;
  }

  function normalizeHandle(value) {
    const handle = String(value || "").trim().replace(/^@+/, "");
    return handle || null;
  }

  function normalizePhone(value) {
    const phone = String(value || "").trim().replace(/[^\d+]/g, "");
    return phone || null;
  }

  function slugify(value) {
    return String(value)
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function activateTab(tabName) {
    document.querySelectorAll("[data-blog-tab]").forEach((button) => {
      button.classList.toggle("active", button.dataset.blogTab === tabName);
    });
    document.querySelectorAll(".blog-cms-tab-panel").forEach((panel) => {
      panel.hidden = panel.id !== `blog-tab-${tabName}`;
    });
  }

  function replaceOptions(select, options) {
    if (!select) return;
    select.replaceChildren();
    options.forEach((item) => {
      const option = document.createElement("option");
      option.value = item.value == null ? "" : item.value;
      option.textContent = item.label;
      select.appendChild(option);
    });
  }

  function rebuildPooolDropdown(select) {
    if (!select || !window.PooolDropdown || !select.classList.contains("admin-select")) return;
    const wrapper = select.closest(".poool-dropdown");
    if (wrapper && wrapper.parentNode) {
      wrapper.parentNode.insertBefore(select, wrapper);
      select.style.display = "";
      wrapper.remove();
    }
    window.PooolDropdown.fromSelect(select, {
      placeholder: select.options[0] ? select.options[0].textContent : "Select...",
      noLabel: true,
      searchable: select.hasAttribute("data-searchable"),
      className: "poool-dropdown--sm poool-dropdown--inline",
    });
  }

  function statusBadge(status) {
    const span = document.createElement("span");
    span.className = `blog-cms-status blog-cms-status--${status || "draft"}`;
    span.textContent = humanStatus(status);
    return span;
  }

  function humanStatus(status) {
    return String(status || "draft").replaceAll("_", " ");
  }

  function normalizeEditableStatus(status) {
    if (status === "published" || status === "changes_pending") return "draft";
    return status || "draft";
  }

  function textCell(text, weight) {
    const td = document.createElement("td");
    td.textContent = text == null ? "" : String(text);
    if (weight) td.style.fontWeight = weight;
    return td;
  }

  function updateCountLabel(showing, total) {
    const label = byId("blog-count-label");
    if (label) {
      label.textContent = showing === total ? `${formatCount(total)} articles` : `${formatCount(showing)} of ${formatCount(total)} articles`;
    }
    syncFilterUi();
  }

  function syncFilterUi() {
    const clear = byId("blog-clear-filters-btn");
    if (clear) clear.hidden = !anyFilterActive();
  }

  function renderEmptyTable(tableId, colspan, message) {
    const tbody = byId(tableId);
    if (!tbody) return;
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = colspan;
    td.style.textAlign = "center";
    td.style.padding = "32px";
    td.style.color = "var(--admin-text-muted)";
    td.textContent = message;
    tr.appendChild(td);
    tbody.replaceChildren(tr);
  }

  function showAlert(message, type) {
    const alert = byId("blog-alert");
    if (!alert) return;
    alert.textContent = message;
    alert.style.display = "";
    alert.style.color = type === "error" ? "var(--admin-danger)" : "var(--admin-success, #027a48)";
  }

  function clearAlert() {
    const alert = byId("blog-alert");
    if (!alert) return;
    alert.textContent = "";
    alert.style.display = "none";
  }

  function setLoading(isLoading) {
    state.loading = isLoading;
    const refresh = byId("blog-refresh-btn");
    if (refresh) {
      refresh.disabled = isLoading;
      refresh.textContent = isLoading ? "Refreshing..." : "Refresh";
    }
  }

  function markEditorDirty() {
    if (!state.isEditorPage) return;
    state.editorDirty = true;
  }

  function byId(id) {
    return document.getElementById(id);
  }

  function setText(id, value) {
    const el = byId(id);
    if (el) el.textContent = value == null ? "--" : String(value);
  }

  function setValue(id, value) {
    const el = byId(id);
    if (!el) return;
    el.value = value == null ? "" : String(value);
    el.closest(".poool-dropdown")?._pooolDropdown?.setValue(el.value);
  }

  function formatCount(value) {
    return typeof value === "number" ? value.toLocaleString() : "--";
  }

  function formatDate(value) {
    if (!value) return "--";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }

  function toLocalDateTime(value) {
    if (!value) return "";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    const offset = date.getTimezoneOffset();
    return new Date(date.getTime() - offset * 60000).toISOString().slice(0, 16);
  }

  function fromLocalDateTime(value) {
    if (!value) return null;
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? null : date.toISOString();
  }
})();
