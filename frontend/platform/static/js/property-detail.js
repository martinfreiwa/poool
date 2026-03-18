// Gallery Lightbox functionality
// Dynamically get images from the actual gallery DOM elements
function getGalleryImages() {
  const images = [];

  // Get all gallery image divs in order
  const galleryImages = [
    document.querySelector("#gallery-main-image img"),
    document.querySelector("#gallery-image-top-left img"),
    document.querySelector("#gallery-image-top-right img"),
    document.querySelector("#gallery-image-bottom-left img"),
    document.querySelector("#gallery-image-bottom-right img"),
  ];

  // Collect all available images with their src and alt text
  galleryImages.forEach((img) => {
    if (img && img.src) {
      images.push({
        src: img.src,
        caption: img.alt || "Property view",
      });
    }
  });

  // Fallback to default if no images found
  if (images.length === 0) {
    return [{ src: "/static/images/villa1.webp", caption: "Main property view" }];
  }

  return images;
}

let currentImageIndex = 0;
let galleryImages = [];

// Initialize FAQ expansion monitoring
document.addEventListener("DOMContentLoaded", function () {
  initializeFAQExpansionMonitoring();
});

// Monitor FAQ expansions to update Similar Properties position
function initializeFAQExpansionMonitoring() {
  const faqItems = document.querySelectorAll(".faq-item");

  faqItems.forEach((faqItem) => {
    const faqContent = faqItem.querySelector(".faq-item-content");
    if (faqContent) {
      faqContent.addEventListener("click", function () {
        // Wait for FAQ animation to complete
        setTimeout(() => {
          updateSimilarPropertiesPosition();
        }, 450); // Slightly longer than the 0.4s transition
      });
    }
  });
}

// Update Similar Properties section position based on main card height
function updateSimilarPropertiesPosition() {
  const mainCard = document.getElementById("property-main-card");
  const similarPropertiesWrapper = document.querySelector(
    ".similar-properties-wrapper",
  );

  if (!mainCard || !similarPropertiesWrapper) return;

  // Calculate new position based on main card height
  const mainCardTop = 620; // Original main card top position
  const gap = 80; // Gap between main card and similar properties
  const mainCardHeight = mainCard.offsetHeight;

  const newPosition = mainCardTop + mainCardHeight + gap;

  // Update similar properties position
  similarPropertiesWrapper.style.top = newPosition + "px";
}

function openLightbox(index) {
  // Update gallery images array when opening
  galleryImages = getGalleryImages();

  const modal = document.getElementById("lightbox-modal");
  const img = document.getElementById("lightbox-img");
  const caption = document.getElementById("lightbox-caption");

  if (modal && img && caption) {
    currentImageIndex = index;

    // First set display to flex
    modal.style.display = "flex";

    // Set image and caption
    img.src = galleryImages[index].src;
    caption.textContent = galleryImages[index].caption;

    // Force a reflow to ensure display change is applied
    modal.offsetHeight;

    // Then add opening animation class
    modal.classList.add("lightbox-opening");

    // Prevent body scroll
    document.body.style.overflow = "hidden";
    document.body.classList.add("lightbox-open");

    // Remove opening class after animation
    setTimeout(() => {
      modal.classList.remove("lightbox-opening");
    }, 300);

    // Close lightbox when clicking/tapping outside the image
    modal.onclick = function (event) {
      if (event.target === modal) {
        closeLightbox();
      }
    };

    // Add touch event support for mobile
    modal.addEventListener(
      "touchend",
      function (event) {
        if (event.target === modal) {
          closeLightbox();
        }
      },
      { passive: true },
    );
  }
}

function closeLightbox() {
  const modal = document.getElementById("lightbox-modal");
  const img = document.getElementById("lightbox-img");

  if (modal) {
    // Reset image inline styles before closing to allow CSS animations to work
    if (img) {
      img.style.transition = "";
      img.style.transform = "";
      img.style.opacity = "";
    }

    // Add closing animation class
    modal.classList.add("lightbox-closing");

    // Wait for animation to finish before hiding
    setTimeout(() => {
      modal.style.display = "none";
      modal.classList.remove("lightbox-closing");
      document.body.style.overflow = "auto";
      document.body.classList.remove("lightbox-open");
    }, 300);
  }
}

