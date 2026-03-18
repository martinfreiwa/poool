// Mobile Investment Calculator Implementation
document.addEventListener("DOMContentLoaded", function () {
  // Only initialize on mobile devices
  if (window.innerWidth > 768) return;

  // Calculator configuration
  const CHART_HEIGHT = 168; // Mobile chart height

  // Compute nice Y-axis bounds based on data (same as desktop)
  function computeNiceMax(maxValue) {
    const padded = maxValue * 1.15;
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

  // Calculator elements
  const calcMainValue = document.querySelector(
    ".mobile-calculator-top-text .amount",
  );
  const calcYAxis = document.querySelector(".mobile-chart-y-axis");
  const calcChartBars = document.querySelector(".mobile-chart-bars");

  // Legend value elements
  const investmentLegendValue = document.getElementById(
    "mobile-calc-investment",
  );
  const rentalLegendValue = document.getElementById("mobile-calc-rental");
  const appreciationLegendValue = document.getElementById(
    "mobile-calc-appreciation",
  );

  // Slider sections
  const sliderSections = {
    investment: document.querySelector(
      ".mobile-slider-section.investment-amount",
    ),
    growth: document.querySelector(".mobile-slider-section.property-growth"),
    yield: document.querySelector(".mobile-slider-section.rental-yield"),
  };

  // Current values - Updated defaults for mobile
  let currentValues = {
    investment: 50000,
    growth: 16,
    yield: 16,
  };

  // Real Estate Investment Calculation Function using Integer Cents (same logic as desktop)
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

  // Format currency for display
  function formatCurrency(amount) {
    if (amount >= 1000000) {
      return `${(amount / 1000000).toFixed(1)}M`;
    } else if (amount >= 1000) {
      return `${Math.round(amount / 1000)}k`;
    } else {
      return Math.round(amount).toString();
    }
  }

  // Format full currency with commas
  function formatFullCurrency(amount) {
    return Math.round(amount).toLocaleString();
  }

  // Update Y-axis with dynamic scaling
  function updateYAxis(maxValue) {
    if (!calcYAxis) return computeNiceMax(maxValue);

    const yAxisMax = computeNiceMax(maxValue);
    const steps = 5; // Mobile has 5 Y-axis labels
    const stepValue = yAxisMax / (steps - 1);

    const yAxisLines = calcYAxis.querySelectorAll(".mobile-y-axis-line");

    yAxisLines.forEach((line, index) => {
      const value = yAxisMax - stepValue * index;
      const label = line.querySelector(".mobile-y-axis-label");
      if (label) {
        label.textContent = formatCurrency(value);
      }
    });

    return yAxisMax;
  }

  // Update chart bars with calculated data
  function updateChartBars(calculationData, yAxisMax) {
    if (!calcChartBars) return;

    const bars = calcChartBars.querySelectorAll(".mobile-chart-bar");

    calculationData.forEach((yearData, index) => {
      if (index < bars.length) {
        const bar = bars[index];
        const barStack = bar.querySelector(".mobile-bar-stack");

        if (barStack) {
          // Calculate total bar height
          const totalHeight = (yearData.total / yAxisMax) * CHART_HEIGHT;
          barStack.style.height = `${totalHeight}px`;

          // Update segment heights as percentages
          const investmentPercent =
            (yearData.investment / yearData.total) * 100;
          const rentalPercent = (yearData.rental / yearData.total) * 100;
          const appreciationPercent =
            (yearData.appreciation / yearData.total) * 100;

          // Update segment styles
          const investmentSegment = barStack.querySelector(
            ".mobile-bar-segment.investment",
          );
          const rentalSegment = barStack.querySelector(
            ".mobile-bar-segment.rental",
          );
          const appreciationSegment = barStack.querySelector(
            ".mobile-bar-segment.appreciation",
          );

          if (investmentSegment) {
            investmentSegment.style.height = `${investmentPercent}%`;
            investmentSegment.style.bottom = "0%";
          }

          if (rentalSegment) {
            rentalSegment.style.height = `${rentalPercent}%`;
            rentalSegment.style.bottom = `${investmentPercent}%`;
          }

          if (appreciationSegment) {
            appreciationSegment.style.height = `${appreciationPercent}%`;
            appreciationSegment.style.bottom = `${investmentPercent + rentalPercent}%`;
          }
        }
      }
    });
  }

  // Update main title
  function updateMainTitle(calculationData) {
    if (!calcMainValue) return;

    const cumulativeReturns = calculationData.reduce((sum, year) => {
      return sum + year.appreciation + year.rental;
    }, 0);

    const totalReturn = cumulativeReturns + calculationData[0].investment;
    calcMainValue.textContent = `USD ${formatFullCurrency(totalReturn)} in 5 years`;
  }

  // Update legend values
  function updateLegendValues(calculationData) {
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

      if (investmentLegendValue) {
        investmentLegendValue.textContent = `$${formatFullCurrency(totalInvestment)}`;
      }
      if (rentalLegendValue) {
        rentalLegendValue.textContent = `$${formatFullCurrency(totalRental)}`;
      }
      if (appreciationLegendValue) {
        appreciationLegendValue.textContent = `$${formatFullCurrency(totalAppreciation)}`;
      }
    }
  }

  // Main calculation and update function
  function updateCalculator() {
    const calculationData = calculateInvestmentReturns(
      currentValues.investment,
      currentValues.growth,
      currentValues.yield,
    );

    const maxValue = Math.max(...calculationData.map((year) => year.total));

    const yAxisMax = updateYAxis(maxValue);
    updateChartBars(calculationData, yAxisMax);
    updateMainTitle(calculationData);
    updateLegendValues(calculationData);
  }

  // Initialize slider functionality
  function initializeSliders() {
    // Investment amount slider
    if (sliderSections.investment) {
      const thumb = sliderSections.investment.querySelector(
        ".mobile-slider-thumb",
      );
      const fill = sliderSections.investment.querySelector(
        ".mobile-slider-fill",
      );
      const valueDisplay = sliderSections.investment.querySelector(
        ".mobile-slider-value",
      );
      const labelValue = document.querySelector(
        ".mobile-slider-labels.investment .value",
      );

      if (thumb && fill) {
        makeSliderDraggable(thumb, fill, valueDisplay, labelValue, {
          min: 500,
          max: 590000,
          value: currentValues.investment,
          format: (val) => `USD ${formatFullCurrency(val)}`,
          onChange: (val) => {
            currentValues.investment = val;
            updateCalculator();
          },
        });
      }

      // Make value inputs editable
      setupManualInput(
        valueDisplay,
        "investment",
        500,
        590000,
        (val) => `USD ${formatFullCurrency(val)}`,
      );
      setupManualInput(
        labelValue,
        "investment",
        500,
        590000,
        (val) => `USD ${formatFullCurrency(val)}`,
      );
    }

    // Property growth slider
    if (sliderSections.growth) {
      const thumb = sliderSections.growth.querySelector(".mobile-slider-thumb");
      const fill = sliderSections.growth.querySelector(".mobile-slider-fill");
      const valueDisplay = sliderSections.growth.querySelector(
        ".mobile-slider-value",
      );
      const labelValue = document.querySelector(
        ".mobile-slider-labels.growth .value",
      );

      if (thumb && fill) {
        makeSliderDraggable(thumb, fill, valueDisplay, labelValue, {
          min: 1,
          max: 20,
          value: currentValues.growth,
          format: (val) => `${val}%`,
          onChange: (val) => {
            currentValues.growth = val;
            updateCalculator();
          },
        });
      }

      // Make value inputs editable
      setupManualInput(valueDisplay, "growth", 1, 20, (val) => `${val}%`);
      setupManualInput(labelValue, "growth", 1, 20, (val) => `${val}%`);
    }

    // Rental yield slider
    if (sliderSections.yield) {
      const thumb = sliderSections.yield.querySelector(".mobile-slider-thumb");
      const fill = sliderSections.yield.querySelector(".mobile-slider-fill");
      const valueDisplay = sliderSections.yield.querySelector(
        ".mobile-slider-value",
      );
      const labelValue = document.querySelector(
        ".mobile-slider-labels.yield .value",
      );

      if (thumb && fill) {
        makeSliderDraggable(thumb, fill, valueDisplay, labelValue, {
          min: 1,
          max: 20,
          value: currentValues.yield,
          format: (val) => `${val}%`,
          onChange: (val) => {
            currentValues.yield = val;
            updateCalculator();
          },
        });
      }

      // Make value inputs editable
      setupManualInput(valueDisplay, "yield", 1, 20, (val) => `${val}%`);
      setupManualInput(labelValue, "yield", 1, 20, (val) => `${val}%`);
    }
  }

  // Setup manual input functionality for value displays
  function setupManualInput(element, property, min, max, formatter) {
    if (!element) return;

    element.contentEditable = true;
    element.setAttribute("data-property", property);

    element.addEventListener("click", function (e) {
      e.stopPropagation();
      this.focus();
      // Select all text for easy editing
      const selection = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(this);
      selection.removeAllRanges();
      selection.addRange(range);
    });

    element.addEventListener("keydown", function (e) {
      if (e.key === "Enter") {
        e.preventDefault();
        this.blur();
      }
      // Allow backspace, delete, arrows, and numbers
      if (
        ![8, 9, 37, 38, 39, 40, 46].includes(e.keyCode) &&
        (e.keyCode < 48 || e.keyCode > 57) &&
        (e.keyCode < 96 || e.keyCode > 105)
      ) {
        if (e.key !== "." && e.key !== ",") {
          e.preventDefault();
        }
      }
    });

    element.addEventListener("blur", function () {
      let value = this.textContent.trim();
      // Remove currency symbols and percentage signs
      value = value.replace(/[^0-9.,]/g, "").replace(/,/g, "");

      let numValue = parseFloat(value);
      if (isNaN(numValue)) {
        numValue = currentValues[property]; // Reset to current value if invalid
      } else {
        numValue = Math.max(min, Math.min(max, numValue));
      }

      currentValues[property] = numValue;

      // Update both displays
      const formattedValue = formatter(numValue);
      this.textContent = formattedValue;

      // Update the corresponding slider position
      updateSliderPosition(property);

      // Recalculate
      updateCalculator();
    });

    element.addEventListener("input", function () {
      // Prevent line breaks
      if (this.innerHTML.includes("<br>") || this.innerHTML.includes("<div>")) {
        this.textContent = this.textContent;
      }
    });
  }

  // Update slider position when value is manually entered
  function updateSliderPosition(property) {
    const section = sliderSections[property];
    if (!section) return;

    const thumb = section.querySelector(".mobile-slider-thumb");
    const fill = section.querySelector(".mobile-slider-fill");
    if (!thumb || !fill) return;

    let min, max;
    switch (property) {
      case "investment":
        min = 500;
        max = 590000;
        break;
      case "growth":
      case "yield":
        min = 1;
        max = 20;
        break;
    }

    const value = currentValues[property];
    const percent = (value - min) / (max - min);
    const track = section.querySelector(".mobile-slider-track");
    const trackWidth = track ? track.offsetWidth : 0;
    if (trackWidth === 0) return; // Track not rendered yet

    const position = percent * trackWidth;

    thumb.style.left = `${Math.max(0, position - 8)}px`;
    fill.style.width = `${position}px`;
  }

  // Make slider draggable on mobile
  function makeSliderDraggable(thumb, fill, valueDisplay, labelValue, options) {
    const sliderTrack = thumb.parentElement;

    function getTrackWidth() {
      return sliderTrack ? sliderTrack.offsetWidth : 0;
    }

    // Set initial position
    const initialPercent =
      (options.value - options.min) / (options.max - options.min);
    const currentTrackWidth = getTrackWidth();
    const initialPosition = initialPercent * currentTrackWidth;
    thumb.style.left = `${Math.max(0, initialPosition - 8)}px`;
    fill.style.width = `${initialPosition}px`;

    // Update displays
    if (valueDisplay) {
      valueDisplay.textContent = options.format(options.value);
      valueDisplay.contentEditable = true;
    }
    if (labelValue) {
      labelValue.textContent = options.format(options.value);
      labelValue.contentEditable = true;
    }

    let isDragging = false;
    let startX = 0;
    let startLeft = 0;

    // Touch events for mobile
    thumb.addEventListener("touchstart", (e) => {
      isDragging = true;
      startX = e.touches[0].clientX;
      startLeft = parseFloat(thumb.style.left) || 0;
      e.preventDefault();
    });

    document.addEventListener("touchmove", (e) => {
      if (!isDragging) return;

      const deltaX = e.touches[0].clientX - startX;
      let newLeft = startLeft + deltaX;
      const currentTrackWidth = getTrackWidth();

      // Constrain to track bounds
      newLeft = Math.max(0, Math.min(currentTrackWidth - 16, newLeft));

      // Calculate value
      const percent = (newLeft + 8) / currentTrackWidth;
      const value = Math.round(
        options.min + percent * (options.max - options.min),
      );

      // Update visual elements
      thumb.style.left = `${newLeft}px`;
      fill.style.width = `${newLeft + 8}px`;

      // Update displays
      if (valueDisplay) valueDisplay.textContent = options.format(value);
      if (labelValue) labelValue.textContent = options.format(value);

      // Call onChange
      options.onChange(value);

      e.preventDefault();
    });

    document.addEventListener("touchend", () => {
      isDragging = false;
    });

    // Mouse events for desktop testing
    thumb.addEventListener("mousedown", (e) => {
      isDragging = true;
      startX = e.clientX;
      startLeft = parseFloat(thumb.style.left) || 0;
      e.preventDefault();
    });

    document.addEventListener("mousemove", (e) => {
      if (!isDragging) return;

      const deltaX = e.clientX - startX;
      let newLeft = startLeft + deltaX;
      const currentTrackWidth = getTrackWidth();

      // Constrain to track bounds
      newLeft = Math.max(0, Math.min(currentTrackWidth - 16, newLeft));

      // Calculate value
      const percent = (newLeft + 8) / currentTrackWidth;
      const value = Math.round(
        options.min + percent * (options.max - options.min),
      );

      // Update visual elements
      thumb.style.left = `${newLeft}px`;
      fill.style.width = `${newLeft + 8}px`;

      // Update displays
      if (valueDisplay) valueDisplay.textContent = options.format(value);
      if (labelValue) labelValue.textContent = options.format(value);

      // Call onChange
      options.onChange(value);

      e.preventDefault();
    });

    document.addEventListener("mouseup", () => {
      isDragging = false;
    });
  }

  // Initialize calculator
  function initMobileCalculator() {
    initializeSliders();
    updateCalculator(); // Initial calculation
  }

  // Start calculator when DOM is ready
  initMobileCalculator();
});
