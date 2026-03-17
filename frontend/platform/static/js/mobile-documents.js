// Mobile Documents Tab Switching

document.addEventListener("DOMContentLoaded", function () {
  const tabs = document.querySelectorAll(".mobile-documents-tab");

  tabs.forEach((tab) => {
    tab.addEventListener("click", function (e) {
      e.preventDefault();

      // Remove active class from all tabs
      tabs.forEach((t) => t.classList.remove("active"));

      // Add active class to clicked tab
      this.classList.add("active");

      // The documents list stays the same regardless of which tab is selected
      // This matches the desktop behavior where clicking tabs doesn't change the documents
    });
  });
});
