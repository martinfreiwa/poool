// Mobile Financial Section JavaScript
document.addEventListener("DOMContentLoaded", function () {
  // Only initialize on mobile devices
  if (window.innerWidth > 768) return;

  // Initialize mobile financial section functionality
  initializeMobileFinancialSection();
});

function initializeMobileFinancialSection() {
  // Set up initial state - Property cost tab should be active by default
  const propertyCostTab = document.getElementById("mobile-tab-property-cost");
  const rentalIncomeTab = document.getElementById("mobile-tab-rental-income");
  const propertyCostContent = document.getElementById(
    "mobile-property-cost-content",
  );
  const rentalIncomeContent = document.getElementById(
    "mobile-rental-income-content",
  );

  // Ensure initial state is correct
  if (
    propertyCostTab &&
    rentalIncomeTab &&
    propertyCostContent &&
    rentalIncomeContent
  ) {
    propertyCostTab.classList.add("active");
    rentalIncomeTab.classList.remove("active");
    propertyCostContent.style.display = "flex";
    rentalIncomeContent.style.display = "none";
  } else {
  }
}

// Tab switching function (called from onclick in template)
function switchMobileFinancialTab(tabType) {
  const propertyCostTab = document.getElementById("mobile-tab-property-cost");
  const rentalIncomeTab = document.getElementById("mobile-tab-rental-income");
  const propertyCostContent = document.getElementById(
    "mobile-property-cost-content",
  );
  const rentalIncomeContent = document.getElementById(
    "mobile-rental-income-content",
  );

  if (
    !propertyCostTab ||
    !rentalIncomeTab ||
    !propertyCostContent ||
    !rentalIncomeContent
  ) {
    return;
  }

  // Remove active class from all tabs
  propertyCostTab.classList.remove("active");
  rentalIncomeTab.classList.remove("active");

  // Hide all content
  propertyCostContent.style.display = "none";
  rentalIncomeContent.style.display = "none";

  // Activate selected tab and show content
  if (tabType === "property-cost") {
    propertyCostTab.classList.add("active");
    propertyCostContent.style.display = "flex";
    propertyCostContent.style.flexDirection = "column";
    propertyCostContent.style.gap = "16px";
  } else if (tabType === "rental-income") {
    rentalIncomeTab.classList.add("active");
    rentalIncomeContent.style.display = "flex";
    rentalIncomeContent.style.flexDirection = "column";
    rentalIncomeContent.style.gap = "16px";
  }
}

// Make the function globally available
window.switchMobileFinancialTab = switchMobileFinancialTab;
