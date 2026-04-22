from playwright.sync_api import sync_playwright
import os

def get_page_content(url):
    with sync_playwright() as p:
        # Launch chromium in headless mode
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        try:
            print(f"Navigating to {url}...")
            page.goto(url)
            
            # Use networkidle to ensure content is rendered
            page.wait_for_load_state('networkidle')
            
            # Wait for #main-content specifically if it's there
            try:
                page.wait_for_selector('#main-content', timeout=5000)
                content = page.locator('#main-content').inner_text()
                print("--- START OF #main-content ---")
                print(content)
                print("--- END OF #main-content ---")
            except Exception:
                print("Element #main-content not found. Getting full page text.")
                print(page.inner_text('body'))
            
            # Save a screenshot and the full HTML for debug
            page.screenshot(path='.tmp/landing_screenshot.png', full_page=True)
            print("Screenshot saved to .tmp/landing_screenshot.png")
            
            with open('.tmp/landing_rendered.html', 'w') as f:
                f.write(page.content())
            print("Rendered HTML saved to .tmp/landing_rendered.html")
            
        except Exception as e:
            print(f"Error: {e}")
        finally:
            browser.close()

if __name__ == "__main__":
    url = "http://localhost:8888/landing.html#main-content"
    get_page_content(url)
