const http = require("http");
const fs = require("fs");
const path = require("path");
const querystring = require("querystring");

const PORT = 8888;
const BASE = path.dirname(__dirname);
const WWW_ROOT = path.join(BASE, "www");
const PLATFORM_ROOT = path.join(BASE, "platform");

const MIME_TYPES = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "application/javascript",
  ".mjs": "application/javascript",
  ".json": "application/json",
  ".webp": "image/webp",
  ".webp": "image/webp",
  ".webp": "image/webp",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".eot": "application/vnd.ms-fontobject",
  ".otf": "font/otf",
  ".mp4": "video/mp4",
  ".webm": "video/webm",
};

function tryFiles(root, urlPath) {
  let filePath = path.join(root, urlPath);
  if (fs.existsSync(filePath) && fs.statSync(filePath).isFile())
    return filePath;
  let htmlPath = filePath + ".html";
  if (fs.existsSync(htmlPath) && fs.statSync(htmlPath).isFile())
    return htmlPath;
  if (filePath.includes(".eot") && fs.existsSync(filePath + "@"))
    return filePath + "@";
  return null;
}

function findFile(urlPath) {
  let result = tryFiles(WWW_ROOT, urlPath);
  if (result) return result;
  result = tryFiles(path.join(WWW_ROOT, "en"), urlPath);
  if (result) return result;
  result = tryFiles(path.join(WWW_ROOT, "id"), urlPath);
  if (result) return result;
  result = tryFiles(PLATFORM_ROOT, urlPath);
  if (result) return result;
  if (urlPath.startsWith("/en")) {
    let indexPath = path.join(WWW_ROOT, "en", "index.html");
    if (fs.existsSync(indexPath)) return indexPath;
  }
  if (urlPath.startsWith("/id")) {
    let indexPath = path.join(WWW_ROOT, "id", "index.html");
    if (fs.existsSync(indexPath)) return indexPath;
  }
  return null;
}

function serveFile(res, filePath) {
  const ext = path.extname(filePath).replace("@", "");
  const contentType = MIME_TYPES[ext] || "application/octet-stream";
  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(500);
      res.end("Error");
      return;
    }
    res.writeHead(200, {
      "Content-Type": contentType,
      "Access-Control-Allow-Origin": "*",
      "Cache-Control": "no-cache",
    });
    res.end(data);
  });
}

function redirectToPlatform(res, targetPath) {
  res.writeHead(303, {
    Location: `https://platform.poool.app${targetPath}`,
    "Cache-Control": "no-cache",
  });
  res.end();
}

// Collect POST body
function getBody(req) {
  return new Promise((resolve) => {
    let body = [];
    req.on("data", (c) => body.push(c));
    req.on("end", () => resolve(Buffer.concat(body).toString()));
  });
}

const server = http.createServer(async (req, res) => {
  let urlPath = decodeURIComponent(req.url.split("?")[0]);
  const query = req.url.includes("?") ? `?${req.url.split("?").slice(1).join("?")}` : "";

  const platformRedirects = new Map([
    ["/auth/login", "/auth/login"],
    ["/auth/signup", "/auth/signup"],
    ["/signup", "/auth/signup"],
    ["/marketplace", "/marketplace"],
    ["/blog", "/blog"],
    ["/terms", "/terms"],
    ["/terms-and-conditions", "/terms"],
    ["/cookies", "/cookies"],
    ["/privacy-policy", "/privacy-policy"],
    ["/privacy", "/privacy-policy"],
    ["/currency-policy", "/currency-policy"],
    ["/currency", "/currency-policy"],
    ["/aml-kyc-policy", "/aml-kyc-policy"],
    ["/imprint", "/imprint"],
    ["/gdpr-data-request", "/gdpr-data-request"],
  ]);

  if (req.method === "GET" || req.method === "HEAD") {
    const target = platformRedirects.get(urlPath);
    if (target) {
      redirectToPlatform(res, `${target}${query}`);
      return;
    }

    if (urlPath.startsWith("/p/")) {
      redirectToPlatform(res, `${urlPath}${query}`);
      return;
    }
  }

  // ============= LOCAL AUTH (mock) =============
  if (urlPath === "/auth/login" && req.method === "POST") {
    await getBody(req);
    // Set a session cookie and redirect
    res.writeHead(200, {
      "Content-Type": "text/html",
      "Set-Cookie": "poool_session=local_mock_session; Path=/; HttpOnly",
      "HX-Redirect": "/platform/marketplace",
    });
    // HTMX will follow HX-Redirect header. Also send fallback HTML.
    res.end('<script>window.location.href="/platform/marketplace";</script>');
    return;
  }

  if (urlPath === "/auth/signup" && req.method === "POST") {
    await getBody(req);
    res.writeHead(200, {
      "Content-Type": "text/html",
      "Set-Cookie": "poool_session=local_mock_session; Path=/; HttpOnly",
      "HX-Redirect": "/platform/marketplace",
    });
    res.end('<script>window.location.href="/platform/marketplace";</script>');
    return;
  }

  // ============= ROOT REDIRECT =============
  if (urlPath === "/") {
    urlPath = "/en/index.html";
  }

  // ============= PLATFORM PAGES =============
  if (urlPath.startsWith("/platform/") || urlPath === "/platform") {
    let platformPath = urlPath.replace("/platform", "") || "/";
    if (platformPath === "/") platformPath = "/marketplace.html";
    if (platformPath.endsWith("/")) platformPath += "index.html";

    // Serve static assets from platform
    let resolved = tryFiles(PLATFORM_ROOT, platformPath);
    if (!resolved) resolved = tryFiles(PLATFORM_ROOT, platformPath + ".html");
    if (!resolved) resolved = tryFiles(PLATFORM_ROOT, "/login.html");

    if (resolved) {
      serveFile(res, resolved);
      return;
    }
  }

  // ============= MAIN SITE =============
  if (urlPath.endsWith("/")) urlPath += "index.html";

  const resolved = findFile(urlPath);
  if (!resolved) {
    res.writeHead(404);
    res.end("Not found");
    return;
  }

  serveFile(res, resolved);
});

server.listen(PORT, "0.0.0.0", () => {});
