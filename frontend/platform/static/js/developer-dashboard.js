document.addEventListener("DOMContentLoaded", function () {
  const tableSortState = {
    key: "sales",
    direction: "desc",
  };

  function getSortValue(row, key) {
    if (key === "asset") {
      return (row.dataset.asset || "").toLowerCase();
    }
    return Number(row.dataset[key] || 0);
  }

  function sortDeveloperAssetsTable(key) {
    const wrapper = document.getElementById("developer-assets-wrapper");
    const body = document.getElementById("developer-assets-body");
    if (!wrapper || !body) return;

    if (tableSortState.key === key) {
      tableSortState.direction = tableSortState.direction === "asc" ? "desc" : "asc";
    } else {
      tableSortState.key = key;
      tableSortState.direction = key === "asset" ? "asc" : "desc";
    }

    const rows = Array.from(body.querySelectorAll(".developer-assets-table__row[data-asset]"));
    rows.sort((a, b) => {
      const aValue = getSortValue(a, tableSortState.key);
      const bValue = getSortValue(b, tableSortState.key);

      if (typeof aValue === "string") {
        return tableSortState.direction === "asc"
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      }

      return tableSortState.direction === "asc" ? aValue - bValue : bValue - aValue;
    });

    rows.forEach((row) => body.appendChild(row));

    wrapper.querySelectorAll(".developer-assets-table__sort").forEach((button) => {
      const isActive = button.dataset.devSort === tableSortState.key;
      button.classList.toggle("developer-assets-table__sort--active", isActive);
      button.dataset.direction = isActive ? tableSortState.direction : "";
      button.setAttribute("aria-sort", isActive ? tableSortState.direction : "none");
    });
  }

  document.addEventListener("click", (event) => {
    if (!(event.target instanceof Element)) return;
    const sortButton = event.target.closest(".developer-assets-table__sort");
    if (!sortButton) return;

    const key = sortButton.dataset.devSort;
    if (!key) return;

    sortDeveloperAssetsTable(key);
  });

  document.addEventListener("htmx:afterSwap", (event) => {
    if (event.target && event.target.id === "developer-assets-wrapper") {
      tableSortState.key = "sales";
      tableSortState.direction = "desc";
    }
  });

  // Animate metric numbers with easing
  function animateCounter(element, target, duration = 2000) {
    const startTime = performance.now();

    // Quadratic ease-in function (starts slow, speeds up)
    const easeInQuad = (t) => t * t;

    const updateCounter = (currentTime) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);

      // Apply easing function for smooth acceleration
      const easedProgress = easeInQuad(progress);
      const current = target * easedProgress;

      // Format the number based on the original format
      if (element.dataset.format === "currency") {
        element.textContent = "$" + Math.floor(current).toLocaleString();
      } else if (element.dataset.format === "percentage") {
        // Handle decimal percentages
        if (element.dataset.finalValue.includes(".")) {
          element.textContent = current.toFixed(1) + "%";
        } else {
          element.textContent = Math.floor(current) + "%";
        }
      } else if (element.dataset.format === "decimal") {
        element.textContent = current.toFixed(1) + "x";
      } else if (element.dataset.format === "currency-k") {
        // Handle decimal currency-k values (like $1.5k)
        if (element.dataset.finalValue.includes(".")) {
          element.textContent = "$" + current.toFixed(1) + "k";
        } else {
          element.textContent = "$" + Math.floor(current) + "k";
        }
      } else {
        element.textContent = Math.floor(current).toLocaleString();
      }

      if (progress < 1) {
        requestAnimationFrame(updateCounter);
      } else {
        // Set final value to ensure accuracy
        element.textContent = element.dataset.finalValue;
      }
    };

    requestAnimationFrame(updateCounter);
  }

  // Find all metric numbers and animate them
  const metricNumbers = document.querySelectorAll(".metric-number");
  metricNumbers.forEach((element, index) => {
    const text = element.textContent.trim();
    let targetValue = 0;
    let format = "";

    // Parse the value and determine format
    if (text.startsWith("$") && text.endsWith("k")) {
      // Handle $XXk format (including decimals like $1.5k)
      targetValue = parseFloat(text.replace(/[$k,]/g, ""));
      format = "currency-k";
    } else if (text.startsWith("$")) {
      // Handle regular currency
      targetValue = parseInt(text.replace(/[$,]/g, ""));
      format = "currency";
    } else if (text.endsWith("%")) {
      // Handle percentage (including decimals)
      targetValue = parseFloat(text.replace("%", ""));
      format = "percentage";
    } else if (text.endsWith("x")) {
      // Handle multiplier format
      targetValue = parseFloat(text.replace("x", ""));
      format = "decimal";
    } else {
      // Handle regular numbers
      targetValue = parseInt(text.replace(/,/g, ""));
      format = "number";
    }

    // Store original value and format
    element.dataset.finalValue = text;
    element.dataset.format = format;

    // Start from 0 with appropriate format
    if (format === "currency") {
      element.textContent = "$0";
    } else if (format === "currency-k") {
      element.textContent = "$0k";
    } else if (format === "percentage" || format === "decimal") {
      element.textContent = "0";
    } else {
      element.textContent = "0";
    }

    // Stagger animation start for visual effect
    setTimeout(
      () => {
        animateCounter(element, targetValue);
      },
      100 + index * 50,
    );
  });
});
