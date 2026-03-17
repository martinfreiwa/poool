import re

with open('frontend/platform/transactions.html', 'r', encoding='utf-8') as f:
    text = f.read()

# We want to replace <div id="wallet-transactions-footer"...> with pagination
footer_start = '<div id="wallet-transactions-footer" class="wallet-transactions-footer">'

# Find it and everything till the end except the closing tags
start_idx = text.find(footer_start)

if start_idx != -1:
    # Look for </main> after this
    end_idx = text.find('</main>', start_idx)
    
    pagination = """<div style="display: flex; justify-content: space-between; align-items: center; padding: 16px 24px; border-top: 1px solid #EAECF0;">
    <div style="font-family: 'Inter', sans-serif; font-size: 14px; font-weight: 500; color: #344054;">
        Page 1 of 10
    </div>
    <div style="display: flex; gap: 12px;">
        <button style="padding: 8px 14px; border: 1px solid #D0D5DD; border-radius: 8px; background: white; cursor: pointer; font-family: 'Inter', sans-serif; font-weight: 600; font-size: 14px; color: #344054; box-shadow: 0px 1px 2px rgba(16, 24, 40, 0.05);">Previous</button>
        <button style="padding: 8px 14px; border: 1px solid #D0D5DD; border-radius: 8px; background: white; cursor: pointer; font-family: 'Inter', sans-serif; font-weight: 600; font-size: 14px; color: #344054; box-shadow: 0px 1px 2px rgba(16, 24, 40, 0.05);">Next</button>
    </div>
</div>
</div>
"""
    # Replace from footer_start to end_idx with pagination
    text = text[:start_idx] + pagination + text[end_idx:]

with open('frontend/platform/transactions.html', 'w', encoding='utf-8') as f:
    f.write(text)
