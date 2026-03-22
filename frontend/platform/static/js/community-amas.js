/**
 * community-amas.js — Expert AMAs Tab Logic
 * Wires the Expert AMAs tab to /api/community/amas endpoints
 */
document.addEventListener('DOMContentLoaded', function () {

    let _activeAmaId = null;  // The "hero" AMA (live, accepting_questions, or upcoming)

    // ─── Status Config ──────────────────────────────────────────────
    const STATUS_BADGES = {
        'live':                 { text: '🔴 LIVE NOW',         bg: '#FEF3F2', color: '#B42318', border: '#FECDCA' },
        'accepting_questions':  { text: '🙋 QUESTIONS OPEN',   bg: '#ECFDF3', color: '#027A48', border: '#ABEFC6' },
        'scheduled':            { text: '📅 UPCOMING',          bg: '#EFF8FF', color: '#175CD3', border: '#B2DDFF' },
        'closed':               { text: 'CLOSED',               bg: '#F9FAFB', color: '#667085', border: '#EAECF0' },
        'archived':             { text: 'ARCHIVED',              bg: '#F9FAFB', color: '#98A2B3', border: '#EAECF0' },
    };

    // ─── Load All AMAs ──────────────────────────────────────────────

    async function loadAmas() {
        const loading = document.getElementById('ama-loading');
        const empty = document.getElementById('ama-empty');
        const hero = document.getElementById('ama-hero');
        const questionsSection = document.getElementById('ama-questions-section');
        const pastSection = document.getElementById('ama-past-section');

        try {
            const res = await fetch('/api/community/amas');
            if (!res.ok) {
                loading.style.display = 'none';
                empty.style.display = 'block';
                return;
            }
            const data = await res.json();
            const amas = data.amas || [];

            loading.style.display = 'none';

            if (amas.length === 0) {
                empty.style.display = 'block';
                hero.style.display = 'none';
                questionsSection.style.display = 'none';
                pastSection.style.display = 'none';
                return;
            }

            // Separate active vs past
            const activeStatuses = ['live', 'accepting_questions', 'scheduled'];
            const active = amas.find(a => activeStatuses.includes(a.status));
            const past = amas.filter(a => ['closed', 'archived'].includes(a.status));

            empty.style.display = 'none';

            // Render hero (active AMA)
            if (active) {
                renderHero(active);
                _activeAmaId = active.id;
                loadAmaDetail(active.id);
            } else {
                hero.style.display = 'none';
                questionsSection.style.display = 'none';
            }

            // Render past AMAs
            if (past.length > 0) {
                renderPastAmas(past);
            } else {
                pastSection.style.display = 'none';
            }

        } catch (e) {
            console.error('Failed to load AMAs', e);
            loading.style.display = 'none';
            empty.style.display = 'block';
        }
    }

    // ─── Render Hero Card ───────────────────────────────────────────

    function renderHero(ama) {
        const hero = document.getElementById('ama-hero');
        hero.style.display = 'block';

        // Badge
        const badge = STATUS_BADGES[ama.status] || STATUS_BADGES['scheduled'];
        const badgeEl = document.getElementById('ama-hero-badge');
        badgeEl.textContent = badge.text;
        badgeEl.style.background = badge.bg;
        badgeEl.style.color = badge.color;
        badgeEl.style.border = '1px solid ' + badge.border;

        // Title & Expert (use textContent for XSS safety)
        document.getElementById('ama-hero-title').textContent = ama.title;
        const expertEl = document.getElementById('ama-hero-expert');
        expertEl.textContent = '';
        const withSpan = document.createElement('span');
        withSpan.textContent = 'with ';
        const boldName = document.createElement('strong');
        boldName.textContent = ama.expert_name;
        expertEl.appendChild(withSpan);
        expertEl.appendChild(boldName);
        if (ama.expert_title) {
            const titleSpan = document.createElement('span');
            titleSpan.textContent = ' — ' + ama.expert_title;
            expertEl.appendChild(titleSpan);
        }

        // Description
        const descEl = document.getElementById('ama-hero-desc');
        descEl.textContent = ama.description || '';

        // Date
        const dateText = document.getElementById('ama-hero-date-text');
        if (ama.scheduled_at) {
            const d = new Date(ama.scheduled_at);
            dateText.textContent = d.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' }) +
                ' — ' + d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', timeZoneName: 'short' });
        } else {
            dateText.textContent = 'Date TBA';
        }

        // Submit question button — only show if accepting questions or live
        const submitBtn = document.getElementById('ama-submit-question-btn');
        if (['accepting_questions', 'live'].includes(ama.status)) {
            submitBtn.style.display = '';
        } else {
            submitBtn.style.display = 'none';
        }
    }

    // ─── Load AMA Detail (Questions) ────────────────────────────────

    async function loadAmaDetail(amaId) {
        const questionsSection = document.getElementById('ama-questions-section');
        const questionsList = document.getElementById('ama-questions-list');
        const countEl = document.getElementById('ama-question-count');

        try {
            const res = await fetch('/api/community/amas/' + amaId);
            if (!res.ok) return;
            const data = await res.json();

            const questions = data.questions || [];
            countEl.textContent = data.question_count + ' question' + (data.question_count !== 1 ? 's' : '');

            if (questions.length > 0) {
                questionsSection.style.display = 'block';
                questionsList.innerHTML = '';

                for (const q of questions) {
                    const card = document.createElement('div');
                    card.style.cssText = 'background:#fff;border:1px solid #EAECF0;border-radius:12px;padding:16px 20px;transition:box-shadow 0.2s;';

                    // Top row: question + featured badge
                    const topRow = document.createElement('div');
                    topRow.style.cssText = 'display:flex;align-items:flex-start;gap:12px;';

                    const questionText = document.createElement('p');
                    questionText.style.cssText = 'flex:1;font-size:14px;color:#181D27;line-height:1.5;margin:0;word-break:break-word;';
                    questionText.textContent = q.question;
                    topRow.appendChild(questionText);

                    if (q.is_featured) {
                        const featuredBadge = document.createElement('span');
                        featuredBadge.textContent = '⭐ Featured';
                        featuredBadge.style.cssText = 'white-space:nowrap;font-size:11px;font-weight:600;background:#FFF9C4;color:#F57F17;padding:2px 8px;border-radius:6px;border:1px solid #FFF59D;';
                        topRow.appendChild(featuredBadge);
                    }

                    card.appendChild(topRow);

                    // Answer (if answered)
                    if (q.answer) {
                        const answerDiv = document.createElement('div');
                        answerDiv.style.cssText = 'margin-top:12px;padding:12px 16px;background:#F0FDF4;border-radius:8px;border-left:3px solid #03FF88;';
                        const answerLabel = document.createElement('div');
                        answerLabel.style.cssText = 'font-size:11px;font-weight:700;color:#027A48;margin-bottom:4px;text-transform:uppercase;letter-spacing:0.5px;';
                        answerLabel.textContent = '💬 Expert Answer';
                        const answerText = document.createElement('p');
                        answerText.style.cssText = 'font-size:14px;color:#344054;line-height:1.5;margin:0;word-break:break-word;';
                        answerText.textContent = q.answer;
                        answerDiv.appendChild(answerLabel);
                        answerDiv.appendChild(answerText);
                        card.appendChild(answerDiv);
                    }

                    // Bottom row: upvote + time
                    const bottomRow = document.createElement('div');
                    bottomRow.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-top:12px;';

                    const upvoteBtn = document.createElement('button');
                    upvoteBtn.className = 'feed-reaction-btn';
                    upvoteBtn.style.cssText = 'display:flex;align-items:center;gap:6px;padding:6px 14px;border-radius:20px;border:1px solid #EAECF0;background:' +
                        (q.user_has_upvoted ? '#EFF8FF' : '#fff') + ';color:' + (q.user_has_upvoted ? '#175CD3' : '#667085') +
                        ';font-size:13px;font-weight:600;cursor:pointer;transition:all 0.15s;';
                    upvoteBtn.textContent = '👍 ' + q.upvote_count;
                    upvoteBtn.setAttribute('data-qid', q.id);
                    upvoteBtn.onclick = function () { handleUpvote(q.id, this); };
                    bottomRow.appendChild(upvoteBtn);

                    const timeSpan = document.createElement('span');
                    timeSpan.style.cssText = 'font-size:12px;color:#98A2B3;';
                    timeSpan.textContent = timeAgo(q.created_at);
                    bottomRow.appendChild(timeSpan);

                    card.appendChild(bottomRow);
                    questionsList.appendChild(card);
                }
            } else {
                questionsSection.style.display = 'block';
                questionsList.innerHTML = '<div style="text-align:center;padding:32px;color:#667085;font-size:14px;">No questions submitted yet. Be the first to ask!</div>';
            }
        } catch (e) {
            console.error('Failed to load AMA detail', e);
        }
    }

    // ─── Render Past AMAs ───────────────────────────────────────────

    function renderPastAmas(amas) {
        const section = document.getElementById('ama-past-section');
        const grid = document.getElementById('ama-past-grid');
        section.style.display = 'block';
        grid.innerHTML = '';

        for (const ama of amas) {
            const card = document.createElement('div');
            card.className = 'ama-archive-card';
            card.style.cursor = 'pointer';
            card.onclick = function () {
                _activeAmaId = ama.id;
                loadAmaDetail(ama.id);
                document.getElementById('ama-questions-section').scrollIntoView({ behavior: 'smooth' });
            };

            const topic = document.createElement('div');
            topic.className = 'ama-archive-topic';
            topic.textContent = ama.title;
            card.appendChild(topic);

            const expert = document.createElement('div');
            expert.className = 'ama-archive-expert';
            expert.textContent = 'with ' + ama.expert_name + (ama.expert_title ? ' — ' + ama.expert_title : '');
            card.appendChild(expert);

            const stats = document.createElement('div');
            stats.className = 'ama-archive-stats';
            const datePart = ama.scheduled_at ? new Date(ama.scheduled_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) : 'Past';
            stats.textContent = datePart + ' · ' + (ama.status === 'archived' ? 'Archived' : 'Closed');
            card.appendChild(stats);

            const link = document.createElement('a');
            link.className = 'ama-archive-link';
            link.textContent = 'View Q&A →';
            link.href = '#';
            link.onclick = function (e) {
                e.stopPropagation();
                _activeAmaId = ama.id;
                loadAmaDetail(ama.id);
                document.getElementById('ama-questions-section').scrollIntoView({ behavior: 'smooth' });
            };
            card.appendChild(link);

            grid.appendChild(card);
        }
    }

    // ─── Upvote Handler ─────────────────────────────────────────────

    async function handleUpvote(questionId, btn) {
        try {
            const res = await fetch('/api/community/amas/' + _activeAmaId + '/questions/' + questionId + '/upvote', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
            });
            if (!res.ok) throw new Error(await res.text());
            const data = await res.json();

            // Refresh the questions section to update counts
            if (_activeAmaId) {
                loadAmaDetail(_activeAmaId);
            }
        } catch (e) {
            console.error('Upvote failed', e);
        }
    }

    // ─── Question Submission ────────────────────────────────────────

    window.openQuestionModal = function () {
        const modal = document.getElementById('ama-question-modal');
        modal.style.display = 'flex';
        document.getElementById('ama-question-input').value = '';
        document.getElementById('ama-question-charcount').textContent = '0 / 500';
    };

    window.closeQuestionModal = function () {
        document.getElementById('ama-question-modal').style.display = 'none';
    };

    // Character counter
    const qInput = document.getElementById('ama-question-input');
    if (qInput) {
        qInput.addEventListener('input', function () {
            const len = this.value.length;
            document.getElementById('ama-question-charcount').textContent = len + ' / 500';
        });
    }

    window.submitQuestion = async function () {
        const input = document.getElementById('ama-question-input');
        const question = input.value.trim();

        if (question.length < 10) {
            return alert('Your question must be at least 10 characters.');
        }
        if (question.length > 500) {
            return alert('Question is too long. Max 500 characters.');
        }
        if (!_activeAmaId) {
            return alert('No active AMA to submit to.');
        }

        const btn = document.getElementById('ama-question-submit-btn');
        btn.disabled = true;
        btn.textContent = 'Submitting...';

        try {
            const res = await fetch('/api/community/amas/' + _activeAmaId + '/questions', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ question })
            });

            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }

            closeQuestionModal();
            loadAmaDetail(_activeAmaId);

            // Show success toast
            showToast('✅ Question submitted! The community can upvote it now.');
        } catch (e) {
            alert('Failed to submit: ' + e.message);
        } finally {
            btn.disabled = false;
            btn.textContent = 'Submit';
        }
    };

    // ─── Toast Helper ───────────────────────────────────────────────

    function showToast(msg) {
        const toast = document.createElement('div');
        toast.style.cssText = 'position:fixed;bottom:24px;left:50%;transform:translateX(-50%);background:#181D27;color:#fff;padding:12px 24px;border-radius:12px;font-size:14px;font-weight:500;z-index:10000;animation:fadeIn 0.3s ease;box-shadow:0 8px 24px rgba(0,0,0,0.2);';
        toast.textContent = msg;
        document.body.appendChild(toast);
        setTimeout(function () { toast.remove(); }, 4000);
    }

    // ─── Time Ago ───────────────────────────────────────────────────

    function timeAgo(dateStr) {
        const now = Date.now();
        const date = new Date(dateStr).getTime();
        const diff = Math.floor((now - date) / 1000);
        if (diff < 60) return 'just now';
        if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
        if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
        if (diff < 604800) return Math.floor(diff / 86400) + 'd ago';
        return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }

    // ─── Init ───────────────────────────────────────────────────────

    const amaTabBtn = document.querySelector('[data-tab="community-ama-tab"]');
    if (amaTabBtn) {
        amaTabBtn.addEventListener('click', function () {
            loadAmas();
        });
    }

    window.loadAmas = loadAmas;
});
