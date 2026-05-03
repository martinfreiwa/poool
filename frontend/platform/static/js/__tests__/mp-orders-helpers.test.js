/**
 * Vitest unit tests for mp-orders pure helpers.
 *   npx vitest run frontend/platform/static/js/__tests__/mp-orders-helpers.test.js
 */
const { describe, test, expect } = require('vitest');
const H = require('../mp-orders-helpers.js');

describe('formatMoney', () => {
  test('zero', () => expect(H.formatMoney(0)).toBe('$0.00'));
  test('cents to dollars', () => expect(H.formatMoney(12345)).toBe('$123.45'));
  test('thousands separator', () => expect(H.formatMoney(100_000_00)).toBe('$100,000.00'));
  test('null/undefined', () => {
    expect(H.formatMoney(null)).toBe('$0.00');
    expect(H.formatMoney(undefined)).toBe('$0.00');
  });
});

describe('ageHours', () => {
  test('fixed reference', () => {
    const now = Date.parse('2026-05-03T12:00:00Z');
    expect(H.ageHours('2026-05-03T06:00:00Z', now)).toBe(6);
    expect(H.ageHours('2026-05-02T12:00:00Z', now)).toBe(24);
  });
  test('invalid date', () => expect(H.ageHours('not-a-date')).toBe(null));
  test('future date clamps to 0', () => {
    const now = Date.parse('2026-05-03T00:00:00Z');
    expect(H.ageHours('2026-05-04T00:00:00Z', now)).toBe(0);
  });
});

describe('ageBucket', () => {
  test('fresh < 6h', () => expect(H.ageBucket(2)).toBe('fresh'));
  test('aging 6–24h', () => expect(H.ageBucket(12)).toBe('aging'));
  test('stale ≥ 24h', () => expect(H.ageBucket(48)).toBe('stale'));
  test('unknown when null', () => expect(H.ageBucket(null)).toBe('unknown'));
});

describe('heldThresholdClass', () => {
  test('neutral < $10k', () => expect(H.heldThresholdClass(500_000)).toBe('admin-badge--neutral'));
  test('warning $10k–$100k', () => expect(H.heldThresholdClass(5_000_000)).toBe('admin-badge--warning'));
  test('danger ≥ $100k', () => expect(H.heldThresholdClass(15_000_000)).toBe('admin-badge--danger'));
  test('boundary $10k', () => expect(H.heldThresholdClass(1_000_000)).toBe('admin-badge--warning'));
  test('boundary $100k', () => expect(H.heldThresholdClass(10_000_000)).toBe('admin-badge--danger'));
});

describe('isInternalUser', () => {
  test('admin literal', () => expect(H.isInternalUser({ user_email: 'admin' })).toBe(true));
  test('admin@', () => expect(H.isInternalUser({ user_email: 'admin@example.com' })).toBe(true));
  test('+admin tag', () => expect(H.isInternalUser({ user_email: 'ops+admin@x.com' })).toBe(true));
  test('@poool.app domain', () => expect(H.isInternalUser({ user_email: 'jane@poool.app' })).toBe(true));
  test('@poool.internal domain', () => expect(H.isInternalUser({ user_email: 'sys@poool.internal' })).toBe(true));
  test('regular user', () => expect(H.isInternalUser({ user_email: 'jane@gmail.com' })).toBe(false));
  test('missing email', () => expect(H.isInternalUser({})).toBe(false));
});

describe('heldCents', () => {
  test('price * remaining', () => {
    expect(H.heldCents({ price_cents: 50000, quantity: 10, quantity_filled: 0 })).toBe(500_000);
    expect(H.heldCents({ price_cents: 50000, quantity: 10, quantity_filled: 3 })).toBe(350_000);
  });
  test('zero remaining', () => {
    expect(H.heldCents({ price_cents: 50000, quantity: 5, quantity_filled: 5 })).toBe(0);
  });
});

describe('anomalyFlags', () => {
  const base = { user_email: 'jane@gmail.com', price_cents: 100, quantity: 1, quantity_filled: 0, created_at: '2026-05-03T12:00:00Z' };
  const now = Date.parse('2026-05-03T13:00:00Z');

  test('no flags on benign order', () => {
    expect(H.anomalyFlags(base, { now })).toEqual([]);
  });
  test('large hold flag', () => {
    const o = { ...base, price_cents: 600_000_0, quantity: 1 }; // $60k
    const flags = H.anomalyFlags(o, { now });
    expect(flags.some((f) => f.label === 'Large hold')).toBe(true);
  });
  test('multi-order user flag', () => {
    const flags = H.anomalyFlags(base, { now, userOrders: 5 });
    expect(flags.some((f) => f.label === '5x user')).toBe(true);
  });
  test('internal user flag', () => {
    const flags = H.anomalyFlags({ ...base, user_email: 'admin@poool.app' }, { now });
    expect(flags.some((f) => f.label === 'Internal')).toBe(true);
  });
  test('stale 48h+ flag', () => {
    const old = { ...base, created_at: '2026-04-30T00:00:00Z' };
    const flags = H.anomalyFlags(old, { now });
    expect(flags.some((f) => f.label === 'Stale 48h+')).toBe(true);
  });
});
