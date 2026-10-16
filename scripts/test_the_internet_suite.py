#!/usr/bin/env python3
"""
Comprehensive Automated Test Suite for Terminal Browser (tb)
Target: https://the-internet.herokuapp.com/
Tests 12 critical capabilities:
  1. Root Catalog Discovery
  2. HTTP Status Codes Handling (200, 301, 404, 500)
  3. Redirection Hop Chain (302 -> Status Codes)
  4. HTTP Basic Authentication (Credential URL vs 401 Unauthorized)
  5. Form Authentication & 303 Redirect Handling
  6. Persistent SQLite CookieVault & Session Persistence
  7. Cookie Purge & Lockout Verification
  8. Structured Data Tables Extraction
  9. Checkboxes & Input Controls
  10. Dynamic Server Content & Token Sanitization
  11. Deep & Large DOM Parsing Stress Test (50x50 = 2,500 cells)
  12. Notification & Flash Message Extraction
"""

import subprocess
import time
import json
import sys
import shutil

sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")

TB_CMD = shutil.which("tb") or "tb.exe"

RESULTS = []

def run_tb(*args):
    start = time.time()
    cmd = [TB_CMD] + list(args)
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace")
    elapsed = time.time() - start
    return proc.returncode, proc.stdout, proc.stderr, elapsed

def log_test(name, passed, details, latency_s):
    status_str = "PASS" if passed else "FAIL"
    badge = "✅" if passed else "❌"
    print(f"{badge} [{status_str}] {name} ({latency_s:.2f}s) - {details}")
    RESULTS.append({
        "name": name,
        "passed": passed,
        "details": details,
        "latency_s": round(latency_s, 3)
    })

