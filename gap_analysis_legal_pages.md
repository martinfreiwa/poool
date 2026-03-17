# Gap Analysis: Missing Legal & Compliance Pages

After reviewing the current `platform` directory, the following legal and compliance pages are currently present:
- `terms.html` (Terms and Conditions)
- `privacy-policy.html` (Privacy Policy)
- `cookies.html` (Cookie Policy)
- `currency-policy.html` (Currency Policy)

For a fully production-ready fractional investment platform, the following critical legal/compliance pages are missing and need to be created:

1. **Anti-Money Laundering (AML) Policy** (`aml-policy.html`)
   - Detailed explanation of how the platform monitors transactions and prevents money laundering activities.

2. **Investment Risk Disclosure** (`risk-disclosure.html`)
   - Mandatory disclaimer stating that fractional investments carry risk, past performance does not guarantee future results, and users may lose their principal.

3. **Know Your Customer (KYC) Verification Terms** (`kyc-policy.html`)
   - Specific terms outlining the data collection and verification process used by our KYC vendors, user responsibilities, and data retention specific to KYC. 

4. **Complaints & Dispute Resolution Policy** (`complaints-policy.html`)
   - Formal procedure for users to submit complaints, response time SLAs, and escalation paths (regulatory bodies if applicable).

5. **Localized/Regional Terms** (`eu-gdpr-notice.html`, etc.)
   - Specific addendums depending on the operating jurisdictions (e.g., GDPR rights for EU users, or SEC disclaimers for US users).

## Recommended Action
Create blocking issues for content drafting and legal review for the above 5 documents, followed by implementation as standalone HTML pages in the `frontend/platform/` directory following the existing layout conventions used in `terms.html`.
