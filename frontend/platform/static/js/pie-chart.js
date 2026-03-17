/**
 * Dynamic Pie Chart Generator
 * Generates interactive pie charts with hover effects and percentage pointers
 */

class PieChart {
  constructor(containerId, data, options = {}) {
    this.container = document.getElementById(containerId);
    if (!this.container) {
      return;
    }

    this.data = data;
    this.options = {
      radius: options.radius || 80,
      centerHoleRadius: options.centerHoleRadius || 25,
      width: options.width || 300,
      height: options.height || 300,
      showPointers: options.showPointers !== false,
      pointerLength: options.pointerLength || 40,
      hoverScale: options.hoverScale || 1.08,
      backgroundColor: options.backgroundColor || "#FCFCFD",
      ...options,
    };

    // Position center to balance upward pointer lines and pie
    this.centerX = this.options.width / 2;
    this.centerY = this.options.height / 2 + 20; // Center pie with slight offset for pointer lines

    this.init();
  }

  init() {
    this.container.innerHTML = "";
    this.createSVG();
    this.renderChart();
  }

  createSVG() {
    this.svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    this.svg.setAttribute(
      "viewBox",
      `0 0 ${this.options.width} ${this.options.height}`,
    );
    this.svg.setAttribute("width", this.options.width);
    this.svg.setAttribute("height", this.options.height);
    this.svg.style.overflow = "visible";
    this.container.appendChild(this.svg);
  }

  polarToCartesian(centerX, centerY, radius, angleInDegrees) {
    const angleInRadians = ((angleInDegrees - 90) * Math.PI) / 180.0;
    return {
      x: centerX + radius * Math.cos(angleInRadians),
      y: centerY + radius * Math.sin(angleInRadians),
    };
  }

  createArcPath(centerX, centerY, radius, startAngle, endAngle) {
    const start = this.polarToCartesian(centerX, centerY, radius, endAngle);
    const end = this.polarToCartesian(centerX, centerY, radius, startAngle);
    const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";

    return [
      "M",
      centerX,
      centerY,
      "L",
      start.x,
      start.y,
      "A",
      radius,
      radius,
      0,
      largeArcFlag,
      0,
      end.x,
      end.y,
      "Z",
    ].join(" ");
  }

  getMidAngle(startAngle, endAngle) {
    return startAngle + (endAngle - startAngle) / 2;
  }

