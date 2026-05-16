/**
 * Leaderboard demo dataset — split off from leaderboard.js (audit C2).
 *
 * Only loaded when `?demo` is present in the URL, via dynamic import.
 * Production page loads never fetch or parse this file.
 *
 * Exports two ES module symbols:
 *   - `DEMO_RANKINGS`: 20-row fixture used for visual review.
 *   - `getDemoData(realData, currentMetric, currentTimeframe)`: returns a
 *     `LeaderboardResponse`-shaped object sorted by the requested metric.
 */

export const DEMO_RANKINGS = [
  { rank: 1,  display_name: 'Alexander K.',  avatar_url: null, tier_name: 'Premium', tier_badge_color: '#7F56D9', metric_value: 4825000, is_current_user: false, metrics: { total_invested_cents: 4825000, asset_count: 12, portfolio_roi_bps: 1450, affiliate_count: 8,  referral_network_value_cents: 920000,  highest_investment_cents: 1500000 }},
  { rank: 2,  display_name: 'Sophia M.',     avatar_url: null, tier_name: 'Elite',   tier_badge_color: '#2E90FA', metric_value: 3690000, is_current_user: false, metrics: { total_invested_cents: 3690000, asset_count: 9,  portfolio_roi_bps: 1280, affiliate_count: 5,  referral_network_value_cents: 410000,  highest_investment_cents: 1200000 }},
  { rank: 3,  display_name: 'Maximilian R.', avatar_url: null, tier_name: 'Elite',   tier_badge_color: '#2E90FA', metric_value: 2850000, is_current_user: false, metrics: { total_invested_cents: 2850000, asset_count: 7,  portfolio_roi_bps: 1120, affiliate_count: 3,  referral_network_value_cents: 180000,  highest_investment_cents: 950000  }},
  { rank: 4,  display_name: 'Emma L.',       avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 2150000, is_current_user: false, metrics: { total_invested_cents: 2150000, asset_count: 6,  portfolio_roi_bps: 980,  affiliate_count: 11, referral_network_value_cents: 750000,  highest_investment_cents: 800000  }},
  { rank: 5,  display_name: 'Lukas W.',      avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 1780000, is_current_user: false, metrics: { total_invested_cents: 1780000, asset_count: 5,  portfolio_roi_bps: 1340, affiliate_count: 2,  referral_network_value_cents: 95000,   highest_investment_cents: 750000  }},
  { rank: 6,  display_name: 'Hannah B.',     avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 1320000, is_current_user: false, metrics: { total_invested_cents: 1320000, asset_count: 4,  portfolio_roi_bps: 870,  affiliate_count: 6,  referral_network_value_cents: 320000,  highest_investment_cents: 600000  }},
  { rank: 7,  display_name: 'Noah S.',       avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 980000,  is_current_user: false, metrics: { total_invested_cents: 980000,  asset_count: 3,  portfolio_roi_bps: 760,  affiliate_count: 1,  referral_network_value_cents: 45000,   highest_investment_cents: 500000  }},
  { rank: 8,  display_name: 'Mia F.',        avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 750000,  is_current_user: false, metrics: { total_invested_cents: 750000,  asset_count: 3,  portfolio_roi_bps: 920,  affiliate_count: 4,  referral_network_value_cents: 210000,  highest_investment_cents: 350000  }},
  { rank: 9,  display_name: 'Julian D.',     avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 520000,  is_current_user: false, metrics: { total_invested_cents: 520000,  asset_count: 2,  portfolio_roi_bps: 650,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 300000  }},
  { rank: 10, display_name: 'Lena V.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 350000,  is_current_user: false, metrics: { total_invested_cents: 350000,  asset_count: 1,  portfolio_roi_bps: 480,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 350000  }},
  { rank: 11, display_name: 'Oliver P.',     avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 310000,  is_current_user: false, metrics: { total_invested_cents: 310000,  asset_count: 2,  portfolio_roi_bps: 540,  affiliate_count: 3,  referral_network_value_cents: 62000,   highest_investment_cents: 200000  }},
  { rank: 12, display_name: 'Isabelle T.',   avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 275000,  is_current_user: false, metrics: { total_invested_cents: 275000,  asset_count: 1,  portfolio_roi_bps: 390,  affiliate_count: 1,  referral_network_value_cents: 18000,   highest_investment_cents: 275000  }},
  { rank: 13, display_name: 'Marcus H.',     avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 240000,  is_current_user: false, metrics: { total_invested_cents: 240000,  asset_count: 1,  portfolio_roi_bps: 310,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 240000  }},
  { rank: 14, display_name: 'Clara N.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 195000,  is_current_user: false, metrics: { total_invested_cents: 195000,  asset_count: 1,  portfolio_roi_bps: 280,  affiliate_count: 2,  referral_network_value_cents: 9500,    highest_investment_cents: 195000  }},
  { rank: 15, display_name: 'Felix A.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 160000,  is_current_user: false, metrics: { total_invested_cents: 160000,  asset_count: 1,  portfolio_roi_bps: 220,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 160000  }},
  { rank: 16, display_name: 'Anna C.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 132000,  is_current_user: false, metrics: { total_invested_cents: 132000,  asset_count: 1,  portfolio_roi_bps: 190,  affiliate_count: 1,  referral_network_value_cents: 5000,    highest_investment_cents: 132000  }},
  { rank: 17, display_name: 'David R.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 110000,  is_current_user: false, metrics: { total_invested_cents: 110000,  asset_count: 1,  portfolio_roi_bps: 160,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 110000  }},
  { rank: 18, display_name: 'Yuki T.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 85000,   is_current_user: false, metrics: { total_invested_cents: 85000,   asset_count: 1,  portfolio_roi_bps: 120,  affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 85000   }},
  { rank: 19, display_name: 'Sara K.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 60000,   is_current_user: false, metrics: { total_invested_cents: 60000,   asset_count: 1,  portfolio_roi_bps: 90,   affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 60000   }},
  { rank: 20, display_name: 'Tom B.',        avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 40000,   is_current_user: true,  metrics: { total_invested_cents: 40000,   asset_count: 1,  portfolio_roi_bps: 60,   affiliate_count: 0,  referral_network_value_cents: 0,       highest_investment_cents: 40000   }},
];

export function getDemoData(realData, currentMetric, currentTimeframe) {
  var metricKey = {
    'invested':    'total_invested_cents',
    'assets':      'asset_count',
    'roi':         'portfolio_roi_bps',
    'affiliates':  'affiliate_count',
    'revenue':     'referral_network_value_cents',
    'highest_inv': 'highest_investment_cents',
  }[currentMetric] || 'total_invested_cents';

  var timeframeMultiplier = currentTimeframe === 'weekly' ? 0.05 : (currentTimeframe === 'monthly' ? 0.2 : 1);
  var copied = JSON.parse(JSON.stringify(DEMO_RANKINGS));
  var sorted = copied.sort(function (a, b) {
    return b.metrics[metricKey] - a.metrics[metricKey];
  });

  sorted.forEach(function (entry, i) {
    entry.rank = i + 1;
    if (metricKey !== 'asset_count' && metricKey !== 'affiliate_count' && metricKey !== 'portfolio_roi_bps') {
       entry.metrics[metricKey] = Math.round(entry.metrics[metricKey] * timeframeMultiplier);
    }
    entry.metric_value = entry.metrics[metricKey];
  });

  var mockMe = sorted.find(function(r) { return r.is_current_user; }) || sorted[sorted.length-1];
  return {
    rankings: sorted,
    my_rank: (realData && realData.my_rank && realData.my_rank.rank) ? realData.my_rank : mockMe,
    total_participants: 20,
    metric_type: currentMetric,
    timeframe: currentTimeframe,
    last_updated: new Date().toISOString(),
    has_more: false,
  };
}
