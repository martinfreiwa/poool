import re

def fix_preview():
    with open('frontend/platform/commodities-preview.html', 'r') as f:
        content = f.read()

    # 1. Fix the card crushing issue
    content = content.replace('class="property-grid lp-grid card-v19"', 'class="lp-grid card-v19"')

    # 2. Remove the "cheap" project details section from all options
    # The section starts with: <!-- Project Details Section -->
    # And ends with the last </div> before  </div>\n  </section>
    
    # We can split by section and strip out the details block
    sections = content.split('<section class="option-container"')
    new_content = sections[0]
    
    for section in sections[1:]:
        # Find where the details block starts
        idx = section.find('<!-- Project Details Section -->')
        if idx != -1:
            # We must just remove the whole details block div. 
            # Luckily, the structure is:
            # <!-- Project Details Section -->
            # <div style="margin-top: ..."> ... </div>
            # </div>
            # </section>
            
            # Find the closing tag of the container which should be right after the details div.
            # We can use regex to remove everything from <!-- Project Details Section --> up to the final </div>\n  </section> (exclusive).
            
            # Simple regex to remove the block
            clean_section = re.sub(
                r'<!-- Project Details Section -->.*?</div>.*?</div>.*?</div>.*?</div>\s+</div>', 
                '', 
                section, 
                flags=re.DOTALL
            )
            
            # Sometimes regexes can be tricky with greedy matches. Let's just do a clean split.
            # The details block has 3 sub-divs. 
            # A safer approach:
            parts = section.split('<!-- Project Details Section -->')
            if len(parts) == 2:
                top_part = parts[0]
                # we just need to append the closing tags that come after the details block.
                # The remaining structure is always:
                #    </div>
                #  </section>
                new_section = top_part.rstrip() + "\n    </div>\n"
            else:
                new_section = section
                
            new_content += '<section class="option-container"' + new_section
        else:
            new_content += '<section class="option-container"' + section

    with open('frontend/platform/commodities-preview.html', 'w') as f:
        f.write(new_content)

if __name__ == "__main__":
    fix_preview()
    print("Fixed layout and removed text blocks.")
