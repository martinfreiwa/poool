const https = require('https');
const fs = require('fs');
const path = require('path');

const COOKIE = 'auth_session=authenticated; session_id=session_dguvurawipxe';
const DEST = '/Users/martin/Downloads/poool/platform.poool.app';

function download(urlPath) {
    return new Promise((resolve) => {
        const destPath = path.join(DEST, urlPath);
        fs.mkdirSync(path.dirname(destPath), { recursive: true });

        const url = `https://platform.poool.app${encodeURI(urlPath).replace(/%25/g, '%')}`;

        https.get(url, { headers: { Cookie: COOKIE } }, (res) => {
            if (res.statusCode === 301 || res.statusCode === 302) {
                const loc = res.headers.location;
                https.get(loc.startsWith('http') ? loc : `https://platform.poool.app${loc}`,
                    { headers: { Cookie: COOKIE } }, (res2) => {
                        const chunks = [];
                        res2.on('data', c => chunks.push(c));
                        res2.on('end', () => {
                            const buf = Buffer.concat(chunks);
                            fs.writeFileSync(destPath, buf);
                            console.log(`${urlPath} -> ${buf.length} bytes`);
                            resolve();
                        });
                    });
                return;
            }
            const chunks = [];
            res.on('data', c => chunks.push(c));
            res.on('end', () => {
                const buf = Buffer.concat(chunks);
                fs.writeFileSync(destPath, buf);
                console.log(`${urlPath} -> ${buf.length} bytes`);
                resolve();
            });
        }).on('error', (e) => {
            console.log(`FAILED: ${urlPath} - ${e.message}`);
            resolve();
        });
    });
}

async function main() {
    const files = [
        '/images/Logo Pool.svg',
        '/images/Logo premium.svg',
        '/images/Featured icon.png',
        '/static/images/message-chat-circle grey.svg',
        '/static/images/Menu developer/Assets.svg',
        '/static/images/Menu developer/Dashboard.svg',
        '/static/images/Menu developer/Notifications.svg',
        '/static/images/Menu developer/Ranking.svg',
        '/static/images/Menu developer/Settings.svg',
        '/static/images/Menu developer/Support.svg',
    ];

    for (const f of files) {
        await download(f);
    }

    // Also get any additional assets referenced in all pages
    const pages = ['marketplace', 'wallet', 'portfolio', 'cart', 'support', 'settings', 'kyc', 'commodities-marketplace'];
    const allAssets = new Set();

    for (const page of pages) {
        const html = fs.readFileSync(path.join(DEST, `${page}.html`), 'utf8');
        // Find all /static/ and /images/ references
        const matches = html.match(/(?:src|href)="(\/(?:static|images)\/[^"]+)"/g) || [];
        for (const m of matches) {
            const asset = m.match(/"([^"]+)"/)[1];
            allAssets.add(asset);
        }
    }

    console.log(`\nFound ${allAssets.size} unique assets across all pages`);

    for (const asset of allAssets) {
        const destPath = path.join(DEST, asset);
        if (!fs.existsSync(destPath) || fs.statSync(destPath).size === 0) {
            await download(asset);
        }
    }

    console.log('\nDone!');
}

main();
