const express = require("express");
const path = require("path");
const fs = require("fs").promises;

const app = express();
const PORT = process.env.PORT || 8003;

// Path to the pre-rendered content
const PRERENDERED_HTML_PATH = path.join(__dirname, "dist", "index.html");
// Path to the original frontend
const FRONTEND_PATH = path.join(__dirname, "..", "frontend", "www");

// Bot user agents to check for
const BOT_USER_AGENTS = [
  "googlebot",
  "bingbot",
  "yandexbot",
  "duckduckbot",
  "slurp",
  "baiduspider",
  "facebookexternalhit",
  "twitterbot",
  "rogerbot",
  "linkedinbot",
  "embedly",
  "quora link preview",
  "showyoubot",
  "outbrain",
  "pinterest/0.",
  "developers.google.com/+/web/snippet",
  "slackbot",
  "vkshare",
  "w3c_validator",
  "redditbot",
  "applebot",
  "whatsapp"
];

// Middleware to check if request is from a bot
function isBot(req) {
  const userAgent = req.headers["user-agent"]?.toLowerCase() || "";
  return BOT_USER_AGENTS.some(bot => userAgent.includes(bot));
}

// Main middleware for serving appropriate content
app.use(async (req, res, next) => {
  // Only intercept the root path
  if (req.path !== "/") {
    return next();
  }

  try {
    if (isBot(req)) {
      console.log("Bot detected, serving pre-rendered content");
      const prerenderedHTML = await fs.readFile(PRERENDERED_HTML_PATH, "utf8");
      return res.send(prerenderedHTML);
    } 
    
    // For non-bot requests, just pass through to the static middleware
    next();
  } catch (error) {
    console.error("Error serving content:", error);
    next();
  }
});

// Serve the original frontend for human users
app.use(express.static(FRONTEND_PATH));

// Start the server
app.listen(PORT, () => {
  console.log(`Server running at http://localhost:${PORT}`);
  console.log(`- Serving frontend from: ${FRONTEND_PATH}`);
  console.log(`- Serving pre-rendered content to bots from: ${PRERENDERED_HTML_PATH}`);
}); 