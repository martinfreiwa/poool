import re

with open('frontend/platform/transactions.html', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace Title
content = content.replace('<title>Wallet - POOOL</title>', '<title>Transactions - POOOL</title>')

# Replace Header and remove balances, transactions header
# The idea is to find <div id="wallet-header" class="wallet-header">
# and the start of <div id="wallet-transactions-section" class="wallet-transactions-section">
# and replace everything in between.

start_marker = '<div id="wallet-header" class="wallet-header">'
end_marker = '<div id="wallet-transactions-section" class="wallet-transactions-section">'

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

if start_idx != -1 and end_idx != -1:
    new_header = """<div id="wallet-header" class="wallet-header" style="flex-direction: column; align-items: flex-start; gap: 24px;">
    <div class="breadcrumbs" style="display: flex; align-items: center; font-family: 'Inter', sans-serif; font-size: 14px; line-height: 20px;">
        <a href="/wallet" style="color: #667085; text-decoration: none;">Wallet</a>
        <svg style="margin: 0 8px;" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M6 12L10 8L6 4" stroke="#D0D5DD" stroke-width="1.33333" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span style="color: #0000FF; font-weight: 500;">All transactions</span>
    </div>
    <div style="display: flex; justify-content: space-between; align-items: flex-end; width: 100%;">
        <h1 id="wallet-page-title" class="wallet-page-title" style="margin: 0;">Transactions</h1>
        
        <div style="display: flex; gap: 12px;">
            <div style="position: relative;">
                <svg style="position: absolute; left: 14px; top: 50%; transform: translateY(-50%);" width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M17.5 17.5L13.875 13.875M15.8333 9.16667C15.8333 12.8486 12.8486 15.8333 9.16667 15.8333C5.48477 15.8333 2.5 12.8486 2.5 9.16667C2.5 5.48477 5.48477 2.5 9.16667 2.5C12.8486 2.5 15.8333 5.48477 15.8333 9.16667Z" stroke="#667085" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                <input type="text" placeholder="Search" style="padding: 10px 14px 10px 40px; border: 1px solid #D0D5DD; border-radius: 8px; font-family: 'Inter', sans-serif; font-size: 16px; width: 240px; box-shadow: 0px 1px 2px rgba(16, 24, 40, 0.05); outline: none;">
            </div>
            <button style="display: flex; align-items: center; gap: 8px; padding: 10px 16px; border: 1px solid #D0D5DD; border-radius: 8px; background: white; cursor: pointer; font-family: 'Inter', sans-serif; font-weight: 600; font-size: 14px; color: #344054; box-shadow: 0px 1px 2px rgba(16, 24, 40, 0.05);">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M2.5 5.83333H17.5M5 10H15M8.33333 14.1667H11.6667" stroke="#344054" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Filter
            </button>
            <button style="display: flex; align-items: center; gap: 8px; padding: 10px 16px; border: 1px solid #D0D5DD; border-radius: 8px; background: white; cursor: pointer; font-family: 'Inter', sans-serif; font-weight: 600; font-size: 14px; color: #344054; box-shadow: 0px 1px 2px rgba(16, 24, 40, 0.05);">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M17.5 13.3333V14.1667C17.5 15.0871 16.7538 15.8333 15.8333 15.8333H4.16667C3.24619 15.8333 2.5 15.0871 2.5 14.1667V13.3333M14.1667 9.16667L10 13.3333M10 13.3333L5.83333 9.16667M10 13.3333V2.5" stroke="#344054" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Download report
            </button>
        </div>
    </div>
</div>
"""
    content = content[:start_idx] + new_header + content[end_idx:]

with open('frontend/platform/transactions.html', 'w', encoding='utf-8') as f:
    f.write(content)
