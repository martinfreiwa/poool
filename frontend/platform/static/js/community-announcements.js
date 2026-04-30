(function () {
  function switchToFeed() {
    const feedTab = document.querySelector(".community-tab-btn[data-tab='community-feed-tab']");
    if (!feedTab) return;

    if (typeof window.switchCommunityTab === "function") {
      window.switchCommunityTab(feedTab);
      return;
    }

    feedTab.click();
  }

  document.addEventListener("click", function (event) {
    const trigger = event.target.closest("[data-community-ann-read-more]");
    if (!trigger) return;

    event.preventDefault();
    switchToFeed();
  });
})();
