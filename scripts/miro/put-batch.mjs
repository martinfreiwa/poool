// PUT image bytes to Miro S3 upload URLs in parallel.
// Reads batch-NNN-urls.json with [{file, url, token}, ...] entries.
//
// Usage: node scripts/miro/put-batch.mjs <urls-json-path>

import { readFileSync } from "node:fs";
import { request } from "node:https";
import path from "node:path";

const urlsPath = process.argv[2];
if (!urlsPath) throw new Error("Pass urls JSON path");

const items = JSON.parse(readFileSync(urlsPath, "utf8"));

function put(item) {
  return new Promise((resolve, reject) => {
    const body = readFileSync(item.file);
    const u = new URL(item.url);
    const req = request({
      method: "PUT",
      hostname: u.hostname,
      path: u.pathname + u.search,
      headers: { "Content-Type": "image/png", "Content-Length": body.length },
    }, (res) => {
      const chunks = [];
      res.on("data", c => chunks.push(c));
      res.on("end", () => resolve({ file: path.basename(item.file), status: res.statusCode, size: body.length }));
    });
    req.on("error", reject);
    req.write(body);
    req.end();
  });
}

const results = await Promise.all(items.map(put));
for (const r of results) console.log(`${r.status} ${(r.size/1024).toFixed(0)}KB  ${r.file}`);
const fail = results.filter(r => r.status >= 400).length;
console.log(`ok=${results.length - fail} fail=${fail}`);
if (fail > 0) process.exit(1);
