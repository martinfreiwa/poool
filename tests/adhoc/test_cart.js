const fs = require('fs');

async function test() {
  const loginRes = await fetch("http://localhost:8888/auth/standard-login", {
    method: "POST",
    headers: {"Content-Type": "application/x-www-form-urlencoded"},
    body: "email=admin%40poool.finance&password=TestPass123%21",
    redirect: "manual"
  });
  
  const cookies = loginRes.headers.get("set-cookie");
  
  const cartRes = await fetch("http://localhost:8888/cart", {
    headers: { "cookie": cookies }
  });
  const html = await cartRes.text();
  fs.writeFileSync("/Users/martin/Projects/poool/cart_out.html", html);
  console.log("Written to cart_out.html. Summary box present?", html.includes('id="cart-page-summary"'));
}
test();
