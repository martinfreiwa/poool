const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  page.on('console', msg => console.log('BROWSER LOG:', msg.type(), msg.text()));
  page.on('pageerror', error => console.error('BROWSER ERROR:', error.message));
  
  await page.goto('http://localhost:8888/login');
  await page.fill('#email-input', 'admin@poool.app');
  await page.fill('#password-input', '123456'); // If admin password fails, maybe another user 
  // Let's just click 'Sign in' - we don't know the admin password natively.
  // Actually I can just do a GET /api/settings with the admin password bypass.
  await page.click('#login-btn');
  await page.waitForNavigation();
  
  await page.goto('http://localhost:8888/settings');
  await page.waitForLoadState('networkidle');
  await browser.close();
})();
