const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const PLATFORM_DIR = path.join(__dirname, 'platform.poool.app');
const BASE_URL = 'https://platform.poool.app';
const COOKIE = 'auth_session=authenticated; session_id=session_dguvurawipxe';

function findHtmlFiles(dir, fileList = []) {
  if (!fs.existsSync(dir)) return fileList;
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const fullPath = path.join(dir, file);
    if (fs.statSync(fullPath).isDirectory()) {
      findHtmlFiles(fullPath, fileList);
    } else if (fullPath.endsWith('.html')) {
      fileList.push(fullPath);
    }
  }
  return fileList;
}

const htmlFiles = findHtmlFiles(PLATFORM_DIR);
const assetPaths = new Set();
const regexes = [
  /src=["'](\/images\/[^"']+)["']/g,
  /src=["'](\/static\/[^"']+)["']/g,
  /href=["'](\/static\/[^"']+)["']/g,
  /href=["'](\/images\/[^"']+)["']/g,
  /url\(['"]?(\/static\/[^'"\)]+)['"]?\)/g,
  /url\(['"]?(\/images\/[^'"\)]+)['"]?\)/g
];

console.log(`Scanning ${htmlFiles.length} HTML files...`);
for (const file of htmlFiles) {
  const content = fs.readFileSync(file, 'utf8');
  for (const regex of regexes) {
    let match;
    while ((match = regex.exec(content)) !== null) {
      try {
        const pathObj = new URL(match[1], 'http://localhost');
        assetPaths.add(pathObj.pathname);
      } catch(e) {}
    }
  }
}

let dlCount = 0;
for (const assetPath of assetPaths) {
  const localFilePath = path.join(PLATFORM_DIR, decodeURI(assetPath).substring(1));
  if (fs.existsSync(localFilePath)) continue;
  
  const dir = path.dirname(localFilePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  const url = `${BASE_URL}${assetPath.split(' ').join('%20')}`;
  console.log(`Downloading: ${assetPath}`);
  
  try {
     execSync(`curl -s -L -H "Cookie: ${COOKIE}" "${url}" -o "${localFilePath}"`);
     dlCount++;
  } catch (err) {
     console.log(`Failed: ${assetPath}`);
  }
}
console.log(`Successfully downloaded ${dlCount} missing assets!`);
