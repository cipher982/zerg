import { test, expect } from './fixtures';
import { 
  setupTestData, 
  cleanupTestData, 
  waitForDashboardReady, 
  checkBackendHealth,
  TestContext
} from './helpers/test-helpers';
import { 
  setupDebugCapture, 
  waitForWasmInit, 
  debugDomState 
} from './helpers/debug-helpers';

test.describe('Debug Smoke Tests - Identify Frontend Issues', () => {
  let testContext: TestContext;

  test.beforeEach(async () => {
    // Ensure backend is healthy
    const isHealthy = await checkBackendHealth();
    expect(isHealthy).toBe(true);
    
    // Start with clean state
    testContext = { agents: [], threads: [] };
  });

  test.afterEach(async () => {
    // Clean up any test data
    await cleanupTestData(testContext);
  });

  test('Debug: Frontend initialization sequence', async ({ page }) => {
    // Set up debug capture
    const debug = await setupDebugCapture(page, 'Frontend initialization');
    
    // Navigate to the page
    console.log('Navigating to http://localhost:8002');
    await page.goto('http://localhost:8002');
    
    // Wait a moment for initial page load
    await page.waitForTimeout(2000);
    
    // Debug: Check DOM state immediately after navigation
    console.log('\n=== Initial DOM State ===');
    await debugDomState(page);
    
    // Check if basic HTML loaded
    const title = await page.title();
    console.log('Page title:', title);
    
    // Check for critical files
    const scriptsLoaded = await page.evaluate(() => {
      const scripts = Array.from(document.querySelectorAll('script')).map(s => ({
        src: s.src,
        type: s.type,
        async: s.async,
        defer: s.defer
      }));
      return scripts;
    });
    console.log('Scripts loaded:', JSON.stringify(scriptsLoaded, null, 2));
    
    // Wait for WASM initialization
    console.log('\n=== Waiting for WASM initialization ===');
    const wasmInitialized = await waitForWasmInit(page, 15000);
    console.log('WASM initialized:', wasmInitialized);
    
    // Check DOM state after WASM should have loaded
    console.log('\n=== DOM State after WASM wait ===');
    await debugDomState(page);
    
    // Check if config.js loaded properly
    const configLoaded = await page.evaluate(() => {
      return (window as any).API_CONFIG || (window as any).APP_CONFIG;
    });
    console.log('Config loaded:', configLoaded);
    
    // Try to check for any WASM-related errors
    const wasmErrors = debug.errors.filter(e => 
      e.toLowerCase().includes('wasm') || 
      e.toLowerCase().includes('module') ||
      e.toLowerCase().includes('import')
    );
    if (wasmErrors.length > 0) {
      console.error('WASM-related errors found:', wasmErrors);
    }
    
    // Check network errors
    if (debug.networkErrors.length > 0) {
      console.error('Network errors:', debug.networkErrors);
    }
    
    // Dump all debug logs
    debug.dumpLogs();
    
    // Final check: Can we find any dashboard elements?
    const dashboardElements = await page.evaluate(() => {
      return {
        appContainer: !!document.querySelector('#app-container'),
        dashboardRoot: !!document.querySelector('#dashboard-root'),
        dashboardContainer: !!document.querySelector('#dashboard-container'),
        dashboard: !!document.querySelector('#dashboard'),
        table: !!document.querySelector('table'),
        createButton: !!document.querySelector('[data-testid="create-agent-btn"]')
      };
    });
    console.log('Dashboard elements found:', dashboardElements);
    
    // The test passes if we get this far - we're just debugging
    expect(true).toBe(true);
  });

  test('Debug: API connectivity check', async ({ page }) => {
    const debug = await setupDebugCapture(page, 'API connectivity');
    
    await page.goto('http://localhost:8002');
    
    // Check if frontend can reach backend
    const apiResponse = await page.evaluate(async () => {
      try {
        const response = await fetch('http://localhost:8001/api/health');
        const data = await response.json();
        return { success: true, data, status: response.status };
      } catch (error) {
        return { success: false, error: error.toString() };
      }
    });
    
    console.log('API health check from browser:', apiResponse);
    
    // Check system info endpoint
    const systemInfo = await page.evaluate(async () => {
      try {
        const response = await fetch('http://localhost:8001/api/system/info');
        const data = await response.json();
        return { success: true, data, status: response.status };
      } catch (error) {
        return { success: false, error: error.toString() };
      }
    });
    
    console.log('System info from browser:', systemInfo);
    
    debug.dumpLogs();
    
    expect(apiResponse.success).toBe(true);
  });

  test('Debug: Step-by-step dashboard load', async ({ page }) => {
    const debug = await setupDebugCapture(page, 'Step-by-step dashboard');
    
    // Go to page
    await page.goto('http://localhost:8002');
    console.log('1. Navigated to page');
    
    // Wait for any content
    await page.waitForTimeout(3000);
    console.log('2. Waited 3 seconds');
    
    // Check what's in the DOM
    const elements = await page.evaluate(() => {
      const checkElement = (selector: string) => {
        const el = document.querySelector(selector);
        if (!el) return null;
        return {
          exists: true,
          innerHTML: el.innerHTML.substring(0, 100),
          children: el.children.length,
          visible: (el as HTMLElement).offsetWidth > 0 && (el as HTMLElement).offsetHeight > 0
        };
      };
      
      return {
        body: checkElement('body'),
        appContainer: checkElement('#app-container'),
        dashboardRoot: checkElement('#dashboard-root'),
        dashboardContainer: checkElement('#dashboard-container'),
        dashboard: checkElement('#dashboard'),
        table: checkElement('table'),
        createBtn: checkElement('[data-testid="create-agent-btn"]')
      };
    });
    
    console.log('3. DOM elements check:', JSON.stringify(elements, null, 2));
    
    // Try to manually trigger dashboard render if it exists
    const manualRender = await page.evaluate(() => {
      if ((window as any).crate && (window as any).crate.components && (window as any).crate.components.dashboard) {
        try {
          const doc = document;
          (window as any).crate.components.dashboard.refresh_dashboard(doc);
          return 'Dashboard refresh called';
        } catch (e) {
          return 'Error calling refresh: ' + e.toString();
        }
      }
      return 'Dashboard module not found in window.crate';
    });
    
    console.log('4. Manual render attempt:', manualRender);
    
    // Wait and check again
    await page.waitForTimeout(2000);
    
    const finalCheck = await page.evaluate(() => {
      return {
        tableExists: !!document.querySelector('table'),
        tableRows: document.querySelectorAll('tr').length,
        buttons: Array.from(document.querySelectorAll('button')).map(b => b.textContent)
      };
    });
    
    console.log('5. Final check:', finalCheck);
    
    debug.dumpLogs();
    
    // This test is for debugging, not assertions
    expect(true).toBe(true);
  });
});
