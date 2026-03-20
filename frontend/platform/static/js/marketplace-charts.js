// frontend/platform/static/js/marketplace-charts.js
// ApexCharts Candlestick + Volume for POOOL Trading Page

class MarketplaceChart {
    constructor(containerId, assetId) {
        this.container = document.getElementById(containerId);
        this.assetId = assetId;
        this.chart = null;
        this.currentInterval = '1h';
    }

    async init() {
        if (!this.container || typeof ApexCharts === 'undefined') {
            console.warn('[Chart] ApexCharts not loaded or container missing');
            return;
        }

        const mockData = this._generateMockCandles(60);
        this._renderChart(mockData);

        // Listen for live trades
        if (window.marketBus) {
            window.marketBus.on('trade:executed', (trade) => {
                this._appendLiveCandle(trade);
            });
        }
    }

    _renderChart(data) {
        const candleData = data.map(d => ({
            x: new Date(d.time),
            y: [d.open, d.high, d.low, d.close]
        }));

        const volumeData = data.map(d => ({
            x: new Date(d.time),
            y: d.volume,
        }));

        // Light theme colors
        const bg = '#ffffff';
        const foreColor = '#6b7280';
        const gridColor = 'rgba(0,0,0,0.06)';
        const axisBorder = 'rgba(0,0,0,0.08)';
        const crosshairColor = 'rgba(0,0,0,0.08)';
        const volColor = 'rgba(0,0,0,0.06)';

        const options = {
            chart: {
                type: 'candlestick',
                height: 420,
                background: bg,
                fontFamily: "'TT Norms Pro', sans-serif",
                foreColor: foreColor,
                toolbar: { show: false },
                animations: {
                    enabled: true,
                    easing: 'easeinout',
                    speed: 400,
                    dynamicAnimation: { enabled: true, speed: 300 }
                },
            },
            series: [
                {
                    name: 'Price',
                    type: 'candlestick',
                    data: candleData
                },
                {
                    name: 'Volume',
                    type: 'bar',
                    data: volumeData
                }
            ],
            plotOptions: {
                candlestick: {
                    colors: {
                        upward: '#22c55e',
                        downward: '#ef4444'
                    },
                    wick: { useFillColor: true }
                },
                bar: {
                    columnWidth: '60%',
                }
            },
            colors: ['#22c55e', volColor],
            fill: {
                type: ['solid', 'solid'],
                opacity: [1, 0.3]
            },
            grid: {
                borderColor: gridColor,
                strokeDashArray: 3,
                xaxis: { lines: { show: false } },
                yaxis: { lines: { show: true } },
            },
            xaxis: {
                type: 'datetime',
                labels: {
                    style: { colors: foreColor, fontSize: '11px' },
                    datetimeUTC: false
                },
                axisBorder: { color: axisBorder },
                axisTicks: { color: axisBorder },
                crosshairs: {
                    stroke: { color: crosshairColor, width: 1, dashArray: 4 }
                }
            },
            yaxis: [
                {
                    seriesName: 'Price',
                    tooltip: { enabled: true },
                    labels: {
                        style: { colors: foreColor, fontSize: '11px' },
                        formatter: (val) => '$' + val.toFixed(2),
                        offsetX: -8
                    },
                    forceNiceScale: true,
                },
                {
                    seriesName: 'Volume',
                    opposite: true,
                    show: false,
                    max: (max) => max * 4,
                }
            ],
            tooltip: {
                theme: 'light',
                style: { fontSize: '12px', fontFamily: "'TT Norms Pro', sans-serif" },
                x: { format: 'dd MMM HH:mm' },
                custom: ({ seriesIndex, dataPointIndex, w }) => {
                    if (seriesIndex === 0) {
                        const o = w.globals.seriesCandleO[0][dataPointIndex];
                        const h = w.globals.seriesCandleH[0][dataPointIndex];
                        const l = w.globals.seriesCandleL[0][dataPointIndex];
                        const c = w.globals.seriesCandleC[0][dataPointIndex];
                        const isUp = c >= o;
                        const tipBg = '#ffffff';
                        const tipColor = '#1a1d27';
                        return `<div style="padding:10px 14px;font-size:12px;line-height:1.6;background:${tipBg};color:${tipColor};">
                            <div style="color:${isUp ? '#22c55e' : '#ef4444'};font-weight:700;margin-bottom:4px;">${isUp ? '▲' : '▼'} $${c.toFixed(2)}</div>
                            <div>O: <b>$${o.toFixed(2)}</b></div>
                            <div>H: <b>$${h.toFixed(2)}</b></div>
                            <div>L: <b>$${l.toFixed(2)}</b></div>
                            <div>C: <b>$${c.toFixed(2)}</b></div>
                        </div>`;
                    }
                    return '';
                }
            },
            legend: { show: false },
            stroke: { width: [1, 0] },
        };

        if (this.chart) {
            this.chart.destroy();
        }

        this.chart = new ApexCharts(this.container, options);
        this.chart.render();
    }

    _generateMockCandles(count) {
        const data = [];
        const now = Date.now();
        let intervalMs;

        switch (this.currentInterval) {
            case '1m': intervalMs = 60 * 1000; break;
            case '1h': intervalMs = 3600 * 1000; break;
            case '1d': intervalMs = 86400 * 1000; break;
            case '1w': intervalMs = 7 * 86400 * 1000; break;
            default: intervalMs = 3600 * 1000;
        }

        let lastClose = 105.00;
        for (let i = count; i > 0; i--) {
            const time = now - (i * intervalMs);
            const volatility = this.currentInterval === '1m' ? 0.5 : 2.0;
            const open = lastClose;
            const close = open + (Math.random() - 0.48) * volatility;
            const high = Math.max(open, close) + Math.random() * (volatility * 0.8);
            const low = Math.min(open, close) - Math.random() * (volatility * 0.8);
            const volume = Math.floor(Math.random() * 80) + 10;

            data.push({
                time,
                open: parseFloat(open.toFixed(2)),
                high: parseFloat(high.toFixed(2)),
                low: parseFloat(low.toFixed(2)),
                close: parseFloat(close.toFixed(2)),
                volume
            });
            lastClose = close;
        }
        return data;
    }

    _appendLiveCandle(trade) {
        if (!this.chart) return;
        const price = trade.price / 100;
        const time = new Date(trade.timestamp).getTime();

        this.chart.appendData([
            { data: [{ x: time, y: [price, price + 0.1, price - 0.1, price] }] },
            { data: [{ x: time, y: trade.quantity }] }
        ]);
    }

    switchInterval(interval) {
        this.currentInterval = interval;

        document.querySelectorAll('.mkt-interval-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.interval === interval);
        });

        const newData = this._generateMockCandles(60);
        this._renderChart(newData);
    }
}
