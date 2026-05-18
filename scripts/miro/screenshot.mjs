// Playwright screenshot runner for Miro UX-review pipeline.
//
// Usage:
//   node scripts/miro/screenshot.mjs --bucket A
//   node scripts/miro/screenshot.mjs --ids investor-portfolio,investor-wallet
//   node scripts/miro/screenshot.mjs --all
//
// Env (.env at repo root):
//   POOOL_BASE_URL=http://localhost:8888
//   POOOL_USER=support@traffic-creator.com
//   POOOL_PASS=...
//   POOOL_HEADLESS=1     (set 0 to watch the browser)
//   POOOL_CONCURRENCY=3
//
// Output: tmp/miro-screenshots/<role>/<id>-<viewport>.png + tmp/miro-screenshots/index.json

import { chromium } from "playwright";
import { mkdir, writeFile, readFile, stat } from "node:fs/promises";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import "dotenv/config";

import { PAGES, VIEWPORTS } from "./pages.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const OUT_DIR = path.join(REPO_ROOT, "tmp/miro-screenshots");
const STATE_PATH = path.join(OUT_DIR, "_session.json");
const INDEX_PATH = path.join(OUT_DIR, "index.json");

const BASE_URL = process.env.POOOL_BASE_URL || "http://localhost:8888";
const USER = process.env.POOOL_USER;
const PASS = process.env.POOOL_PASS;
const HEADLESS = process.env.POOOL_HEADLESS !== "0";
const CONCURRENCY = parseInt(process.env.POOOL_CONCURRENCY || "3", 10);
const NAV_TIMEOUT_MS = 30_000;
const SETTLE_MS = 1500;

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { bucket: null, buckets: null, ids: null, all: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--bucket") out.bucket = args[++i];
    else if (args[i] === "--buckets") out.buckets = args[++i].split(",").map(s => s.trim());
    else if (args[i] === "--ids") out.ids = args[++i].split(",").map(s => s.trim());
    else if (args[i] === "--all") out.all = true;
  }
  return out;
}

function pickPages({ bucket, buckets, ids, all }) {
  if (all) return PAGES;
  if (ids) return PAGES.filter(p => ids.includes(p.id));
  if (buckets) return PAGES.filter(p => buckets.includes(p.bucket));
  if (bucket) return PAGES.filter(p => p.bucket === bucket);
  throw new Error("Pass --bucket <X>, --buckets A,B, --ids id1,id2, or --all");
}

function pngDims(filePath) {
  const b = readFileSync(filePath);
  return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
}

async function ensureLogin(browser) {
  if (existsSync(STATE_PATH)) {
    try {
      const stats = await stat(STATE_PATH);
      const ageMin = (Date.now() - stats.mtimeMs) / 60_000;
      if (ageMin < 240) {
        console.log(`[auth] reusing session (${ageMin.toFixed(0)}min old)`);
        return STATE_PATH;
      }
      console.log(`[auth] session stale (${ageMin.toFixed(0)}min), re-login`);
    } catch {}
  }
  if (!USER || !PASS) {
    throw new Error("POOOL_USER / POOOL_PASS missing in .env");
  }
  console.log(`[auth] logging in as ${USER}`);
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  await page.goto(`${BASE_URL}/auth/login`, { waitUntil: "networkidle", timeout: NAV_TIMEOUT_MS });
  await page.fill("#email-input", USER);
  await page.fill("#password-input", PASS);
  await Promise.all([
    page.waitForLoadState("networkidle", { timeout: NAV_TIMEOUT_MS }).catch(() => {}),
    page.click('#login-form button[type="submit"], #login-form button:not([type])').catch(async () => {
      await page.press("#password-input", "Enter");
    }),
  ]);
  await page.waitForTimeout(SETTLE_MS);
  const url = page.url();
  if (url.includes("/auth/login")) {
    const err = await page.locator("#auth-error").textContent().catch(() => "");
    throw new Error(`Login failed (still on /auth/login). Error text: "${err}"`);
  }
  await ctx.storageState({ path: STATE_PATH });
  await ctx.close();
  console.log(`[auth] session saved → ${path.relative(REPO_ROOT, STATE_PATH)}`);
  return STATE_PATH;
}