  renderChart() {
    let currentAngle = 0;

    // Create group for segments
    const segmentsGroup = document.createElementNS(
      "http://www.w3.org/2000/svg",
      "g",
    );
    segmentsGroup.setAttribute("class", "pie-segments");
    this.svg.appendChild(segmentsGroup);

    // Create group for pointers (rendered on top)
    const pointersGroup = document.createElementNS(
      "http://www.w3.org/2000/svg",
      "g",
    );
    pointersGroup.setAttribute("class", "pie-pointers");
    this.svg.appendChild(pointersGroup);

    this.data.forEach((segment, index) => {
      const angle = (segment.percentage / 100) * 360;
      const endAngle = currentAngle + angle;

      // Create segment group
      const segmentGroup = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "g",
      );
      segmentGroup.setAttribute("class", "pie-segment");
      segmentGroup.setAttribute("data-index", index);
      segmentGroup.setAttribute("data-label", segment.label);
      segmentGroup.setAttribute("data-percentage", segment.percentage);

      // Create path
      const path = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "path",
      );
      const pathData = this.createArcPath(
        this.centerX,
        this.centerY,
        this.options.radius,
        currentAngle,
        endAngle,
      );
      path.setAttribute("d", pathData);
      path.setAttribute("fill", segment.color);
      path.setAttribute("stroke", "none");
      path.style.transformOrigin = `${this.centerX}px ${this.centerY}px`;
      path.style.transition = "transform 0.3s ease, filter 0.3s ease";
      path.style.cursor = "pointer";

      // Add hover effect
      segmentGroup.addEventListener("mouseenter", () => {
        path.style.transform = `scale(${this.options.hoverScale})`;
        path.style.filter = "drop-shadow(0 4px 12px rgba(0, 0, 0, 0.15))";
        segmentGroup.style.zIndex = "10";
      });

      segmentGroup.addEventListener("mouseleave", () => {
        path.style.transform = "scale(1)";
        path.style.filter = "none";
        segmentGroup.style.zIndex = "1";
      });

      segmentGroup.appendChild(path);
      segmentsGroup.appendChild(segmentGroup);

      // Create pointer line and label if enabled
      if (this.options.showPointers) {
        this.createPointer(pointersGroup, currentAngle, endAngle, segment);
      }

      currentAngle = endAngle;
    });

    // Create center hole
    const centerHole = document.createElementNS(
      "http://www.w3.org/2000/svg",
      "circle",
    );
    centerHole.setAttribute("cx", this.centerX);
    centerHole.setAttribute("cy", this.centerY);
    centerHole.setAttribute("r", this.options.centerHoleRadius);
    centerHole.setAttribute("fill", this.options.backgroundColor);
    this.svg.appendChild(centerHole);
  }

  createPointer(group, startAngle, endAngle, segment) {
    const midAngle = this.getMidAngle(startAngle, endAngle);

    // Calculate start point (deeper inside the pie, not at the edge)
    const startRadius = this.options.radius * 0.6; // Start at 60% of radius
    const startPoint = this.polarToCartesian(
      this.centerX,
      this.centerY,
      startRadius,
      midAngle,
    );

    // Lines angle outward and upward - but keep horizontal spread minimal
    const endPoint = this.polarToCartesian(
      this.centerX,
      this.centerY,
      this.options.radius + this.options.pointerLength * 0.3, // Minimal horizontal extension
      midAngle,
    );

    // Push lines much higher to create steep angle
    endPoint.y = endPoint.y - 70;

    // Position text above the line
    const textX = endPoint.x;
    const textY = endPoint.y - 5;

    // Create percentage label (smaller and lighter weight)
    const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
    text.setAttribute("x", textX);
    text.setAttribute("y", textY);
    text.setAttribute("text-anchor", "middle");
    text.setAttribute("fill", "#000000");
    text.setAttribute("font-family", "TT Norms Pro, sans-serif");
    text.setAttribute("font-size", "12");
    text.setAttribute("font-weight", "400");
    text.style.pointerEvents = "none";
    text.textContent = `${segment.percentage}%`;
    group.appendChild(text);

    // Create angled pointer line
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
    line.setAttribute("x1", startPoint.x);
    line.setAttribute("y1", startPoint.y);
    line.setAttribute("x2", endPoint.x);
    line.setAttribute("y2", endPoint.y);
    line.setAttribute("stroke", "#000000");
    line.setAttribute("stroke-width", "1");
    line.style.pointerEvents = "none";
    group.appendChild(line);
  }

  updateData(newData) {
    this.data = newData;
    this.init();
  }
}

// Initialize pie charts when DOM is ready
document.addEventListener("DOMContentLoaded", function () {
  // Common chart data for both desktop and mobile
  const chartData = [
    {
      label: "NEO AGRO",
      percentage: 55,
      color: "#98FB96",
    },
    {
      label: "POOOL AGRO",
      percentage: 45,
      color: "#0000FF",
    },
  ];

  // Desktop pie chart
  const desktopChartContainer = document.getElementById(
    "financials-pie-chart-dynamic",
  );
  if (desktopChartContainer) {
    const desktopChart = new PieChart(
      "financials-pie-chart-dynamic",
      chartData,
      {
        radius: 85,
        centerHoleRadius: 25,
        width: 210,
        height: 210,
        showPointers: true,
        pointerLength: 25,
        hoverScale: 1.08,
        backgroundColor: "#FCFCFD",
      },
    );

    // Store chart instance for potential updates
    window.financialsPieChart = desktopChart;
  }

  // Mobile pie chart
  const mobileChartContainer = document.getElementById(
    "mobile-financials-pie-chart",
  );
  if (mobileChartContainer) {
    const mobileChart = new PieChart("mobile-financials-pie-chart", chartData, {
      radius: 80,
      centerHoleRadius: 22,
      width: 200,
      height: 200,
      showPointers: true,
      pointerLength: 20,
      hoverScale: 1.08,
      backgroundColor: "#FCFCFD",
    });

    // Store chart instance for potential updates
    window.mobileFinancialsPieChart = mobileChart;
  }
});
