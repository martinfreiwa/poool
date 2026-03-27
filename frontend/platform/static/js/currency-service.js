/**
 * currency-service.js
 * 
 * Fetches the latest USD to IDR exchange rate using the Frankfurter API.
 * Provides utility methods to format USD amounts to local IDR currency representations.
 */
const CurrencyService = (function () {
  "use strict";

  let usdToIdrRate = null;
  let isFetching = false;
  let fetchPromise = null;

  async function fetchExchangeRate() {
    if (usdToIdrRate !== null) {
      return usdToIdrRate;
    }
    
    if (isFetching) {
      return fetchPromise;
    }

    isFetching = true;
    fetchPromise = fetch("https://api.frankfurter.dev/v1/latest?base=USD&symbols=IDR")
      .then(response => {
        if (!response.ok) throw new Error("Failed to fetch exchange rate");
        return response.json();
      })
      .then(data => {
        usdToIdrRate = data.rates.IDR;
        isFetching = false;
        return usdToIdrRate;
      })
      .catch(err => {
        console.error("Currency conversion error:", err);
        isFetching = false;
        return null;
      });

    return fetchPromise;
  }

  /**
   * Converts a formatted USD string (e.g., "$1,500.00") or a number to formatted IDR string.
   */
  async function convertUsdToIdrFormatted(usdValue) {
    const rate = await fetchExchangeRate();
    if (!rate) return null;

    let numericValue = 0;
    if (typeof usdValue === "string") {
      // Remove $ and commas, then parse
      const cleanString = usdValue.replace(/[\$,]/g, "");
      numericValue = parseFloat(cleanString);
    } else if (typeof usdValue === "number") {
      numericValue = usdValue;
    }

    if (isNaN(numericValue)) return null;

    const idrValue = numericValue * rate;
    
    // Format to IDR: Rp 23.250.000 (No decimals for IDR is standard)
    return new Intl.NumberFormat('id-ID', {
      style: 'currency',
      currency: 'IDR',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0
    }).format(idrValue);
  }

  /**
   * Appends an IDR equivalent subtitle to a container.
   * Target container should have a place to insert or we can append a small span.
   * @param {string} usdString - The USD value (e.g. "$10,000.00")
   * @param {HTMLElement} elementToUpdate - The element to inject the IDR string into (or create next to)
   */
  async function attachIdrSubtitle(usdString, elementId) {
    const el = document.getElementById(elementId);
    if (!el) return;

    const idrString = await convertUsdToIdrFormatted(usdString);
    if (!idrString) return;

    // Look for an existing IDR subtitle block to avoid duplicates
    let subtitleEl = el.parentNode.querySelector('.idr-subtitle');
    if (!subtitleEl) {
      subtitleEl = document.createElement('div');
      subtitleEl.className = 'idr-subtitle';
      subtitleEl.style.fontSize = '0.45em';
      subtitleEl.style.color = '#667085';
      subtitleEl.style.fontWeight = '500';
      subtitleEl.style.marginTop = '4px';
      subtitleEl.style.lineHeight = '1';
      subtitleEl.style.opacity = '0.8';
      el.parentNode.appendChild(subtitleEl);
    }
    subtitleEl.textContent = `~ ${idrString}`;
  }

  return {
    fetchExchangeRate,
    convertUsdToIdrFormatted,
    attachIdrSubtitle
  };
})();

window.CurrencyService = CurrencyService;
