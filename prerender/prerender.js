const { chromium } = require("playwright-chromium");
const fs = require("fs").promises;
const path = require("path");

// Configuration
const SITE_URL = "http://localhost:8002"; // Your dev server URL
const OUTPUT_DIR = path.join(__dirname, "dist");
const WAIT_TIME = 5000; // Time to wait for WASM to execute fully (ms)

async function prerender() {
  console.log("Starting pre-rendering process...");
  
  // Create output directory if it doesn't exist
  await fs.mkdir(OUTPUT_DIR, { recursive: true });
  
  // Launch browser
  const browser = await chromium.launch();
  
  try {
    const context = await browser.newContext();
    const page = await context.newPage();
    
    // Navigate to the site
    console.log(`Loading ${SITE_URL}...`);
    await page.goto(SITE_URL, { waitUntil: "networkidle" });
    
    // Give time for WASM to execute and render content
    console.log(`Waiting ${WAIT_TIME/1000} seconds for WASM execution...`);
    await page.waitForTimeout(WAIT_TIME);
    
    // Get the rendered HTML
    const html = await page.content();
    
    // Save the HTML
    const outputPath = path.join(OUTPUT_DIR, "index.html");
    await fs.writeFile(outputPath, html);
    console.log(`Pre-rendered HTML saved to ${outputPath}`);
    
    // Take a screenshot for verification
    const screenshotPath = path.join(OUTPUT_DIR, "screenshot.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });
    console.log(`Screenshot saved to ${screenshotPath}`);
  } catch (error) {
    console.error("Error during pre-rendering:", error);
  } finally {
    await browser.close();
  }
  
  console.log("Pre-rendering complete!");
}

// Run the prerender function
prerender(); 