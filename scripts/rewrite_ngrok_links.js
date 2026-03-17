const fs = require('fs');
const path = require('path');

const BASE_DIR = '/Users/martin/Downloads/poool';
const WWW_EN_DIR = path.join(BASE_DIR, 'www.poool.app', 'en');
const WWW_ID_DIR = path.join(BASE_DIR, 'www.poool.app', 'id');
const PLATFORM_DIR = path.join(BASE_DIR, 'platform.poool.app');

function walkDir(dir, callback) {
    if (!fs.existsSync(dir)) return;
    fs.readdirSync(dir).forEach(f => {
        let dirPath = path.join(dir, f);
        let isDirectory = fs.statSync(dirPath).isDirectory();
        isDirectory ? walkDir(dirPath, callback) : callback(dirPath);
    });
}

let modifiedFiles = 0;

function processFile(filePath) {
    const ext = path.extname(filePath);
    // Only process HTML, JS, CSS
    if (!['.html', '.js', '.css'].includes(ext)) return;

    let content = fs.readFileSync(filePath, 'utf8');
    let originalContent = content;

    // Replace absolute URLs with relative paths from the root
    // https://platform.poool.app/something -> /platform/something
    content = content.replace(/https:\/\/platform\.poool\.app\//g, '/platform/');
    content = content.replace(/https:\/\/platform\.poool\.app/g, '/platform');

    // https://www.poool.app/something -> /something
    content = content.replace(/https:\/\/www\.poool\.app\//g, '/');
    content = content.replace(/https:\/\/www\.poool\.app/g, '/');

    // Specific fix for HTMX auth routes that might be relative already
    // We want to make sure hx-post="/auth/login" works (handled by our server proxy)
    // It's already fine.

    if (content !== originalContent) {
        fs.writeFileSync(filePath, content, 'utf8');
        console.log(`Updated links in: ${filePath.replace(BASE_DIR, '')}`);
        modifiedFiles++;
    }
}

console.log('Replacing absolute domain links with relative paths...');

walkDir(WWW_EN_DIR, processFile);
walkDir(WWW_ID_DIR, processFile);
walkDir(PLATFORM_DIR, processFile);

console.log(`\nDone! Replaced links in ${modifiedFiles} files.`);
console.log('All links will now automatically use your ngrok domain.');
