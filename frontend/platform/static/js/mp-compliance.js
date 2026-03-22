/**
 * Compliance & OJK Reports — mp-compliance.js
 */
(function() {
  'use strict';

  function triggerDownload(url, filename) {
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    if (typeof mpToast !== 'undefined') {
       mpToast(`Exporting ${filename}...`, 'success');
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    // 1. OJK Report
    const btnOjk = document.getElementById('btn-export-ojk');
    if (btnOjk) {
      btnOjk.addEventListener('click', () => {
        const quarter = document.getElementById('ojk-quarter')?.value || '2026-Q1';
        const url = `/api/admin/marketplace/compliance/ojk-report?quarter=${encodeURIComponent(quarter)}`;
        triggerDownload(url, `ojk_report_${quarter}.csv`);
      });
    }

    // 2. AML Travel Rule Data
    const btnAml = document.getElementById('btn-export-aml');
    if (btnAml) {
      btnAml.addEventListener('click', () => {
        const start = document.getElementById('aml-start')?.value || '';
        const end = document.getElementById('aml-end')?.value || '';
        let url = `/api/admin/marketplace/compliance/travel-rule`;
        const params = new URLSearchParams();
        if (start) params.append('from_date', start);
        if (end) params.append('to_date', end);
        if (params.toString()) url += `?${params.toString()}`;
        
        triggerDownload(url, `travel_rule_export.csv`);
      });
    }

    // 3. Tax Export
    const btnTax = document.getElementById('btn-export-tax');
    if (btnTax) {
      btnTax.addEventListener('click', () => {
        const year = document.getElementById('tax-year')?.value || '2025';
        const url = `/api/admin/marketplace/compliance/tax-export?year=${encodeURIComponent(year)}`;
        triggerDownload(url, `tax_export_${year}.csv`);
      });
    }
  });
})();
