/**
 * Pure helpers for mp-orders. Browser-loaded as IIFE, importable in Node tests.
 * No DOM, no fetch — only data transforms.
 */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) {
    module.exports = factory();
  } else {
    root.MpOrdersHelpers = factory();
  }
}(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  function formatMoney(cents) {
    return '$' + (Number(cents || 0) / 100).toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  }

  function ageHours(dateStr, now) {
    const t = new Date(dateStr).getTime();
    if (!Number.isFinite(t)) return null;
    const ref = (typeof now === 'number' ? now : Date.now());
    return Math.max(0, (ref - t) / 3600000);
  }

  function ageBucket(hours) {
    if (hours == null) return 'unknown';
    if (hours < 6) return 'fresh';
    if (hours < 24) return 'aging';
    return 'stale';
  }

  function heldThresholdClass(cents) {
    if (cents >= 10_000_000) return 'admin-badge--danger';
    if (cents >= 1_000_000)  return 'admin-badge--warning';
    return 'admin-badge--neutral';
  }

  function isInternalUser(order) {
    const email = String(order && order.user_email || '').toLowerCase();
    return email === 'admin'
      || email.startsWith('admin@')
      || email.includes('+admin@')
      || email.endsWith('@poool.app')
      || email.endsWith('@poool.internal');
  }

  function remainingQuantity(order) {
    return Number(order && order.quantity || 0) - Number(order && order.quantity_filled || 0);
  }

  function heldCents(order) {
    return Number(order && order.price_cents || 0) * remainingQuantity(order);
  }

  function anomalyFlags(order, opts) {
    const userOrders = (opts && opts.userOrders) || 0;
    const now = (opts && opts.now);
    const flags = [];
    const held = heldCents(order);
    if (held >= 5_000_000) flags.push({ tone: 'danger',  label: 'Large hold' });
    if (userOrders >= 3)   flags.push({ tone: 'warning', label: userOrders + 'x user' });
    if (isInternalUser(order)) flags.push({ tone: 'danger', label: 'Internal' });
    if (ageHours(order.created_at, now) >= 48) flags.push({ tone: 'warning', label: 'Stale 48h+' });
    return flags;
  }

  return {
    formatMoney,
    ageHours,
    ageBucket,
    heldThresholdClass,
    isInternalUser,
    remainingQuantity,
    heldCents,
    anomalyFlags,
  };
}));
