/**
 * Dynamic Pie Chart Generator using ApexCharts
 * Replaces custom SVG generator with a robust, interactive ApexCharts implementation.
 */

document.addEventListener("DOMContentLoaded", function () {
  // Desktop pie chart
  const desktopContainer = document.getElementById("financials-pie-chart-dynamic");
  if (desktopContainer && typeof ApexCharts !== "undefined") {
    const operatorName = desktopContainer.getAttribute("data-operator-name") || "Operator";
    const operatorPct = parseFloat(desktopContainer.getAttribute("data-operator-pct") || "55");
    const pooolPct = parseFloat(desktopContainer.getAttribute("data-poool-pct") || "45");

    const options = {
      series: [operatorPct, pooolPct],
      chart: {
        type: 'donut',
        height: 220,
        animations: {
          enabled: true,
          easing: 'easeinout',
          speed: 800,
          animateGradually: {
            enabled: true,
            delay: 150
          },
          dynamicAnimation: {
            enabled: true,
            speed: 350
          }
        }
      },
      labels: [operatorName, 'POOOL AGRO'],
      colors: ['#98FB96', '#0000FF'],
      stroke: {
        show: false
      },
      dataLabels: {
        enabled: true,
        formatter: function (val) {
          return val.toFixed(0) + "%";
        },
        style: {
          fontSize: '12px',
          fontFamily: 'TT Norms Pro, sans-serif',
          fontWeight: '600',
          colors: ['#000000', '#FFFFFF']
        },
        dropShadow: {
            enabled: false
        }
      },
      plotOptions: {
        pie: {
          donut: {
            size: '45%',
            background: 'transparent'
          },
          expandOnClick: true,
          customScale: 1
        }
      },
      legend: {
        show: false
      },
      tooltip: {
        enabled: true,
        y: {
          formatter: function (val) {
            return val + "%";
          }
        }
      },
      responsive: [{
        breakpoint: 480,
        options: {
          chart: {
            height: 200
          }
        }
      }]
    };

    const chart = new ApexCharts(desktopContainer, options);
    chart.render();
    window.financialsPieChart = chart;
  }

  // Mobile pie chart (if container exists)
  const mobileContainer = document.getElementById("mobile-financials-pie-chart");
  if (mobileContainer && typeof ApexCharts !== "undefined") {
    const operatorPct = parseFloat(mobileContainer.getAttribute("data-operator-pct") || "55");
    const pooolPct = parseFloat(mobileContainer.getAttribute("data-poool-pct") || "45");

    const options = {
      series: [operatorPct, pooolPct],
      chart: {
        type: 'donut',
        height: 180
      },
      labels: ['NEO AGRO', 'POOOL AGRO'],
      colors: ['#98FB96', '#0000FF'],
      stroke: {
        show: false
      },
      dataLabels: {
        enabled: true,
        formatter: function (val) {
          return val.toFixed(0) + "%";
        }
      },
      plotOptions: {
        pie: {
          donut: {
            size: '40%'
          }
        }
      },
      legend: {
        show: false
      }
    };

    const mobileChart = new ApexCharts(mobileContainer, options);
    mobileChart.render();
    window.mobileFinancialsPieChart = mobileChart;
  }
});
