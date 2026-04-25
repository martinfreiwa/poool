/* ===========================
   Developer Assets Page JavaScript
   =========================== */

function safeAssetUrl(rawUrl) {
  try {
    const parsed = new URL(String(rawUrl || ""), window.location.origin);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") return parsed.href;
  } catch (_) {
    // Fall through to the placeholder.
  }
  return "/static/images/seed/villa1.webp";
}

function applyCoverImages() {
  document.querySelectorAll(".dev-asset-card .property-image[data-cover-url]").forEach((image) => {
    const url = safeAssetUrl(image.dataset.coverUrl);
    image.style.backgroundImage = `url("${url.replace(/"/g, "%22")}")`;
  });
}

function isFundedCard(card) {
  const status = (card.dataset.status || "").toLowerCase();
  const fundedStr = (card.dataset.funded || "").toLowerCase();
  const isFundedStatus = ["funded", "rented", "exited"].includes(status);
  const pct = Number.parseFloat(card.dataset.fundingPct) || 0;
  return isFundedStatus || fundedStr === "true" || pct >= 100;
}

function showDevTab(tab) {
  const selectedTab = tab === "funded" ? "funded" : "available";
  document.querySelectorAll("#dev-assets-status-tabs .status-tab").forEach((button) => {
    const isActive = button.dataset.devAssetsTab === selectedTab;
    button.classList.toggle("active", isActive);
    button.setAttribute("aria-pressed", isActive ? "true" : "false");
  });

  document.querySelectorAll("#dev-assets-grid .property-card").forEach((card) => {
    const show = selectedTab === "available" ? !isFundedCard(card) : isFundedCard(card);
    card.style.setProperty("display", show ? "flex" : "none", "important");
  });

  document.querySelectorAll("#dev-assets-grid .ghost-card").forEach((card) => {
    card.style.setProperty("display", "none", "important");
  });
}

function bindStatusTabs() {
  document.querySelectorAll("[data-dev-assets-tab]").forEach((button) => {
    button.setAttribute("aria-pressed", button.classList.contains("active") ? "true" : "false");
    button.addEventListener("click", () => showDevTab(button.dataset.devAssetsTab));
  });
}

function bindAssetCards() {
  document.querySelectorAll(".dev-asset-card[data-asset-id]").forEach((card) => {
    const navigate = () => {
      window.location.href = `/developer/asset-detail?id=${encodeURIComponent(card.dataset.assetId)}`;
    };
    card.addEventListener("click", (event) => {
      if (event.target.closest("button, a")) return;
      navigate();
    });
    card.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      navigate();
    });
  });
}

function bindGalleryControls() {
  document.querySelectorAll(".dev-asset-card .property-nav-prev").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      if (typeof window.cardPrevImage === "function") window.cardPrevImage(button);
    });
  });
  document.querySelectorAll(".dev-asset-card .property-nav-next").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      if (typeof window.cardNextImage === "function") window.cardNextImage(button);
    });
  });
}

// Initialize developer assets page functionality
document.addEventListener("DOMContentLoaded", function () {
  applyCoverImages();
  bindStatusTabs();
  bindAssetCards();
  bindGalleryControls();

  // Initialize property card images with delay
  setTimeout(function () {
    if (typeof initializePropertyDots === "function") {
      initializePropertyDots();
    }
  }, 100);

  if (document.getElementById("dev-assets-status-tabs")) {
    showDevTab("available");
  }
});
