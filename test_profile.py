import requests
url = "http://localhost:8888/api/admin/users/12300000-0000-0000-0000-000000000000/profile"
cookies = {"session": "dummy"} # Need a valid session context
payload = {"first_name": "Test"}
try:
    print(requests.post(url, json=payload, cookies=cookies).json())
except Exception as e:
    print("Error:", e)
