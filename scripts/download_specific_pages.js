const https = require('https');
const fs = require('fs');
const path = require('path');

const COOKIE = 'auth_session=authenticated; session_id=session_dguvurawipxe';
const BASE_DIR = '/Users/martin/Downloads/poool';

function downloadFile(url, destPath, options = {}) {
    return new Promise((resolve, reject) => {
        fs.mkdirSync(path.dirname(destPath), { recursive: true });

        https.get(url, options, (res) => {
            if (res.statusCode === 301 || res.statusCode === 302 || res.statusCode === 308) {
                const redirectUrl = res.headers.location.startsWith('http') ? res.headers.location : new URL(res.headers.location, url).toString();
                https.get(redirectUrl, options, handleResponse);
            } else {
                handleResponse(res);
            }

            function handleResponse(response) {
                if (response.statusCode >= 400) {
                    return resolve(false);
                }
                const fileStream = fs.createWriteStream(destPath);
                response.pipe(fileStream);
                fileStream.on('finish', () => {
                    fileStream.close();
                    const stats = fs.statSync(destPath);
                    console.log(`[OK] ${url} -> ${stats.size} bytes`);
                    resolve(true);
                });
                fileStream.on('error', (err) => {
                    fs.unlink(destPath, () => { });
                    resolve(false);
                });
            }
        }).on('error', (err) => resolve(false));
    });
}

function extractAssets(html) {
    const assets = new Set();
    // Find standard asset links (src, href)
    const regex = /(?:src|href)="(\/(?:static|images|assets)[^"]+)"/g;
    let match;
    while ((match = regex.exec(html)) !== null) {
        assets.add(match[1]);
    }
    return Array.from(assets);
}

async function main() {
    // 1. Download www.poool.app/id/
    console.log('Downloading www.poool.app/id/ (Indonesian version)...');
    await downloadFile('https://www.poool.app/id/', path.join(BASE_DIR, 'www.poool.app', 'id', 'index.html'));

    // 2. Download Platform Pages
    const platformPages = [
        { url: '/marketplace', file: 'marketplace.html' },
        { url: '/commodities-marketplace', file: 'commodities-marketplace.html' },
        { url: '/wallet', file: 'wallet.html' },
        { url: '/portfolio', file: 'portfolio.html' },
        { url: '/cart', file: 'cart.html' },
        { url: '/support', file: 'support.html' },
        { url: '/', file: 'index.html' },
        { url: '/settings', file: 'settings.html' },
        { url: '/kyc', file: 'kyc.html' },
        { url: '/developer/dashboard', file: 'developer/dashboard.html' },
        { url: '/developer/assets', file: 'developer/assets.html' },
        { url: '/developer/add-asset', file: 'developer/add-asset.html' },
    ];

    console.log('\nDownloading platform.poool.app pages...');
    let allAssets = new Set();

    for (const page of platformPages) {
        const destPath = path.join(BASE_DIR, 'platform.poool.app', page.file);
        const success = await downloadFile(`https://platform.poool.app${page.url}`, destPath, { headers: { Cookie: COOKIE } });

        if (success) {
            const html = fs.readFileSync(destPath, 'utf8');
            const assets = extractAssets(html);
            assets.forEach(a => allAssets.add(a));
        }
    }

    // 3. Download related assets from Platform
    console.log(`\nFound ${allAssets.size} unique assets on platform pages. Downloading missing ones...`);

    // Extra specific assets with spaces that might fail default extraction
    allAssets.add('/images/Logo Pool.svg');
    allAssets.add('/images/Logo premium.svg');
    allAssets.add('/images/Featured icon.png');
    allAssets.add('/static/images/message-chat-circle grey.svg');
    ['Assets', 'Dashboard', 'Notifications', 'Ranking', 'Settings', 'Support'].forEach(f => {
        allAssets.add(`/static/images/Menu developer/${f}.svg`);
    });

    for (const asset of allAssets) {
        const decodedUrl = encodeURI(asset).replace(/%25/g, '%');
        const destPath = path.join(BASE_DIR, 'platform.poool.app', decodeURIComponent(asset));

        if (!fs.existsSync(destPath) || fs.statSync(destPath).size === 0) {
            await downloadFile(`https://platform.poool.app${decodedUrl}`, destPath, { headers: { Cookie: COOKIE } });
        }
    }

    console.log('\nDownloads finished!');
}

main();
