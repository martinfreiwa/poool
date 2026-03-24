document.addEventListener('DOMContentLoaded', () => {
    initCharts();
});

// HTMX hooks: Re-initialize the chart when a partial server response replaces the canvas container.
document.body.addEventListener('htmx:afterSwap', function(event) {
    if (event.detail.target.id === 'hero-chart-container') {
        initPortfolioChart();
    }
});

// UI Interactivity: Filter buttons active state management
document.querySelectorAll('.statistics-filters .ds-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
        // Find siblings inside the same filter group and remove 'active'
        const parent = e.currentTarget.closest('.statistics-filters');
        parent.querySelectorAll('.ds-btn').forEach(b => b.classList.remove('active'));
        e.currentTarget.classList.add('active');
    });
});

// Store chart instances to destroy them before re-rendering
let portfolioChartInstance = null;
let allocationChartInstance = null;
let dividendsChartInstance = null;

function initCharts() {
    initPortfolioChart();
    initAllocationChart();
    initDividendsChart();
}

/**
 * Custom Chart.js Plugin to create the Holographic Glow effect under the line.
 * Mimics our Apple-inspired Glassmorphism UI details.
 */
const glowPlugin = {
    id: 'glow',
    beforeDatasetsDraw: (chart) => {
        const ctx = chart.ctx;
        ctx.save();
        ctx.shadowColor = 'rgba(0, 0, 255, 0.4)'; // Brand Blue shadow
        ctx.shadowBlur = 12;
        ctx.shadowOffsetX = 0;
        ctx.shadowOffsetY = 6;
    },
    afterDatasetsDraw: (chart) => {
        chart.ctx.restore();
    }
};

/**
 * Initializes the main Portfolio Growth Line Chart.
 */
function initPortfolioChart() {
    const canvas = document.getElementById('portfolioChart');
    if (!canvas) return;
    
    // Destroy previous instance to prevent memory leaks during HTMX swaps
    if (portfolioChartInstance) {
        portfolioChartInstance.destroy();
    }

    const ctx = canvas.getContext('2d');
    
    // Create the smooth downward gradient (Holographic effect)
    const gradient = ctx.createLinearGradient(0, 0, 0, canvas.height || 400);
    gradient.addColorStop(0, 'rgba(0, 0, 255, 0.15)'); // Electric Blue at 15% opacity
    gradient.addColorStop(1, 'rgba(0, 0, 255, 0)');    // Fades to transparent

    portfolioChartInstance = new Chart(ctx, {
        type: 'line',
        data: {
            labels: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
            datasets: [{
                label: 'Portfolio Value',
                data: [98000, 102000, 101500, 105000, 108000, 112000, 109000, 115000, 118000, 120000, 122000, 124500],
                borderColor: '#0000FF', // Pure Electric Blue
                backgroundColor: gradient,
                borderWidth: 2,
                tension: 0.4, // Creates the fluid, Apple-like curves
                fill: true,
                pointBackgroundColor: '#FFFFFF',
                pointBorderColor: '#0000FF',
                pointBorderWidth: 2,
                pointRadius: 0, // Hidden by default for clean UI
                pointHoverRadius: 6,
                pointHoverBackgroundColor: '#03FF88', // Signal Green on hover
                pointHoverBorderColor: '#FFFFFF',
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: {
                    display: false // Using custom HTML header
                },
                tooltip: {
                    backgroundColor: 'rgba(255, 255, 255, 0.95)',
                    titleColor: '#181D27',
                    bodyColor: '#344054',
                    borderColor: '#E5E7EB',
                    borderWidth: 1,
                    padding: 12,
                    displayColors: false,
                    titleFont: { family: "'TT Norms Pro', sans-serif", size: 13, weight: '600' },
                    bodyFont: { family: "'TT Norms Pro', sans-serif", size: 14, weight: '500' },
                    callbacks: {
                        label: function(context) {
                            let label = context.dataset.label || '';
                            if (label) label += ': ';
                            if (context.parsed.y !== null) {
                                label += new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(context.parsed.y);
                            }
                            return label;
                        }
                    }
                }
            },
            scales: {
                x: {
                    grid: {
                        display: false,
                        drawBorder: false,
                    },
                    ticks: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 12 },
                        color: '#667085'
                    }
                },
                y: {
                    grid: {
                        color: '#E9EAEB',
                        drawBorder: false,
                        borderDash: [4, 4]
                    },
                    ticks: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 12 },
                        color: '#667085',
                        callback: function(value) {
                            return '$' + value / 1000 + 'k';
                        }
                    }
                }
            }
        },
        plugins: [glowPlugin]
    });
}

