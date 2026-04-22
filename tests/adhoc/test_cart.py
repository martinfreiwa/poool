import urllib.request
import urllib.parse
from http.cookiejar import CookieJar

cj = CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cj))

# Login
login_data = urllib.parse.urlencode({'email': 'admin@poool.finance', 'password': 'TestPass123!'}).encode('utf-8')
try:
    opener.open('http://localhost:8888/auth/standard-login', data=login_data)
except Exception as e:
    print("Login err:", e)

# Fetch cart
cart_req = urllib.request.Request('http://localhost:8888/cart')
try:
    resp = opener.open(cart_req)
    html = resp.read().decode('utf-8')
    with open('/Users/martin/Projects/poool/cart_out.html', 'w') as f:
        f.write(html)
    print("Saved to cart_out.html")
except Exception as e:
    print("Cart err:", e)
