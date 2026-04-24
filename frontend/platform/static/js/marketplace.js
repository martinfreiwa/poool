// Marketplace page behavior

/**
 * Switch between marketplace status tabs (Available, Funded, Exited)
 */
function initializeStatusTabs() {
  const tabs = document.querySelectorAll(".status-tabs .status-tab");
  tabs.forEach((tab) => {
    tab.addEventListener("click", function () {
      // Remove active from all siblings
      tabs.forEach((t) => t.classList.remove("active"));
      // Add active to current
      this.classList.add("active");
    });
  });
}

document.addEventListener("DOMContentLoaded", function () {
  initializeStatusTabs();
});

document.addEventListener("htmx:afterSwap", function (evt) {
  setTimeout(function () {
    initializeStatusTabs();
  }, 10);
});