function changeImage(direction) {
  currentImageIndex += direction;

  // Wrap around
  if (currentImageIndex < 0) {
    currentImageIndex = galleryImages.length - 1;
  } else if (currentImageIndex >= galleryImages.length) {
    currentImageIndex = 0;
  }

  const img = document.getElementById("lightbox-img");
  const caption = document.getElementById("lightbox-caption");

  if (img && caption) {
    // Remove any existing animation classes
    img.className = "lightbox-content";

    // Determine swipe direction
    // direction > 0 means NEXT (swipe left): current exits left, new enters from right
    // direction < 0 means PREV (swipe right): current exits right, new enters from left
    const exitDirection = direction > 0 ? "-100vw" : "100vw";
    const enterDirection = direction > 0 ? "100vw" : "-100vw";

    // Animate current image out
    img.style.transform = `translateX(${exitDirection})`;
    img.style.opacity = "0";

    // After exit animation completes, change image and animate in
    setTimeout(() => {
      // Change image source
      img.src = galleryImages[currentImageIndex].src;
      caption.textContent = galleryImages[currentImageIndex].caption;

      // Position new image off-screen on the opposite side
      img.style.transition = "none";
      img.style.transform = `translateX(${enterDirection})`;
      img.style.opacity = "0";

      // Force reflow
      img.offsetHeight;

      // Re-enable transitions and animate in
      img.style.transition =
        "transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s ease";
      img.style.transform = "translateX(0)";
      img.style.opacity = "1";
    }, 300);
  }
}

// Close lightbox on ESC key
document.addEventListener("keydown", function (event) {
  if (event.key === "Escape") {
    closeLightbox();
  } else if (event.key === "ArrowLeft") {
    const modal = document.getElementById("lightbox-modal");
    if (modal && modal.style.display === "flex") {
      changeImage(-1);
    }
  } else if (event.key === "ArrowRight") {
    const modal = document.getElementById("lightbox-modal");
    if (modal && modal.style.display === "flex") {
      changeImage(1);
    }
  }
});

// Preload next images for smoother navigation
function preloadImage(src) {
  const img = new Image();
  img.src = src;
}

