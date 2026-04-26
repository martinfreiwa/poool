/**
 * Compliance & OJK Reports — mp-compliance.js
 */
(function() {
  'use strict';

  function showExportStatus(button, message, type) {
    let status = button.parentElement?.querySelector('.mp-export-status');
    if (!status) {
      status = document.createElement('div');
      status.className = 'mp-export-status';
      status.setAttribute('role', 'status');
      status.setAttribute('aria-live', 'polite');
      status.style.marginTop = '10px';
      status.style.fontSize = '13px';
      button.parentElement?.appendChild(status);
    }
    status.textContent = message;
    status.style.color = type === 'error' ? 'var(--admin-danger, #DC2626)' : 'var(--admin-text-muted)';
  }

  function setButtonLoading(button, isLoading) {
    button.disabled = isLoading;
    button.setAttribute('aria-busy', String(isLoading));
    if (isLoading) {
      button.dataset.originalHtml = button.innerHTML;
      button.textContent = 'Exporting...';
    } else if (button.dataset.originalHtml) {
      button.innerHTML = button.dataset.originalHtml;
      delete button.dataset.originalHtml;
    }
  }

  async function triggerDownload(button, url, filename) {
    if (button.disabled) return;
    setButtonLoading(button, true);
    showExportStatus(button, `Preparing ${filename}...`, 'info');

    try {
      const response = await fetch(url, {
        method: 'GET',
        credentials: 'same-origin',
        headers: { Accept: 'text/csv' }
      });
      const contentType = response.headers.get('content-type') || '';
      if (!response.ok) {
        let detail = '';
        try {
          const body = await response.json();
          detail = body.error ? ` ${body.error}` : '';
        } catch (_err) {
          detail = '';
        }
        throw new Error(`Export failed with HTTP ${response.status}.${detail}`);
      }
      if (!contentType.includes('text/csv')) {
        throw new Error('Export returned an unexpected response type.');
      }

      const blob = await response.blob();
      const objectUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = objectUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(objectUrl);
      showExportStatus(button, `${filename} downloaded.`, 'success');
      if (typeof mpToast !== 'undefined') {
        mpToast(`${filename} downloaded.`, 'success');
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Export failed.';
      showExportStatus(button, message, 'error');
      if (typeof mpToast !== 'undefined') {
        mpToast(message, 'error');
      }
    } finally {
      setButtonLoading(button, false);
    }
  }

  function validateDateRange(start, end) {
    if (start && end && start > end) {
      return 'Start date cannot be after end date.';
    }
    return '';
  }

  function defaultQuarter() {
    const now = new Date();
    const quarter = Math.floor(now.getMonth() / 3) + 1;
    return `${now.getFullYear()}-Q${quarter}`;
  }

  document.addEventListener('DOMContentLoaded', () => {
    // 1. OJK Report
    const btnOjk = document.getElementById('btn-export-ojk');
    if (btnOjk) {
      btnOjk.addEventListener('click', () => {
        const quarter = document.getElementById('ojk-quarter')?.value || defaultQuarter();
        const url = `/api/admin/marketplace/compliance/ojk-report?quarter=${encodeURIComponent(quarter)}`;
        triggerDownload(btnOjk, url, `ojk_report_${quarter}.csv`);
      });
    }

    // 2. AML Travel Rule Data
    const btnAml = document.getElementById('btn-export-aml');
    if (btnAml) {
      btnAml.addEventListener('click', () => {
        const start = document.getElementById('aml-start')?.value || '';
        const end = document.getElementById('aml-end')?.value || '';
        const validationError = validateDateRange(start, end);
        if (validationError) {
          showExportStatus(btnAml, validationError, 'error');
          if (typeof mpToast !== 'undefined') {
            mpToast(validationError, 'error');
          }
          return;
        }
        let url = `/api/admin/marketplace/compliance/travel-rule`;
        const params = new URLSearchParams();
        if (start) params.append('from_date', start);
        if (end) params.append('to_date', end);
        if (params.toString()) url += `?${params.toString()}`;
        
        triggerDownload(btnAml, url, `travel_rule_export.csv`);
      });
    }

    // 3. Tax Export
    const btnTax = document.getElementById('btn-export-tax');
    if (btnTax) {
      btnTax.addEventListener('click', () => {
        const year = document.getElementById('tax-year')?.value || '2025';
        const url = `/api/admin/marketplace/compliance/tax-export?year=${encodeURIComponent(year)}`;
        triggerDownload(btnTax, url, `tax_export_${year}.csv`);
      });
    }
  });
})();
