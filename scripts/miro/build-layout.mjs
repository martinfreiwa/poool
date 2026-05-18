// Read screenshot index, compute Miro positions, output layout plan + upload batches.
//
// Layout: 4 section headers (INVESTOR / AUTH / DEVELOPER / PUBLIC), one row per page
// with desktop (1200px wide) left + mobile (400px wide) right. Title text above each row.
//
// Skip: pilot 5 pages (already uploaded) + any failed screenshots.

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, unlinkSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PAGES } from "./pages.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const INDEX_PATH = path.join(REPO_ROOT, "tmp/miro-screenshots/index.json");
const OUT_DIR = path.join(REPO_ROOT, "tmp/miro-upload");

// Already-uploaded pilot pages
const PILOT_IDS = new Set([
  "investor-portfolio", "investor-wallet", "investor-rewards",
  "investor-transactions", "investor-settings",
]);

// Section headers and bucket grouping
const SECTIONS = [
  { bucket: "A", title: "INVESTOR (Bucket A)", color: "#4DABF7" },
  { bucket: "B", title: "AUTH & ONBOARDING (Bucket B)", color: "#FF922B" },
  { bucket: "D", title: "DEVELOPER (Bucket D)", color: "#9775FA" },
  { bucket: "F", title: "PUBLIC / LANDING (Bucket F)", color: "#51CF66" },
];

const Y_START         = 152000;   // below pilot's settings (~151591)
const SECTION_GAP     = 1200;     // gap before each section header
const SECTION_TO_PAGE = 600;      // gap from header to first page in section
const PAGE_GAP        = 500;      // gap between pages
const TITLE_TO_IMG    = 100;      // gap from title text to images
const X_DESKTOP       = -800;
const X_MOBILE        = 200;
const W_DESKTOP       = 1200;
const W_MOBILE        = 400;
const BATCH_SIZE      = 10;

function dispHeight(displayW, pngW, pngH) {
  return displayW * pngH / pngW;
}

function main() {
  if (!existsSync(INDEX_PATH)) throw new Error("index.json missing — run screenshot.mjs first");
  const index = JSON.parse(readFileSync(INDEX_PATH, "utf8"));

  // Merge all runs, take latest result per (id, viewport).
  const byKey = new Map();
  for (const run of index.runs) {
    for (const r of run.results) {
      if (r.status !== "ok") continue;
      byKey.set(`${r.id}|${r.viewport}`, r);
    }
  }

  // Group successful (id -> {desktop, mobile}) by bucket
  const byBucket = {};
  for (const sec of SECTIONS) byBucket[sec.bucket] = [];
  for (const page of PAGES) {
    if (PILOT_IDS.has(page.id)) continue;
    if (!byBucket[page.bucket]) continue; // skip buckets not in SECTIONS (e.g. C, E)
    const d = byKey.get(`${page.id}|desktop`);
    const m = byKey.get(`${page.id}|mobile`);
    if (!d || !m) continue; // need both viewports
    byBucket[page.bucket].push({ page, desktop: d, mobile: m });
  }

  // Compute layout
  const titleDsl = [];
  const imageItems = []; // for upload batches
  let y = Y_START;
  let aliasN = 100;

  for (const sec of SECTIONS) {
    const pages = byBucket[sec.bucket];
    if (!pages.length) continue;

    // Section header
    y += SECTION_GAP;
    const alias = `sec_${sec.bucket}`;
    titleDsl.push(
      `${alias} TEXT x=0 y=${y} w=2200 size=44 align=center font=plex_sans color=#FFFFFF fill=${sec.color} fill_opacity=0.9 "<p><strong>${escTxt(sec.title)} — ${pages.length} Seiten</strong></p>"`
    );
    y += SECTION_TO_PAGE;

    for (const { page, desktop, mobile } of pages) {
      const dH = dispHeight(W_DESKTOP, desktop.png_w, desktop.png_h);
      const mH = dispHeight(W_MOBILE,  mobile.png_w,  mobile.png_h);
      const maxH = Math.max(dH, mH);

      // Title above row
      const tAlias = `t_${aliasN++}`;
      titleDsl.push(
        `${tAlias} TEXT x=-600 y=${Math.round(y - TITLE_TO_IMG)} w=800 size=24 align=left font=plex_sans color=#2d5cff "<p><strong>${escTxt(page.id)}</strong> | ${escTxt(page.route)}</p>"`
      );

      // Desktop image (top-aligned at Y=y)
      imageItems.push({
        page_id: page.id,
        viewport: "desktop",
        file: desktop.file,
        title: `${page.id}-desktop`,
        width: W_DESKTOP,
        x: X_DESKTOP,
        y: Math.round(y + dH / 2),
      });
      // Mobile image
      imageItems.push({
        page_id: page.id,
        viewport: "mobile",
        file: mobile.file,
        title: `${page.id}-mobile`,
        width: W_MOBILE,
        x: X_MOBILE,
        y: Math.round(y + mH / 2),
      });

      y += maxH + PAGE_GAP;
    }
  }

  // Clean OUT_DIR
  if (existsSync(OUT_DIR)) {
    for (const f of readdirSync(OUT_DIR)) unlinkSync(path.join(OUT_DIR, f));
  } else {
    mkdirSync(OUT_DIR, { recursive: true });
  }

  // Write titles DSL
  writeFileSync(path.join(OUT_DIR, "titles.dsl"), titleDsl.join("\n") + "\n");

  // Write upload batches
  for (let i = 0; i < imageItems.length; i += BATCH_SIZE) {
    const batch = imageItems.slice(i, i + BATCH_SIZE);
    const idx = Math.floor(i / BATCH_SIZE);
    writeFileSync(path.join(OUT_DIR, `batch-${String(idx).padStart(3, "0")}.json`), JSON.stringify(batch, null, 2));
  }

  // Summary
  const summary = {
    sections: SECTIONS.map(s => ({ bucket: s.bucket, title: s.title, page_count: byBucket[s.bucket].length })),
    total_pages: Object.values(byBucket).reduce((a, b) => a + b.length, 0),
    total_images: imageItems.length,
    batches: Math.ceil(imageItems.length / BATCH_SIZE),
    y_span: y - Y_START,
    titles_dsl: "tmp/miro-upload/titles.dsl",
    batches_dir: "tmp/miro-upload/",
  };
  writeFileSync(path.join(OUT_DIR, "summary.json"), JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));
}

function escTxt(s) {
  return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

main();