// Stage carousel functionality
document.addEventListener("DOMContentLoaded", function () {
  // Preload gallery images after page load
  setTimeout(function () {
    galleryImages.forEach(function (image, index) {
      if (index > 4) {
        // Only preload images not visible in initial gallery
        preloadImage(image.src);
      }
    });
  }, 1000);

  // Initialize property card images in Similar Properties section with delay
  setTimeout(function () {
    if (typeof initializePropertyDots === "function") {
      initializePropertyDots();
    } else {
    }
  }, 100);
  // Calculator chart animations
  const chartBars = document.querySelectorAll(".calc-bar");
  chartBars.forEach((bar) => {
    bar.addEventListener("mouseenter", function () {
      // Add tooltip or value display on hover if needed
      const value = this.getAttribute("data-value");
      if (value) {
        // Could add a tooltip here
      }
    });
  });

  // Calculator sliders functionality with native range inputs
  function initializeSliders() {
    // Helper: update the slider track fill color via CSS variable
    function updateSliderTrack(slider) {
      const min = parseFloat(slider.min);
      const max = parseFloat(slider.max);
      const val = parseFloat(slider.value);
      const percent = ((val - min) / (max - min)) * 100;
      slider.style.setProperty("--slider-progress", percent + "%");
      // Also set the background directly for WebKit
      slider.style.background = `linear-gradient(to right, #0000FF ${percent}%, #e2e2e2 ${percent}%)`;
    }

    // Function to format value for display
    function formatValue(val, isUSD = false) {
      if (isUSD) {
        return "USD " + new Intl.NumberFormat("en-US").format(Math.round(val));
      } else {
        return Number.isInteger(val) ? val + "%" : val.toFixed(1) + "%";
      }
    }

    // Slider 1: Investment Amount
    const slider1 = document.getElementById("calc-slider-1");
    const value1 = document.getElementById("calc-slider-value-1");

    if (slider1) {
      updateSliderTrack(slider1);
      slider1.addEventListener("input", function () {
        const val = parseFloat(this.value);
        if (value1) value1.textContent = formatValue(val, true);
        updateSliderTrack(this);
      });
    }

    // Slider 2: Property Value Growth
    const slider2 = document.getElementById("calc-slider-2");
    const value2 = document.getElementById("calc-slider-value-2");

    if (slider2) {
      updateSliderTrack(slider2);
      slider2.addEventListener("input", function () {
        const val = parseFloat(this.value);
        if (value2) value2.textContent = formatValue(val);
        updateSliderTrack(this);
      });
    }

    // Slider 3: Rental Yield
    const slider3 = document.getElementById("calc-slider-3");
    const value3 = document.getElementById("calc-slider-value-3");

    if (slider3) {
      updateSliderTrack(slider3);
      slider3.addEventListener("input", function () {
        const val = parseFloat(this.value);
        if (value3) value3.textContent = formatValue(val);
        updateSliderTrack(this);
      });
    }
  }

  initializeSliders();

  // Financial tabs functionality
  function initializeFinancialTabs() {
    const tabs = document.querySelectorAll(".financial-tab");
    const propertyCostContent = document.getElementById(
      "property-cost-content",
    );
    const rentalIncomeContent = document.getElementById(
      "rental-income-content",
    );

    if (tabs.length > 0) {
      tabs.forEach((tab) => {
        tab.addEventListener("click", function () {
          // Remove active class from all tabs
          tabs.forEach((t) => t.classList.remove("active"));

          // Add active class to clicked tab
          this.classList.add("active");

          // Show/hide content based on clicked tab
          const tabType = this.getAttribute("data-tab");
          if (tabType === "property-cost") {
            if (propertyCostContent) propertyCostContent.style.display = "flex";
            if (rentalIncomeContent) rentalIncomeContent.style.display = "none";
          } else if (tabType === "rental-income") {
            if (propertyCostContent) propertyCostContent.style.display = "none";
            if (rentalIncomeContent) rentalIncomeContent.style.display = "flex";
          }
        });
      });
    }
  }

  initializeFinancialTabs();

  // Documents tabs functionality
  function initializeDocumentsTabs() {
    const documentsTabs = document.querySelectorAll(".documents-tab");

    if (documentsTabs.length > 0) {
      documentsTabs.forEach((tab) => {
        tab.addEventListener("click", function () {
          // Remove active class from all tabs
          documentsTabs.forEach((t) => t.classList.remove("active"));

          // Add active class to clicked tab
          this.classList.add("active");

          // Here you can add logic to show/hide different document content
          // based on the selected tab if needed in the future
          const tabId = this.id;

          // Optional: You can add content switching logic here
          // For now, we'll just handle the visual tab switching
        });
      });
    }
  }

  initializeDocumentsTabs();

  // FAQ section - no functionality, just static display

  const stageCards = document.querySelector(".stage-cards");
  const prevButton = document.querySelector(".nav-button.prev");
  const nextButton = document.querySelector(".nav-button.next");
  const cards = document.querySelectorAll(".stage-card");

  let currentIndex = 0;
  const cardWidth = 290; // Card width
  const cardGap = 12; // Gap between cards
  const cardOffset = cardWidth + cardGap;
  const containerWidth = 584; // Container width
  const cardsVisibleAtOnce = 1; // Show only one card at a time within the container

  function updateCardPosition() {
    if (!stageCards) return;

    // Account for initial centering offset of 147px
    const translateX = -currentIndex * cardOffset;
    stageCards.style.transform = `translateX(${translateX}px)`;

    // Update button states
    if (prevButton) {
      prevButton.disabled = currentIndex === 0;
    }
    if (nextButton) {
      nextButton.disabled = currentIndex >= cards.length - cardsVisibleAtOnce;
    }
  }

  function goToPrevCard() {
    if (currentIndex > 0) {
      currentIndex--;
      updateCardPosition();
    }
  }

  function goToNextCard() {
    if (currentIndex < cards.length - cardsVisibleAtOnce) {
      currentIndex++;
      updateCardPosition();
    }
  }

  // Initialize
  updateCardPosition();

  // Event listeners
  if (prevButton) {
    prevButton.addEventListener("click", goToPrevCard);
  }

  if (nextButton) {
    nextButton.addEventListener("click", goToNextCard);
  }
});