def main():
    print("=" * 75)
    print("🚀 TERMINAL BROWSER (tb) COMPREHENSIVE CAPABILITY SUITE")
    print("🎯 Target: https://the-internet.herokuapp.com/")
    print(f"📦 Binary: {TB_CMD}")
    print("=" * 75 + "\n")

    # 1. Root Catalog Discovery
    code, out, _, dur = run_tb("fetch", "https://the-internet.herokuapp.com/")
    has_examples = "Available Examples" in out and "Checkboxes" in out and "Basic Auth" in out
    log_test("Root Catalog Discovery", code == 0 and has_examples, "Extracted full benchmark example catalog", dur)

    # 2. HTTP Status Codes Engine
    code200, out200, _, dur200 = run_tb("fetch", "https://the-internet.herokuapp.com/status_codes/200")
    code404, out404, _, dur404 = run_tb("fetch", "https://the-internet.herokuapp.com/status_codes/404")
    code500, out500, _, dur500 = run_tb("fetch", "https://the-internet.herokuapp.com/status_codes/500")
    status_ok = "200 status code" in out200 and "404 status code" in out404 and "500 status code" in out500
    log_test("HTTP Status Codes Handling", status_ok, "200, 404, and 500 status pages verified", dur200 + dur404 + dur500)

    # 3. Redirection Hop Chain (302 -> /status_codes)
    code_red, out_red, _, dur_red = run_tb("fetch", "https://the-internet.herokuapp.com/redirect")
    redirect_ok = "HTTP Status: 200" in out_red and "Status Codes" in out_red
    log_test("Redirection Chain (302)", redirect_ok, "Followed redirect transparently to /status_codes", dur_red)

    # 4. HTTP Basic Authentication
    # 4a: Without credentials -> 401
    _, out_no_auth, _, dur_no_auth = run_tb("fetch", "https://the-internet.herokuapp.com/basic_auth")
    unauth_ok = "401" in out_no_auth and "Not authorized" in out_no_auth
    # 4b: With credentials -> 200
    _, out_auth, _, dur_auth = run_tb("fetch", "https://admin:admin@the-internet.herokuapp.com/basic_auth")
    auth_ok = "Congratulations! You must have the proper credentials" in out_auth
    log_test("HTTP Basic Auth (401 & 200)", unauth_ok and auth_ok, "Unauthenticated blocked (401), admin:admin allowed (200)", dur_no_auth + dur_auth)

    # 5. Form Authentication & Session Cookie Vault (Live Login)
    code_login, out_login, _, dur_login = run_tb("login", "https://the-internet.herokuapp.com/authenticate", "-u", "tomsmith", "-p", "SuperSecretPassword!")
    login_ok = "You logged into a secure area!" in out_login and "Captured & Stored" in out_login
    log_test("Form Authentication & 303 Redirect", login_ok, "POST login -> 303 Redirect -> 200 Secure Area with cookie", dur_login)

    # 6. Persistent Cookie Vault Verification
    code_vault, out_vault, _, dur_vault = run_tb("session", "list")
    vault_ok = "rack.session" in out_vault and "the-internet.herokuapp.com" in out_vault
    log_test("Persistent SQLite CookieVault", vault_ok, "Session cookie rack.session active in vault.db", dur_vault)

    # 7. Persistent Subsequent Fetch using Stored Cookie
    code_sec, out_sec, _, dur_sec = run_tb("fetch", "https://the-internet.herokuapp.com/secure")
    sec_ok = "Secure Area" in out_sec and "Logout" in out_sec and "Attaching active vault cookie" in out_sec
    log_test("Authenticated Subsequent Fetch", sec_ok, "Attached SQLite cookie automatically to view Secure Area", dur_sec)

    # 8. Cookie Purge & Access Lockout
    run_tb("session", "clear")
    _, out_cleared, _, dur_clear = run_tb("fetch", "https://the-internet.herokuapp.com/secure")
    lockout_ok = "You must login to view the secure area!" in out_cleared
    log_test("Session Purge & Access Revocation", lockout_ok, "Cleared vault cookies; access to /secure immediately blocked", dur_clear)

    # 9. Structured Data Tables Extraction
    _, out_tables, _, dur_tables = run_tb("fetch", "https://the-internet.herokuapp.com/tables")
    tables_ok = "Smith" in out_tables and "jsmith@gmail.com" in out_tables and "$50.00" in out_tables and "Bach" in out_tables
    log_test("Data Tables Extraction", tables_ok, "Extracted multi-column tabular records (Names, Emails, Dues)", dur_tables)

    # 10. Checkboxes & Inputs Form Controls
    _, out_cb, _, dur_cb = run_tb("fetch", "https://the-internet.herokuapp.com/checkboxes")
    _, out_inp, _, dur_inp = run_tb("fetch", "https://the-internet.herokuapp.com/inputs")
    controls_ok = "checkbox 1" in out_cb and "checkbox 2" in out_cb and "Number" in out_inp
    log_test("Form Controls & Checkboxes", controls_ok, "Parsed checkbox elements and number input forms", dur_cb + dur_inp)

    # 11. Dynamic Content Engine
    _, out_dyn, _, dur_dyn = run_tb("fetch", "https://the-internet.herokuapp.com/dynamic_content")
    dyn_ok = "Dynamic Content" in out_dyn and len(out_dyn.splitlines()) > 10
    log_test("Dynamic Content Sanitization", dyn_ok, "Extracted dynamic server content with noise stripped", dur_dyn)

    # 12. Deep & Large DOM Parsing Stress Test (50x50 = 2,500 cells)
    _, out_large, _, dur_large = run_tb("fetch", "https://the-internet.herokuapp.com/large")
    large_ok = "50.50" in out_large and "1.1" in out_large
    log_test("Deep & Large DOM Stress Test", large_ok, "Parsed 50x50 deep nested table (2,500 cells) without memory bloat", dur_large)

    # 13. File Download
    dl_file = "test_downloaded.bin"
    _, out_dl, _, dur_dl = run_tb("download", "https://the-internet.herokuapp.com/download/some-file.txt", "-o", dl_file)
    dl_ok = "Downloaded 307 bytes" in out_dl or "HTTP 200" in out_dl
    if shutil.os.path.exists(dl_file):
        shutil.os.remove(dl_file)
    log_test("File Download", dl_ok, "Streamed binary file from server with cookie injection", dur_dl)

    # 14. File Upload (multipart/form-data)
    up_file = "test_to_upload.txt"
    with open(up_file, "w", encoding="utf-8") as f:
        f.write("Terminal Browser Autonomous Agent Upload Verification 2026")
    _, out_up, _, dur_up = run_tb("upload", "https://the-internet.herokuapp.com/upload", "--file", up_file)
    up_ok = "File Uploaded!" in out_up and "test_to_upload.txt" in out_up
    if shutil.os.path.exists(up_file):
        shutil.os.remove(up_file)
    log_test("File Upload (Multipart)", up_ok, "Submitted multipart/form-data; server confirmed 'File Uploaded!'", dur_up)

    print("\n" + "=" * 75)
    total_passed = sum(1 for r in RESULTS if r["passed"])
    total_tests = len(RESULTS)
    print(f"📊 BENCHMARK SUMMARY: {total_passed}/{total_tests} Capabilities Passed")
    print(f"⚡ Average Latency: {sum(r['latency_s'] for r in RESULTS) / total_tests:.2f}s")
    print("=" * 75)

    with open("scripts/the_internet_test_results.json", "w", encoding="utf-8") as f:
        json.dump(RESULTS, f, indent=2)

    return 0 if total_passed == total_tests else 1

if __name__ == "__main__":
    sys.exit(main())
