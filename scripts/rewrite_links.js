const fs = require('fs');
const path = require('path');

const ROOT_DIR = '/Users/martin/Downloads/poool';

function processDirectory(dir) {
    const files = fs.readdirSync(dir);

    for (const file of files) {
        const fullPath = path.join(dir, file);
        if (fs.statSync(fullPath).isDirectory()) {
            processDirectory(fullPath);
        } else {
            const ext = path.extname(fullPath).toLowerCase();
            // Only process text files
            if (['.html', '.js', '.css', '.json'].includes(ext)) {
                let content = fs.readFileSync(fullPath, 'utf8');
                let original = content;

                // Replace platform links with local /platform/ route
                content = content.replace(/https:\/\/platform\.poool\.app\//g, '/platform/');
                content = content.replace(/https:\/\/platform\.poool\.app/g, '/platform');

                // Replace www links with local /en/ route (or relative)
                content = content.replace(/https:\/\/www\.poool\.app\//g, '/');
                content = content.replace(/https:\/\/www\.poool\.app/g, '');

                if (content !== original) {
                    fs.writeFileSync(fullPath, content, 'utf8');
                    console.log(`Updated links in: ${fullPath}`);
                }
            }
        }
    }
}

console.log('Starting link rewrite...');
processDirectory(path.join(ROOT_DIR, 'www.poool.app'));
processDirectory(path.join(ROOT_DIR, 'platform.poool.app'));
console.log('Finished link rewrite!');