async function dismissOverlays(page) {
  // Cookie banners + onboarding modals — best-effort, ignore failures
  const selectors = [
    'button:has-text("Accept all")',
    'button:has-text("Accept")',
    'button:has-text("Got it")',
    'button:has-text("Skip")',
    'button:has-text("Close")',
    '[data-dismiss="modal"]',
    '.cookie-banner button',
  ];
  for (const sel of selectors) {
    try {
      const el = page.locator(sel).first();
      if (await el.isVisible({ timeout: 200 })) {
        await el.click({ timeout: 500 });
        await page.waitForTimeout(200);
      }
    } catch {}
  }
}

async function shootPage(ctx, pageMeta, viewport) {
  const page = await ctx.newPage();
  const result = {
    id: pageMeta.id, role: pageMeta.role, route: pageMeta.route,
    viewport: viewport.name, status: "pending", file: null, error: null, http_status: null,
  };
  try {
    if (viewport.userAgent) {
      await page.setExtraHTTPHeaders({ "User-Agent": viewport.userAgent });
    }
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    const url = `${BASE_URL}${pageMeta.route}`;
    const resp = await page.goto(url, { waitUntil: "domcontentloaded", timeout: NAV_TIMEOUT_MS });
    result.http_status = resp?.status() ?? null;
    if (resp && resp.status() >= 400 && resp.status() !== 401) {
      result.status = "http_error";
      result.error = `HTTP ${resp.status()}`;
      await page.close();
      return result;
    }
    // Detect auth bounce for pages we expected to be public
    if (page.url().includes("/auth/login") && !pageMeta.requires_auth) {
      result.status = "auth_redirect";
      result.error = "Expected public but redirected to /auth/login";
    }
    await page.waitForLoadState("networkidle", { timeout: 10_000 }).catch(() => {});
    await dismissOverlays(page);
    await page.waitForTimeout(SETTLE_MS);

    const roleDir = path.join(OUT_DIR, pageMeta.role);
    await mkdir(roleDir, { recursive: true });
    const file = path.join(roleDir, `${pageMeta.id}-${viewport.name}.png`);
    await page.screenshot({ path: file, fullPage: true, type: "png" });
    result.file = path.relative(REPO_ROOT, file);
    const dims = pngDims(file);
    result.png_w = dims.width;
    result.png_h = dims.height;
    if (result.status === "pending") result.status = "ok";
    console.log(`  [${viewport.name}] ${pageMeta.id} → ${result.status} (HTTP ${result.http_status}, ${dims.width}x${dims.height})`);
  } catch (e) {
    result.status = "error";
    result.error = e.message;
    console.error(`  [${viewport.name}] ${pageMeta.id} → ERROR: ${e.message}`);
  } finally {
    await page.close();
  }
  return result;
}

async function runBatch(items, fn, concurrency) {
  const results = [];
  const queue = [...items];
  const workers = Array.from({ length: concurrency }, async () => {
    while (queue.length) {
      const item = queue.shift();
      results.push(await fn(item));
    }
  });
  await Promise.all(workers);
  return results;
}

async function main() {
  const args = parseArgs();
  const pages = pickPages(args);
  console.log(`[plan] ${pages.length} pages × ${VIEWPORTS.length} viewports = ${pages.length * VIEWPORTS.length} screenshots`);
  console.log(`[plan] base=${BASE_URL} headless=${HEADLESS} concurrency=${CONCURRENCY}`);

  await mkdir(OUT_DIR, { recursive: true });

  const browser = await chromium.launch({ headless: HEADLESS });
  try {
    await ensureLogin(browser);

    const ctx = await browser.newContext({ storageState: STATE_PATH });
    const tasks = [];
    for (const p of pages) {
      for (const v of VIEWPORTS) tasks.push({ p, v });
    }
    const results = await runBatch(tasks, ({ p, v }) => shootPage(ctx, p, v), CONCURRENCY);
    await ctx.close();

    const prior = existsSync(INDEX_PATH) ? JSON.parse(await readFile(INDEX_PATH, "utf8")) : { runs: [] };
    prior.runs.push({
      ts: new Date().toISOString(),
      args,
      base_url: BASE_URL,
      results,
    });
    await writeFile(INDEX_PATH, JSON.stringify(prior, null, 2));

    const ok = results.filter(r => r.status === "ok").length;
    const fail = results.length - ok;
    console.log(`\n[done] ok=${ok} fail=${fail} → ${path.relative(REPO_ROOT, INDEX_PATH)}`);
  } finally {
    await browser.close();
  }
}

main().catch(e => { console.error(e); process.exit(1); });
