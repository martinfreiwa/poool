const fs = require("fs");
const path = require("path");

const targetFiles = [
  "cart.html",
  "commodities-marketplace.html",
  "commodity.html",
  "developer/add-asset.html",
  "developer/application-form.html",
  "developer/assets.html",
  "developer/dashboard.html",
  "developer/document-upload-step3.html",
  "developer/property-content.html",
  "developer/submission-success.html",
  "forgot-password.html",
  "index.html",
  "kyc.html",
  "login.html",
  "marketplace.html",
  "portfolio.html",
  "property.html",
  "rewards.html",
  "settings.html",
  "signup.html",
  "support.html",
  "wallet.html",
];

const basePath = "/Users/martin/Projects/poool/frontend/platform";
const searchRegex = /<script src="\/static\/js\/user-data\.js.*?"\s*><\/script>/;
const replacementString =
  '<script src="/static/js/user-data.js"></script>\n  <link rel="stylesheet" href="/static/css/poool-dropdown.css">\n  <script src="/static/js/poool-dropdown.js"></script>\n  <script src="/static/js/poool-dropdown-init.js"></script>';

let modifiedCount = 0;

targetFiles.forEach((file) => {
  const filePath = path.join(basePath, file);
  try {
    let content = fs.readFileSync(filePath, "utf8");

    // Check if the file already has the poool-dropdown.css to avoid double injection
    if (content.includes("poool-dropdown.css")) {
      return;
    }

    if (searchRegex.test(content)) {
      content = content.replace(searchRegex, replacementString);
      fs.writeFileSync(filePath, content, "utf8");
      modifiedCount++;
    } else {
    }
  } catch (err) {
    console.error("Error replacing in file:", file, err);
  }
});
