filepath = 'frontend/platform/static/css/kyc-banner.css'
if not os.path.exists(filepath):
    print("kyc-banner.css not found, skipping.")
    exit(0)

with open(filepath, 'a') as f:
    f.write("""
/* Developer Specific KYC Styling */
.kyc-banner-btn-primary {
    background-color: #0000FF !important;
    color: #98FB96 !important;
    border: none !important;
}
.kyc-banner-btn-primary .kyc-banner-btn-text {
    color: #98FB96 !important;
}
""")
print("Updated kyc-banner.css for developer profile.")
