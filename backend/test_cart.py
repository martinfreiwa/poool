import requests
import subprocess
token = subprocess.check_output(['psql', '-Atc', "SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email='test@poool.app') ORDER BY created_at DESC LIMIT 1", 'poool']).decode().strip()
s = requests.Session()
s.cookies.set('poool_session', token)
r = s.get('http://localhost:8888/cart')
html = r.text
print('Mobile empty cart marker in response?', '<!-- Mobile Empty Cart State -->' in html)
print('mobile-cart-item-card in response?', 'mobile-cart-item-card' in html)
