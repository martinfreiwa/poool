(function () {
    'use strict';

    const LOGO_URL = '/static/images/logos/Logo%20Pool.svg';
    const ASSETS = {
        banner: {
            filename: 'poool-linkedin-twitter-banner.png',
            width: 1200,
            height: 628,
            title: 'POOOL',
            subtitle: 'Co-ownership made simple'
        },
        instagram: {
            filename: 'poool-instagram-post.png',
            width: 1080,
            height: 1080,
            title: 'POOOL',
            subtitle: 'Fractional ownership for premium assets'
        },
        story: {
            filename: 'poool-story.png',
            width: 1080,
            height: 1920,
            title: 'POOOL',
            subtitle: 'Own together. Grow together.'
        }
    };

    function setStatus(message, tone) {
        const el = document.getElementById('affiliate-material-upload-status');
        if (!el) return;
        el.textContent = message || '';
        el.style.color = tone === 'error' ? '#B42318' : tone === 'success' ? '#027A48' : 'var(--text-secondary)';
    }

    function downloadBlob(blob, filename) {
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        link.remove();
        window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    }

    function loadImage(src) {
        return new Promise((resolve, reject) => {
            const img = new Image();
            img.onload = () => resolve(img);
            img.onerror = () => reject(new Error('Unable to load brand logo'));
            img.src = src;
        });
    }

    async function buildPngAsset(asset) {
        const logo = await loadImage(LOGO_URL);
        const canvas = document.createElement('canvas');
        canvas.width = asset.width;
        canvas.height = asset.height;
        const ctx = canvas.getContext('2d');

        ctx.fillStyle = '#F7F8FF';
        ctx.fillRect(0, 0, asset.width, asset.height);
        ctx.fillStyle = '#0000FF';
        ctx.fillRect(0, 0, asset.width, Math.max(10, Math.round(asset.height * 0.018)));

        const logoWidth = Math.round(asset.width * 0.28);
        const logoHeight = Math.round((logo.height / logo.width) * logoWidth);
        ctx.drawImage(logo, (asset.width - logoWidth) / 2, Math.round(asset.height * 0.26), logoWidth, logoHeight);

        ctx.fillStyle = '#101828';
        ctx.textAlign = 'center';
        ctx.font = `${Math.round(asset.width * 0.062)}px "TT Norms Pro", Arial, sans-serif`;
        ctx.fillText(asset.title, asset.width / 2, Math.round(asset.height * 0.56));

        ctx.fillStyle = '#475467';
        ctx.font = `${Math.round(asset.width * 0.026)}px "TT Norms Pro", Arial, sans-serif`;
        ctx.fillText(asset.subtitle, asset.width / 2, Math.round(asset.height * 0.63));

        return new Promise((resolve) => {
            canvas.toBlob((blob) => resolve(blob), 'image/png');
        });
    }

    async function downloadMaterial(kind) {
        const asset = ASSETS[kind];
        if (!asset) return;
        const buttons = Array.from(document.querySelectorAll(`[data-material-download="${kind}"]`));
        buttons.forEach((button) => {
            button.disabled = true;
            button.dataset.originalText = button.textContent;
            button.textContent = 'Preparing...';
        });
        try {
            const blob = await buildPngAsset(asset);
            if (!blob) throw new Error('Unable to generate material');
            downloadBlob(blob, asset.filename);
        } catch (err) {
            console.error('Affiliate material download failed:', err);
            setStatus('Could not prepare the download. Please try again.', 'error');
        } finally {
            buttons.forEach((button) => {
                button.disabled = false;
                button.textContent = button.dataset.originalText || 'Download';
                delete button.dataset.originalText;
            });
        }
    }

    async function downloadAllMaterials() {
        const button = document.querySelector('[data-download-all]');
        if (button) {
            button.disabled = true;
            button.dataset.originalText = button.textContent;
            button.textContent = 'Preparing...';
        }
        try {
            for (const kind of Object.keys(ASSETS)) {
                await downloadMaterial(kind);
            }
        } finally {
            if (button) {
                button.disabled = false;
                button.textContent = button.dataset.originalText || 'Download All';
                delete button.dataset.originalText;
            }
        }
    }

    function materialStatusLabel(status) {
        switch (status) {
            case 'approved':
                return 'Approved';
            case 'rejected':
                return 'Rejected';
            case 'pending_review':
                return 'Pending review';
            default:
                return 'Unknown';
        }
    }

    function appendCell(row, value) {
        const cell = document.createElement('td');
        cell.style.padding = '10px';
        cell.textContent = value || '-';
        row.appendChild(cell);
    }

    function renderMaterials(materials) {
        const body = document.getElementById('affiliate-materials-status-body');
        if (!body) return;
        body.replaceChildren();

        if (!Array.isArray(materials) || materials.length === 0) {
            const row = document.createElement('tr');
            const cell = document.createElement('td');
            cell.colSpan = 4;
            cell.style.padding = '18px';
            cell.style.color = 'var(--text-muted)';
            cell.textContent = 'No custom materials submitted yet.';
            row.appendChild(cell);
            body.appendChild(row);
            return;
        }

        materials.forEach((material) => {
            const row = document.createElement('tr');
            row.style.borderTop = '1px solid var(--border-color)';
            appendCell(row, material.asset_name);
            appendCell(row, materialStatusLabel(material.status));
            appendCell(row, material.created_at ? new Date(material.created_at).toLocaleDateString() : '-');
            appendCell(row, material.review_note || '-');
            body.appendChild(row);
        });
    }

    async function loadMaterials() {
        const body = document.getElementById('affiliate-materials-status-body');
        if (!body) return;
        try {
            const res = await fetch('/api/affiliate/materials', { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Failed to load materials');
            const data = await res.json();
            renderMaterials(data.materials);
        } catch (err) {
            console.error('Affiliate material status load failed:', err);
            renderMaterials([]);
            setStatus('Could not load material review status.', 'error');
        }
    }

    async function submitMaterialUpload(event) {
        event.preventDefault();
        const form = event.currentTarget;
        const fileInput = document.getElementById('material-file');
        const submit = document.getElementById('affiliate-material-upload-submit');
        const file = fileInput && fileInput.files ? fileInput.files[0] : null;

        if (!file) {
            setStatus('Choose a file before submitting.', 'error');
            return;
        }

        if (file.size > 20 * 1024 * 1024) {
            setStatus('File must be 20 MB or smaller.', 'error');
            return;
        }

        if (submit) {
            submit.disabled = true;
            submit.dataset.originalText = submit.textContent;
            submit.textContent = 'Submitting...';
        }
        setStatus('Submitting for review...', 'neutral');

        try {
            const res = await fetch('/api/affiliate/materials/upload', {
                method: 'POST',
                body: new FormData(form),
                credentials: 'same-origin'
            });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) {
                throw new Error(data.error || 'Upload failed');
            }

            form.reset();
            setStatus('Material submitted for review.', 'success');
            await loadMaterials();
        } catch (err) {
            setStatus(err.message || 'Upload failed. Please try again.', 'error');
        } finally {
            if (submit) {
                submit.disabled = false;
                submit.textContent = submit.dataset.originalText || 'Submit for Review';
                delete submit.dataset.originalText;
            }
        }
    }

    document.addEventListener('DOMContentLoaded', () => {
        document.querySelectorAll('[data-material-download]').forEach((button) => {
            button.addEventListener('click', () => downloadMaterial(button.dataset.materialDownload));
        });

        const downloadAll = document.querySelector('[data-download-all]');
        if (downloadAll) {
            downloadAll.addEventListener('click', downloadAllMaterials);
        }

        const form = document.getElementById('affiliate-material-upload-form');
        if (form) {
            form.addEventListener('submit', submitMaterialUpload);
            loadMaterials();
        }
    });
})();
