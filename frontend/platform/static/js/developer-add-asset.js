// Asset Type Selection Handler
function selectAssetType(id) {
  // Remove selection from all cards
  const allCards = document.querySelectorAll(".asset-type-card");
  allCards.forEach((card) => {
    card.classList.remove("selected");
    card.classList.remove("js-selected");
  });

  // Add selection to clicked card
  const selectedCard = document.getElementById(`asset-type-card-${id}`);
  if (selectedCard) {
    selectedCard.classList.add("selected");
    selectedCard.classList.add("js-selected");
  }

  // Store selected asset type for next step
  window.selectedAssetType = id;
}

// Initialize on page load
document.addEventListener("DOMContentLoaded", function () {
  // Set background images for all cards
  const imageElements = document.querySelectorAll(
    ".asset-type-image[data-image-url]",
  );
  imageElements.forEach((element) => {
    const imageUrl = element.getAttribute("data-image-url");
    if (imageUrl) {
      element.style.backgroundImage = `linear-gradient(0deg, rgba(0, 0, 0, 0.2), rgba(0, 0, 0, 0.2)), url("${imageUrl}")`;
      element.style.backgroundSize = "cover";
      element.style.backgroundPosition = "center";
    }
  });

  // Set the default selection to real-estate (first option)
  const defaultCard = document.querySelector(".asset-type-card.selected");
  if (defaultCard) {
    // Extract ID from the card's id attribute
    const cardId = defaultCard.id.replace("asset-type-card-", "");
    window.selectedAssetType = cardId;
    defaultCard.classList.add("js-selected");
  }

  // Handle Next Step button click
  const nextBtn = document.getElementById("add-asset-next-btn");
  if (nextBtn) {
    nextBtn.addEventListener("click", function (e) {
      e.preventDefault();

      // Always proceed - either with selected or default (real-estate)
      if (!window.selectedAssetType) {
        window.selectedAssetType = "real-estate";
      }

      // Persist to localStorage so the application form can read it
      localStorage.setItem("selectedAssetType", window.selectedAssetType);

      // Navigate to application form with selected asset type
      window.location.href = "/developer/application-form";
    });
  }
});
