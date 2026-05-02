/**
 * mp-compliance-i18n.js — minimal en/id dictionary for the compliance page.
 * Persists locale choice in localStorage. Apply via [data-i18n="key"] on
 * any element. Returns key unchanged when missing — graceful fallback.
 */
(function () {
  'use strict';

  const DICT = {
    en: {
      'page.title': 'Compliance & OJK Reports',
      'page.subtitle': 'Generate regulatory reports and access Travel-Rule datasets required by OJK and Bappebti.',
      'health.title': 'Regulatory deadlines',
      'health.refresh': 'Refresh',
      'card.required': 'Required',
      'card.ojk.title': 'OJK Quarterly Report',
      'card.travel.title': 'AML / Travel-Rule Data',
      'card.tax.title': 'Tax & Fiscal Exports',
      'field.period': 'Reporting Period',
      'field.format': 'Format',
      'field.start': 'Start Date',
      'field.end': 'End Date',
      'field.year': 'Fiscal Year',
      'preview.rows': 'Rows',
      'preview.excluded': 'Excluded',
      'preview.size': 'Est. size',
      'preview.last': 'Last export',
      'preview.never': 'Never',
      'btn.download': 'Download Report',
      'btn.export.travel': 'Request Approval & Export',
      'btn.export.tax': 'Export Fiscal Data',
      'btn.refresh': 'Refresh',
      'history.title': 'Recent Exports',
      'history.allTypes': 'All types',
      'modal.confirmTitle': 'Confirm export',
      'modal.cancel': 'Cancel',
      'modal.go': 'Generate & Download',
      'approval.title': 'Pending approval requests',
      'approval.empty': 'No pending requests.',
      'approval.approve': 'Approve',
      'approval.deny': 'Deny',
      'approval.requestBtn': 'Request Approval',
      'schedule.title': 'Auto-Schedule',
      'schedule.empty': 'No schedules configured.',
      'schedule.create': 'Create schedule',
      'schedule.cadence': 'Cadence',
      'schedule.email': 'Delivery email',
      'compare.title': 'vs prior quarter',
      'compare.volume': 'Volume',
      'compare.trades': 'Trades',
      'shortcuts.title': 'Keyboard shortcuts',
      'shortcuts.help': 'Press ? for help',
      'err.dateOrder': 'Start date cannot be after end date.',
    },
    id: {
      'page.title': 'Kepatuhan & Laporan OJK',
      'page.subtitle': 'Buat laporan regulasi dan akses data Travel-Rule yang dibutuhkan OJK dan Bappebti.',
      'health.title': 'Tenggat regulasi',
      'health.refresh': 'Segarkan',
      'card.required': 'Wajib',
      'card.ojk.title': 'Laporan Kuartalan OJK',
      'card.travel.title': 'Data AML / Travel-Rule',
      'card.tax.title': 'Ekspor Pajak & Fiskal',
      'field.period': 'Periode Pelaporan',
      'field.format': 'Format',
      'field.start': 'Tanggal Mulai',
      'field.end': 'Tanggal Akhir',
      'field.year': 'Tahun Fiskal',
      'preview.rows': 'Baris',
      'preview.excluded': 'Dikecualikan',
      'preview.size': 'Estimasi ukuran',
      'preview.last': 'Ekspor terakhir',
      'preview.never': 'Belum pernah',
      'btn.download': 'Unduh Laporan',
      'btn.export.travel': 'Minta Persetujuan & Ekspor',
      'btn.export.tax': 'Ekspor Data Fiskal',
      'btn.refresh': 'Segarkan',
      'history.title': 'Ekspor Terbaru',
      'history.allTypes': 'Semua tipe',
      'modal.confirmTitle': 'Konfirmasi ekspor',
      'modal.cancel': 'Batal',
      'modal.go': 'Buat & Unduh',
      'approval.title': 'Permintaan persetujuan tertunda',
      'approval.empty': 'Tidak ada permintaan tertunda.',
      'approval.approve': 'Setujui',
      'approval.deny': 'Tolak',
      'approval.requestBtn': 'Minta Persetujuan',
      'schedule.title': 'Jadwal Otomatis',
      'schedule.empty': 'Belum ada jadwal.',
      'schedule.create': 'Buat jadwal',
      'schedule.cadence': 'Frekuensi',
      'schedule.email': 'Email pengiriman',
      'compare.title': 'vs kuartal lalu',
      'compare.volume': 'Volume',
      'compare.trades': 'Transaksi',
      'shortcuts.title': 'Pintasan keyboard',
      'shortcuts.help': 'Tekan ? untuk bantuan',
      'err.dateOrder': 'Tanggal mulai tidak boleh setelah tanggal akhir.',
    },
  };

  let locale = localStorage.getItem('mp-compliance-locale') || 'en';
  if (!DICT[locale]) locale = 'en';

  function t(key) {
    return (DICT[locale] && DICT[locale][key]) || (DICT.en[key] || key);
  }

  function setLocale(l) {
    if (!DICT[l]) return;
    locale = l;
    localStorage.setItem('mp-compliance-locale', l);
    apply();
    document.dispatchEvent(new CustomEvent('mp-locale-change', { detail: { locale } }));
  }

  function apply(root) {
    const scope = root || document;
    scope.querySelectorAll('[data-i18n]').forEach((el) => {
      const key = el.getAttribute('data-i18n');
      if (key) el.textContent = t(key);
    });
    scope.querySelectorAll('[data-i18n-attr]').forEach((el) => {
      const pairs = el.getAttribute('data-i18n-attr').split(';');
      pairs.forEach((p) => {
        const [attr, key] = p.split(':');
        if (attr && key) el.setAttribute(attr.trim(), t(key.trim()));
      });
    });
  }

  window.MPI18n = { t, setLocale, apply, get locale() { return locale; } };
  document.addEventListener('DOMContentLoaded', () => apply());
})();