// ===========================
// Documents Modal Functionality
// ===========================
document.addEventListener("DOMContentLoaded", function () {
  const documentsModalOverlay = document.getElementById(
    "documents-modal-overlay",
  );
  const closeModalButton = document.getElementById("close-documents-modal");
  const firstCardAction = document.querySelector(
    ".info-card:first-child .card-action",
  );

  // Function to show documents modal
  function showDocumentsModal() {
    if (documentsModalOverlay) {
      // First make it visible
      documentsModalOverlay.style.display = "flex";
      document.body.classList.add("documents-modal-active");

      // Force a reflow to ensure display change is applied
      documentsModalOverlay.offsetHeight;

      // Then add the active class for animated transition
      requestAnimationFrame(function () {
        documentsModalOverlay.classList.add("active");
      });
    }
  }

  // Function to hide documents modal
  function hideDocumentsModal() {
    if (documentsModalOverlay) {
      // Remove active class to start fade out animation
      documentsModalOverlay.classList.remove("active");

      // Wait for transition to complete before hiding
      setTimeout(function () {
        documentsModalOverlay.style.display = "none";
        document.body.classList.remove("documents-modal-active");
      }, 300); // Match the CSS transition duration
    }
  }

  // Event listener for first card action click
  if (firstCardAction) {
    firstCardAction.addEventListener("click", function (e) {
      e.preventDefault();
      showDocumentsModal();
    });
  }

  // Event listener for close button
  if (closeModalButton) {
    closeModalButton.addEventListener("click", hideDocumentsModal);
  }

  // Event listener for overlay click (close modal when clicking outside)
  if (documentsModalOverlay) {
    documentsModalOverlay.addEventListener("click", function (e) {
      if (e.target === documentsModalOverlay) {
        hideDocumentsModal();
      }
    });
  }

  // Event listener for escape key
  document.addEventListener("keydown", function (e) {
    if (
      e.key === "Escape" &&
      documentsModalOverlay.classList.contains("active")
    ) {
      hideDocumentsModal();
    }
  });

  // Placeholder click handlers for document links (no action for now)
  const documentLinks = document.querySelectorAll(".document-link");
  documentLinks.forEach(function (link) {
    link.addEventListener("click", function (e) {
      e.preventDefault();
      // Document links do nothing for now as requested
    });
  });
});

// ===========================
// YouTube Video Modal Functionality
// ===========================
document.addEventListener("DOMContentLoaded", function () {
  const youtubeModalOverlay = document.getElementById("youtube-modal-overlay");
  const closeYoutubeButton = document.getElementById("close-youtube-modal");
  const videoPlayButton = document.getElementById("video-play-button");
  const youtubeIframe = document.getElementById("youtube-iframe");

  // YouTube video ID extracted from the URL
  const youtubeVideoId = "GTSeeou3Wg8";
  const youtubeEmbedUrl = `https://www.youtube.com/embed/${youtubeVideoId}?autoplay=1&rel=0`;

  // Function to show YouTube modal
  function showYoutubeModal() {
    if (youtubeModalOverlay && youtubeIframe) {
      // Set the iframe source
      youtubeIframe.src = youtubeEmbedUrl;

      // First make it visible
      youtubeModalOverlay.style.display = "flex";
      document.body.classList.add("youtube-modal-active");

      // Force a reflow to ensure display change is applied
      youtubeModalOverlay.offsetHeight;

      // Then add the active class for animated transition
      requestAnimationFrame(function () {
        youtubeModalOverlay.classList.add("active");
      });
    }
  }

  // Function to hide YouTube modal
  function hideYoutubeModal() {
    if (youtubeModalOverlay && youtubeIframe) {
      // Remove active class to start fade out animation
      youtubeModalOverlay.classList.remove("active");

      // Wait for transition to complete before hiding and stopping video
      setTimeout(function () {
        youtubeModalOverlay.style.display = "none";
        document.body.classList.remove("youtube-modal-active");
        // Stop the video by clearing the iframe src
        youtubeIframe.src = "";
      }, 300); // Match the CSS transition duration
    }
  }

  // Event listener for video play button click
  if (videoPlayButton) {
    videoPlayButton.addEventListener("click", function (e) {
      e.preventDefault();
      showYoutubeModal();
    });
  }

  // Event listener for close button
  if (closeYoutubeButton) {
    closeYoutubeButton.addEventListener("click", hideYoutubeModal);
  }

  // Event listener for overlay click (close modal when clicking outside)
  if (youtubeModalOverlay) {
    youtubeModalOverlay.addEventListener("click", function (e) {
      if (e.target === youtubeModalOverlay) {
        hideYoutubeModal();
      }
    });
  }

  // Event listener for escape key
  document.addEventListener("keydown", function (e) {
    if (
      e.key === "Escape" &&
      youtubeModalOverlay.classList.contains("active")
    ) {
      hideYoutubeModal();
    }
  });
});

