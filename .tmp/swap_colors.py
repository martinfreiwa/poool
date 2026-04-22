import re

def swap_colors():
    with open('frontend/platform/commodities-preview.html', 'r') as f:
        content = f.read()

    # We will use CSS variables where possible, or hex codes if needed.
    BLUE = 'var(--blue)'
    MINT = 'var(--mint)'
    
    # 1. Swap Gold / Yellow tones
    content = content.replace('#d4af37', BLUE)
    content = content.replace('rgba(212, 175, 55, 0.1)', 'rgba(0, 0, 255, 0.1)')
    content = content.replace('rgba(212,175,55,0.1)', 'rgba(0, 0, 255, 0.1)')
    
    # 2. Swap Green tones 
    content = content.replace('#059669', BLUE) # Darker green -> Blue on light bgs
    content = content.replace('#10b981', MINT)
    content = content.replace('#34d399', MINT)
    content = content.replace('#a7f3d0', MINT)
    content = content.replace('#064e3b', BLUE) # Dark green background -> Blue
    content = content.replace('rgba(16, 185, 129, 0.1)', 'rgba(152, 251, 150, 0.15)')
    
    # 3. Swap other Blue tones to pure Electric
    content = content.replace('#3b82f6', BLUE)
    content = content.replace('#60a5fa', BLUE)
    
    # 4. Swap Red (Scarcity) to either Mint or Blue to strictly use brand colors
    content = content.replace('#ef4444', BLUE)
    content = content.replace('rgba(239, 68, 68, 0.5)', 'rgba(0, 0, 255, 0.5)')
    
    # 5. Fix up readability for Mint text on white backgrounds (Mint is too bright for white bg)
    # The badge "100% Capital Protected" originally had background #ecfdf5 color #059669.
    # It became background #ecfdf5 color var(--blue). That's fine.
    
    # 6. Any other stray colors
    # e.g., the gold SVGs were stroke="#d4af37". They are now stroke="var(--blue)".
    
    with open('frontend/platform/commodities-preview.html', 'w') as f:
        f.write(content)

if __name__ == "__main__":
    swap_colors()
    print("Done")
