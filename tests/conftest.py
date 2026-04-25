"""Root pytest configuration for the POOOL test tree.

Some older integration checks in this directory are executable scripts that use
their own ``main()`` runners and intentionally named helper functions such as
``test_page(session, results, ...)``. Keep those scripts runnable directly while
preventing pytest from collecting their helpers as fixture-based tests.
"""

collect_ignore = [
    "admin/test_admin_dashboard.py",
    "admin/test_admin_features.py",
    "admin/test_admin_security.py",
    "admin/test_admin_sorting.py",
    "test_auth_login_register.py",
    "test_developer_dashboard.py",
    "test_e2e.py",
    "test_e2e_affiliate_full_funnel.py",
    "test_platform.py",
    "test_rewards.py",
    "test_security_audit.py",
]
