#!/usr/bin/env python3
"""
End-to-End Legal and Static Pages Test
======================================
Verifies that all required legal and static pages are accessible and return 200 OK.
"""
import os
import requests
import sys

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

class E2EResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0

    def check(self, name, condition, detail=""):
        if condition:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            print(f"  ❌ {name} {' - ' + detail if detail else ''}")

    def report(self):
        print("\n" + "="*60)
        print(f"E2E Legal & Static Pages Report: {self.passed} Passed, {self.failed} Failed")
        print("="*60 + "\n")
        return self.failed == 0

def run_legal_tests():
    results = E2EResults()
    print("\n--- Testing Legal and Static Pages ---")
    
    pages = [
        "/terms",
        "/legal/terms",
        "/privacy-policy",
        "/privacy",
        "/legal/privacy",
        "/imprint",
        "/legal/imprint",
        "/aml-kyc-policy",
        "/legal/aml-kyc-policy",
        "/currency-policy",
        "/legal/currency",
        "/legal/currency-policy",
        "/cookies",
        "/legal/cookies",
        "/gdpr-data-request",
        "/legal/gdpr-data-request",
    ]
    
    for page in pages:
        resp = requests.get(f"{BASE_URL}{page}")
        results.check(f"GET {page}", resp.status_code == 200, f"Status: {resp.status_code}")
        if resp.status_code == 200:
             results.check(f"Content Check {page}", "<html" in resp.text.lower(), "Missing HTML output")
             results.check(
                 f"Not Found Template Check {page}",
                 "404 page not found" not in resp.text.lower(),
                 "Rendered 404 template",
             )
        
    return results

if __name__ == "__main__":
    res = run_legal_tests()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
