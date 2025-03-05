# ZERG Pre-Rendering System

This directory contains a simple pre-rendering solution for the ZERG WebAssembly application to make it crawler-friendly while preserving the interactive experience for users.

## How It Works

1. The `prerender.js` script uses Puppeteer to:
   - Load the WASM application in a headless browser
   - Wait for the WASM to execute and render content
   - Capture the resulting HTML with all UI elements
   - Save this as a static HTML snapshot

2. The `server.js` provides a proxy server that:
   - Detects search engine crawlers by user agent
   - Serves pre-rendered HTML to crawlers
   - Serves normal WASM content to human users

## Setup and Usage

### Installation

```bash
cd prerender
npm install
```

### Generating Pre-rendered Content

1. Start your WASM application development server:
```bash
cd ../frontend
./build.sh
```

2. Start your backend server:
```bash
cd ../backend
uvicorn main:app --host 0.0.0.0 --port 8001
```

3. Generate the pre-rendered HTML:
```bash
cd ../prerender
npm run prerender
```

This will create a `dist` folder with:
- `index.html`: The pre-rendered HTML snapshot
- `screenshot.png`: A visual reference of what was captured

### Serving with Bot Detection

To run the server that automatically serves pre-rendered content to crawlers:

```bash
node server.js
```

Then access your site at http://localhost:8003

## Production Deployment

For production deployment, consider:

1. Automating the pre-rendering process as part of your build pipeline
2. Setting up a reverse proxy (Nginx, Cloudflare, etc.) to handle the bot detection
3. Implementing a caching strategy for pre-rendered content

## Testing

To test if the crawler detection works:

```bash
# Simulate a Google bot request
curl -A "Googlebot/2.1 (+http://www.google.com/bot.html)" http://localhost:8003/
```

You should receive the pre-rendered HTML content. 