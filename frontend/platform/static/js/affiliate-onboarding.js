/**
 * Affiliate Onboarding Wizard — Client-Side Logic
 *
 * Handles: step navigation, progress bar, form validation,
 * KYC status check (via API), and compliance exam submission.
 */
document.addEventListener('DOMContentLoaded', () => {
    let currentStep = 1;
    const totalSteps = 5;

    // ─── KYC Status Check ───
    // Fetch real KYC status from backend instead of relying on Jinja template
    fetchKycStatus();

    async function fetchKycStatus() {
        try {
            const res = await fetch('/api/kyc/status', { credentials: 'same-origin' });
            if (!res.ok) return;
            const data = await res.json();
            const status = data.status || data.kyc_status || '';
            if (status === 'approved' || status === 'verified') {
                const verified = document.getElementById('kyc-verified');
                const pending = document.getElementById('kyc-pending');
                if (verified) verified.style.display = 'flex';
                if (pending) pending.style.display = 'none';
            }
        } catch (_) {
            // Silently fail — default state shows "Verification Required"
        }
    }

    // ─── Public API ───

    window.nextStep = function(step) {
        if (validateStep(step - 1)) {
            showStep(step);
        }
    };

    window.prevStep = function(step) {
        showStep(step);
    };

    window.submitExam = async function() {
        const answers = {};
        for (let i = 1; i <= 5; i++) {
            const checked = document.querySelector(`input[name="q${i}"]:checked`);
            if (!checked) {
                showToast('Please answer all questions before submitting.', 'warning');
                return;
            }
            answers['q' + i] = checked.value;
        }

        // Correct answers from the Affiliate Code of Conduct
        const correct = { q1: 'no', q2: 'no', q3: '30days', q4: 'no', q5: 'no' };
        const passed = Object.keys(correct).every(k => answers[k] === correct[k]);

        if (passed) {
            // Gather profile data
            const trafficSource = document.getElementById('traffic-source')?.value || '';
            const audienceSize = document.getElementById('audience-size')?.value || '';
            const mainUrl = document.getElementById('main-url')?.value || '';
            const phoneNumber = document.getElementById('phone-number')?.value || '';
            
            // Gather tax data
            const taxId = document.getElementById('tax-id')?.value || '';
            const companyName = document.getElementById('company-name')?.value || '';

            // Gather accepted policies
            const acceptedPolicies = [];
            if (document.getElementById('cb-terms')?.checked) acceptedPolicies.push('Affiliate Terms & Conditions');
            if (document.getElementById('cb-conduct')?.checked) acceptedPolicies.push('Affiliate Code of Conduct');
            if (document.getElementById('cb-materials')?.checked) acceptedPolicies.push('Approved Marketing Materials Policy');
            if (document.getElementById('cb-payout')?.checked) acceptedPolicies.push('Qualified Referral & Payout Policy');
            if (document.getElementById('cb-privacy')?.checked) acceptedPolicies.push('Affiliate Privacy Notice');

            const btn = document.getElementById('submit-exam-btn');
            if (btn) {
                btn.disabled = true;
                btn.innerText = 'Submitting...';
            }

            // Submit application for admin review
            try {
                const res = await fetch('/api/affiliate/onboarding/submit', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        exam_passed: true,
                        exam_answers: answers,
                        status: 'pending_review',
                        traffic_source: trafficSource,
                        audience_size: audienceSize,
                        main_url: mainUrl,
                        phone_number: phoneNumber,
                        tax_id: taxId,
                        company_name: companyName ? companyName : null,
                        accepted_policies: acceptedPolicies
                    }),
                });

                if (!res.ok) {
                    const data = await res.json().catch(() => ({}));
                    const errorMsg = data.error || 'Failed to submit application.';

                    // Handle specific status codes
                    if (res.status === 409) {
                        showToast(errorMsg, 'warning');
                    } else if (res.status === 429) {
                        showToast(errorMsg, 'warning');
                    } else if (res.status === 403) {
                        showToast(errorMsg, 'error');
                    } else {
                        showToast(errorMsg, 'error');
                    }

                    if (btn) {
                        btn.disabled = false;
                        btn.innerText = 'Submit Application';
                    }
                    return;
                }

                // Hide exam, show pending review state
                document.getElementById('step-5').style.display = 'none';
                document.getElementById('step-success').style.display = 'block';

                // Mark all steps as completed
                document.querySelectorAll('.step-item').forEach(el => {
                    el.classList.remove('active');
                    el.classList.add('completed');
                });
                // Fill progress bar to 100%
                const fill = document.getElementById('stepper-fill');
                if (fill) fill.style.width = '100%';
            } catch (err) {
                console.error('[Onboarding] POST to /api/affiliate/onboarding/submit failed:', err);
                showToast('Failed to submit application. Please try again.', 'error');
                if (btn) {
                    btn.disabled = false;
                    btn.innerText = 'Submit Application';
                }
            }
        } else {
            showToast('One or more answers are incorrect. The compliance quiz requires a 100% pass score. Please review the Syndicate rules and try again.', 'error');

            // Reset only the radio buttons, not the entire form state
            document.querySelectorAll('#quiz-form input[type="radio"]').forEach(r => r.checked = false);
            // Also reset the visual state of radio cards
            document.querySelectorAll('#quiz-form .radio-card-box').forEach(box => {
                box.style.borderColor = '';
                box.style.background = '';
                box.style.boxShadow = '';
            });
        }
    };


    // ─── Validation ───

    function validateStep(step) {
        if (step === 1) {
            const form = document.getElementById('profile-form');
            let valid = true;

            // Check custom dropdowns (hidden selects)
            ['traffic-source', 'audience-size'].forEach(id => {
                const sel = document.getElementById(id);
                if (sel && (!sel.value || sel.value === '')) {
                    valid = false;
                    // Highlight the POOOL dropdown trigger
                    const wrapper = sel.closest('.poool-dropdown');
                    if (wrapper) {
                        const trigger = wrapper.querySelector('.poool-dropdown__trigger');
                        if (trigger) trigger.classList.add('poool-dropdown__trigger--error');
                        // Remove error state when user selects something
                        wrapper.addEventListener('dropdown:change', () => {
                            if (trigger) trigger.classList.remove('poool-dropdown__trigger--error');
                        }, { once: true });
                    }
                }
            });

            // Check native inputs (URL field)
            const urlInput = document.getElementById('main-url');
            if (urlInput && !urlInput.value.trim()) {
                valid = false;
            }

            if (!valid) {
                // Try reportValidity for the URL field; dropdowns show visual error state
                if (urlInput && !urlInput.value.trim()) {
                    urlInput.reportValidity();
                } else {
                    showToast('Please fill in all required fields.', 'warning');
                }
                return false;
            }
        }
        if (step === 2) {
            const verified = document.getElementById('kyc-verified');
            if (verified && verified.style.display === 'none') {
                showToast('You must complete Identity Verification before proceeding.', 'error');
                return false;
            }
        }
        if (step === 3) {
            const form = document.getElementById('tax-form');
            if (!form.checkValidity()) {
                form.reportValidity();
                return false;
            }
        }
        if (step === 4) {
            const boxes = document.querySelectorAll('#step-4 .ds-checkbox-input');
            for (const box of boxes) {
                if (!box.checked) {
                    showToast('You must accept all terms to continue.', 'warning');
                    return false;
                }
            }
        }
        return true;
    }

    // ─── Step Navigation ───

    function showStep(step) {
        // Hide all steps
        document.querySelectorAll('.wizard-step').forEach(el => el.style.display = 'none');

        // Show target step
        const target = document.getElementById('step-' + step);
        if (target) {
            target.style.display = 'block';
            currentStep = step;
        }

        // Update stepper indicators
        document.querySelectorAll('.step-item').forEach(el => {
            const s = parseInt(el.dataset.step);
            el.classList.remove('active', 'completed');
            if (s < step) {
                el.classList.add('completed');
            } else if (s === step) {
                el.classList.add('active');
            }
        });

        // Update progress bar
        updateProgressBar(step);

        // Scroll wizard into view on mobile
        const stepper = document.getElementById('wizard-stepper');
        if (stepper && window.innerWidth < 768) {
            stepper.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    }

    function updateProgressBar(step) {
        const fill = document.getElementById('stepper-fill');
        if (fill) {
            // Step 1 = 0%, Step 5 = 100%
            const pct = ((step - 1) / (totalSteps - 1)) * 100;
            fill.style.width = pct + '%';
        }
    }

    // ─── Toast Helper ───

    function showToast(message, type) {
        // Use the platform's toast system if available
        if (typeof window.showToastMessage === 'function') {
            window.showToastMessage(message, type);
            return;
        }
        // Fallback to alert
        alert(message);
    }

    // ─── Legal Tab Switching ───

    window.switchLegalTab = function(btn) {
        const targetId = btn.getAttribute('data-target');
        if (!targetId) return;

        // Switch tab buttons
        document.querySelectorAll('.legal-tab').forEach(t => t.classList.remove('active'));
        btn.classList.add('active');

        // Switch tab content
        document.querySelectorAll('.legal-tab-content').forEach(c => {
            c.classList.remove('active');
            c.style.display = 'none';
        });
        const target = document.getElementById(targetId);
        if (target) {
            target.classList.add('active');
            target.style.display = 'block';
            target.scrollTop = 0; // Reset scroll position
        }

        // Reset scroll hint visibility
        const hint = document.getElementById('legal-scroll-hint');
        if (hint) hint.classList.remove('hidden');
    };

    // ─── Scroll Hint Auto-Hide ───

    function initScrollHint() {
        const boxes = document.querySelectorAll('.wizard-legal-box');
        const hint = document.getElementById('legal-scroll-hint');
        if (!hint) return;

        boxes.forEach(box => {
            box.addEventListener('scroll', () => {
                // Hide hint when user has scrolled near the bottom
                const atBottom = box.scrollTop + box.clientHeight >= box.scrollHeight - 20;
                if (atBottom) {
                    hint.classList.add('hidden');
                }
            });
        });
    }

    // ─── Initialize ───
    updateProgressBar(1);
    initScrollHint();
    initDropdowns();

    // ─── Convert Native Selects to POOOL Dropdowns ───
    function initDropdowns() {
        if (typeof window.PooolDropdown === 'undefined') return;

        // Traffic Source dropdown
        const trafficSelect = document.getElementById('traffic-source');
        if (trafficSelect) {
            PooolDropdown.fromSelect(trafficSelect, {
                placeholder: 'Select an option...',
                noLabel: true, // Label already exists via ds-label
            });
        }

        // Audience Size dropdown
        const audienceSelect = document.getElementById('audience-size');
        if (audienceSelect) {
            PooolDropdown.fromSelect(audienceSelect, {
                placeholder: 'Select audience size...',
                noLabel: true,
            });
        }
    }
});