// ===========================
// FAQ Accordion Functionality
// ===========================
document.addEventListener("DOMContentLoaded", function () {
  const faqItems = document.querySelectorAll(".faq-item");

  faqItems.forEach(function (item) {
    const itemContent = item.querySelector(".faq-item-content");

    if (itemContent) {
      itemContent.addEventListener("click", function () {
        const isActive = item.classList.contains("active");

        // Close all other FAQ items
        faqItems.forEach(function (otherItem) {
          if (otherItem !== item) {
            otherItem.classList.remove("active");
          }
        });

        // Toggle current item
        if (isActive) {
          item.classList.remove("active");
        } else {
          item.classList.add("active");
        }
      });
    }
  });
});

// Management fees tooltip functionality
document.addEventListener("DOMContentLoaded", function () {
  const managementFeesTooltipContainer = document.querySelector(
    ".management-fees-tooltip-container",
  );

  if (managementFeesTooltipContainer) {
    managementFeesTooltipContainer.addEventListener("click", function (e) {
      e.preventDefault();
      e.stopPropagation();

      // Toggle the active class
      this.classList.toggle("active");
    });

    // Close tooltip when clicking outside
    document.addEventListener("click", function (e) {
      if (!managementFeesTooltipContainer.contains(e.target)) {
        managementFeesTooltipContainer.classList.remove("active");
      }
    });
  }
});