/**
 * Initializes the Asset Allocation Donut Chart.
 */
function initAllocationChart() {
    const canvas = document.getElementById('allocationChart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    
    allocationChartInstance = new Chart(ctx, {
        type: 'doughnut',
        data: {
            labels: ['Real Estate', 'Commodities', 'Private Equity', 'Cash'],
            datasets: [{
                data: [55, 25, 15, 5],
                backgroundColor: [
                    '#0000FF', // Electric Blue
                    '#03FF88', // Signal Green
                    '#8B5CF6', // Accent Purple
                    '#E5E7EB'  // Gray neutral
                ],
                borderWidth: 0,
                hoverOffset: 6
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            cutout: '75%', // Premium thin ring
            plugins: {
                legend: {
                    position: 'right',
                    labels: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 13 },
                        color: '#344054',
                        usePointStyle: true,
                        pointStyle: 'circle',
                        padding: 20
                    }
                },
                tooltip: {
                    backgroundColor: 'rgba(255, 255, 255, 0.95)',
                    titleColor: '#181D27',
                    bodyColor: '#344054',
                    borderColor: '#E5E7EB',
                    borderWidth: 1,
                    padding: 12,
                    titleFont: { family: "'TT Norms Pro', sans-serif", size: 13, weight: '600' },
                    bodyFont: { family: "'TT Norms Pro', sans-serif", size: 14, weight: '500' }
                }
            }
        }
    });
}

/**
 * Initializes the Monthly Dividends Bar Chart.
 */
function initDividendsChart() {
    const canvas = document.getElementById('dividendsChart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    
    dividendsChartInstance = new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
            datasets: [{
                label: 'Yield ($)',
                data: [320, 345, 330, 360, 390, 410],
                backgroundColor: '#98FB96', // Secondary Button Mint
                borderRadius: 4, // Rounded bars
                barThickness: 16
            },
            {
                label: 'Capital Gains ($)',
                data: [120, 100, 150, 140, 180, 210],
                backgroundColor: '#0000FF', // Electric Blue
                borderRadius: 4,
                barThickness: 16
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: {
                    position: 'top',
                    align: 'end',
                    labels: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 12 },
                        color: '#667085',
                        usePointStyle: true,
                        boxWidth: 8
                    }
                },
                tooltip: {
                     backgroundColor: 'rgba(255, 255, 255, 0.95)',
                     titleColor: '#181D27',
                     bodyColor: '#344054',
                     borderColor: '#E5E7EB',
                     borderWidth: 1,
                     padding: 12,
                     titleFont: { family: "'TT Norms Pro', sans-serif", size: 13, weight: '600' },
                     bodyFont: { family: "'TT Norms Pro', sans-serif", size: 14, weight: '500' }
                }
            },
            scales: {
                x: {
                    stacked: true,
                    grid: {
                        display: false,
                        drawBorder: false,
                    },
                    ticks: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 12 },
                        color: '#667085'
                    }
                },
                y: {
                    stacked: true,
                    grid: {
                        color: '#E9EAEB',
                        drawBorder: false,
                        borderDash: [4, 4]
                    },
                    ticks: {
                        font: { family: "'TT Norms Pro', sans-serif", size: 12 },
                        color: '#667085',
                        callback: function(value) {
                            return '$' + value;
                        }
                    }
                }
            }
        }
    });
}