// Investment Calculator Implementation
document.addEventListener("DOMContentLoaded", function () {
  // Calculator configuration
  const CHART_HEIGHT = 180; // Chart container height in pixels

  // Calculator elements
  const calcMainValue = document.getElementById("calc-main-value");
  const calcYAxis = document.getElementById("calc-y-axis");
  const calcChartBars = document.getElementById("calc-chart-bars");

  // Input elements (native range sliders)
  const investmentSlider = document.getElementById("calc-slider-1");
  const growthSlider = document.getElementById("calc-slider-2");
  const yieldSlider = document.getElementById("calc-slider-3");

  const investmentValue = document.getElementById("calc-slider-value-1");
  const growthValue = document.getElementById("calc-slider-value-2");
  const yieldValue = document.getElementById("calc-slider-value-3");

  // Store last calculated data for tooltips
  let lastCalculationData = [];

  // Real Estate Investment Calculation Function using Integer Cents
  function calculateInvestmentReturns(
    investment,
    annualGrowthRate,
    annualYieldRate,
  ) {
    const returns = [];
    // Convert to cents to prevent IEEE754 float precision errors
    const investmentCents = Math.round(investment * 100);
    let currentPropertyValueCents = investmentCents;

    for (let year = 1; year <= 5; year++) {
      // Property appreciation for this year (compound growth in cents)
      const appreciationCents = Math.round(currentPropertyValueCents * (annualGrowthRate / 100));
      currentPropertyValueCents += appreciationCents;

      // Rental income (based on original investment amount, in cents)
      const rentalIncomeCents = Math.round(investmentCents * (annualYieldRate / 100));

      // Total annual return components (converted back to dollars for UI display)
      const yearData = {
        year: year,
        investment: investmentCents / 100,
        appreciation: appreciationCents / 100,
        rental: rentalIncomeCents / 100,
        total: (investmentCents + appreciationCents + rentalIncomeCents) / 100,
      };

      returns.push(yearData);
    }

    return returns;
  }

  // Get current input values from native sliders
  function getCurrentValues() {
    return {
      investment: investmentSlider ? parseFloat(investmentSlider.value) || 100000 : 100000,
      growth: growthSlider ? parseFloat(growthSlider.value) || 10 : 10,
      yield: yieldSlider ? parseFloat(yieldSlider.value) || 12 : 12,
    };
  }

  // Format currency for display (compact)
  function formatCurrency(amount) {
    if (amount >= 1000000) {
      return `${(amount / 1000000).toFixed(1)}M`;
    } else if (amount >= 1000) {
      return `${Math.round(amount / 1000)}k`;
    } else {
      return Math.round(amount).toString();
    }
  }

  // Format currency for main title (always show full number with commas)
  function formatFullCurrency(amount) {
    return Math.round(amount).toLocaleString();
  }

  // Format currency for tooltips (with dollar sign)
  function formatTooltipCurrency(amount) {
    return "$" + Math.round(amount).toLocaleString();
  }

  // Compute nice Y-axis bounds based on data
  function computeNiceMax(maxValue) {
    // Add 15% padding
    const padded = maxValue * 1.15;

    // Round up to a "nice" number
    if (padded <= 0) return 1000;

    const magnitude = Math.pow(10, Math.floor(Math.log10(padded)));
    const normalized = padded / magnitude;

    let niceNormalized;
    if (normalized <= 1.5) niceNormalized = 1.5;
    else if (normalized <= 2) niceNormalized = 2;
    else if (normalized <= 2.5) niceNormalized = 2.5;
    else if (normalized <= 3) niceNormalized = 3;
    else if (normalized <= 5) niceNormalized = 5;
    else if (normalized <= 7.5) niceNormalized = 7.5;
    else niceNormalized = 10;

    return niceNormalized * magnitude;
  }

  // Update Y-axis with auto-scaling based on actual data
  function updateYAxis(maxValue) {
    const yAxisMax = computeNiceMax(maxValue);
    const steps = 6; // Number of Y-axis labels
    const stepValue = yAxisMax / (steps - 1);

    if (!calcYAxis) return yAxisMax;

    const yAxisLines = calcYAxis.querySelectorAll(".calc-y-axis-line");

    yAxisLines.forEach((line, index) => {
      const value = yAxisMax - stepValue * index;
      const numberSpan = line.querySelector(".calc-y-axis-number");
      if (numberSpan) {
        numberSpan.textContent = formatCurrency(Math.max(0, value));
      }
    });

    return yAxisMax;
  }

  // Create tooltip and value label HTML for a bar
  function createBarOverlays(yearData) {
    const currentYear = new Date().getFullYear();
    const barYear = currentYear + yearData.year - 1;

    // Tooltip
    const tooltip = document.createElement("div");
    tooltip.className = "calc-bar-tooltip";
    tooltip.innerHTML = `
      <div style="font-weight:600;margin-bottom:6px;font-size:13px;">${barYear}</div>
      <div class="calc-bar-tooltip-row">
        <span class="calc-bar-tooltip-dot investment"></span>
        <span class="calc-bar-tooltip-label">Investment</span>
        <span class="calc-bar-tooltip-value">${formatTooltipCurrency(yearData.investment)}</span>
      </div>
      <div class="calc-bar-tooltip-row">
        <span class="calc-bar-tooltip-dot appreciation"></span>
        <span class="calc-bar-tooltip-label">Appreciation</span>
        <span class="calc-bar-tooltip-value">${formatTooltipCurrency(yearData.appreciation)}</span>
      </div>
      <div class="calc-bar-tooltip-row">
        <span class="calc-bar-tooltip-dot rental"></span>
        <span class="calc-bar-tooltip-label">Rental</span>
        <span class="calc-bar-tooltip-value">${formatTooltipCurrency(yearData.rental)}</span>
      </div>
      <div style="border-top:1px solid #333;margin-top:6px;padding-top:6px;display:flex;justify-content:space-between;gap:16px;">
        <span style="color:#A4A7AE;">Total</span>
        <span style="font-weight:700;">${formatTooltipCurrency(yearData.total)}</span>
      </div>
    `;

    // Value label above bar
    const valueLabel = document.createElement("div");
    valueLabel.className = "calc-bar-value-label";
    valueLabel.textContent = formatCurrency(yearData.total);

    return { tooltip, valueLabel };
  }

  // Update chart bars with calculated data
  function updateChartBars(calculationData, yAxisMax) {
    if (!calcChartBars) return;
    const bars = calcChartBars.querySelectorAll(".calc-bar");

    calculationData.forEach((yearData, index) => {
      if (index < bars.length) {
        const bar = bars[index];
        const chartBar = bar.querySelector(".calc-chart-bar");

        // Calculate bar height (bars start from bottom)
        const totalHeight = (yearData.total / yAxisMax) * CHART_HEIGHT;

        // Update bar height and position
        chartBar.style.height = `${totalHeight}px`;
        chartBar.style.top = "auto";
        chartBar.style.bottom = "0px";

        // Calculate proportions for stacked segments
        const investmentHeight =
          (yearData.investment / yearData.total) * totalHeight;
        const appreciationHeight =
          (yearData.appreciation / yearData.total) * totalHeight;
        const rentalHeight = (yearData.rental / yearData.total) * totalHeight;

        // Update series segments (stacked from bottom to top)
        const series1 = chartBar.querySelector(".calc-series.series-1");
        const series2 = chartBar.querySelector(".calc-series.series-2");
        const series3 = chartBar.querySelector(".calc-series.series-3");

        if (series1) {
          series1.style.height = `${investmentHeight}px`;
          series1.style.top = "auto";
          series1.style.bottom = "0px";
        }

        if (series2) {
          series2.style.height = `${appreciationHeight}px`;
          series2.style.top = "auto";
          series2.style.bottom = `${investmentHeight}px`;
        }

        if (series3) {
          series3.style.height = `${rentalHeight}px`;
          series3.style.top = "auto";
          series3.style.bottom = `${investmentHeight + appreciationHeight}px`;
        }

        // Remove old tooltips and value labels
        const oldTooltip = bar.querySelector(".calc-bar-tooltip");
        const oldLabel = bar.querySelector(".calc-bar-value-label");
        if (oldTooltip) oldTooltip.remove();
        if (oldLabel) oldLabel.remove();

        // Add new tooltip and value label
        const { tooltip, valueLabel } = createBarOverlays(yearData);
        bar.appendChild(tooltip);
        bar.appendChild(valueLabel);
      }
    });
  }

  // Update main title with total return (excluding original investment)
  function updateMainTitle(calculationData) {
    if (!calcMainValue) return;
    // Calculate cumulative returns over 5 years (appreciation + rental only)
    const cumulativeReturns = calculationData.reduce((sum, year) => {
      return sum + year.appreciation + year.rental;
    }, 0);

    calcMainValue.textContent = `USD ${formatFullCurrency(cumulativeReturns)} in 5 years`;
  }

  // Update statistics card with calculated totals
  function updateStatisticsCard(calculationData) {
    const investmentStat = document.getElementById("calc-stat-investment");
    const rentalStat = document.getElementById("calc-stat-rental");
    const appreciationStat = document.getElementById("calc-stat-appreciation");

    if (calculationData.length > 0) {
      const totalInvestment = calculationData[0].investment;
      const totalRental = calculationData.reduce(
        (sum, year) => sum + year.rental,
        0,
      );
      const totalAppreciation = calculationData.reduce(
        (sum, year) => sum + year.appreciation,
        0,
      );

      if (investmentStat) {
        investmentStat.textContent = `$${formatFullCurrency(totalInvestment)}`;
      }
      if (rentalStat) {
        rentalStat.textContent = `$${formatFullCurrency(totalRental)}`;
      }
      if (appreciationStat) {
        appreciationStat.textContent = `$${formatFullCurrency(totalAppreciation)}`;
      }
    }
  }

  // Main calculation and update function
  function updateCalculator() {
    const values = getCurrentValues();
    const calculationData = calculateInvestmentReturns(
      values.investment,
      values.growth,
      values.yield,
    );

    // Store for tooltip access
    lastCalculationData = calculationData;

    // Find maximum value for Y-axis scaling (auto-scale!)
    const maxValue = Math.max(...calculationData.map((year) => year.total));

    // Update all chart components
    const yAxisMax = updateYAxis(maxValue);
    updateChartBars(calculationData, yAxisMax);
    updateMainTitle(calculationData);
    updateStatisticsCard(calculationData);
  }

  // Event listeners for real-time updates
  function attachCalculatorListeners() {
    [investmentSlider, growthSlider, yieldSlider].forEach((slider) => {
      if (slider) {
        slider.addEventListener("input", updateCalculator);
      }
    });
  }

  // Initialize calculator
  function initCalculator() {
    updateCalculator();
    attachCalculatorListeners();
  }

  // Start calculator when DOM is ready
  initCalculator();
});
