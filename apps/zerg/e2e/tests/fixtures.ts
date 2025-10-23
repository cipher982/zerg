import { test as base, expect, BrowserContext, type Page } from '@playwright/test';

export type { Page };
import * as fs from 'fs';
import * as path from 'path';

// ---------------------------------------------------------------------------
// Shared Playwright *test* object that injects the `X-Test-Worker` header *and*
// appends `worker=<id>` to every WebSocket URL opened by the front-end.  All
// existing spec files can simply switch their import to:
//
//   import { test, expect } from './fixtures';
//
// No other code changes are required.
// ---------------------------------------------------------------------------

// Load dynamic backend port from .env
function getBackendPort(): number {
  // Check environment variable first
  if (process.env.BACKEND_PORT) {
    return parseInt(process.env.BACKEND_PORT);
  }
  
  // Load from .env file
  const envPath = path.resolve(__dirname, '../../.env');
  if (fs.existsSync(envPath)) {
    const envContent = fs.readFileSync(envPath, 'utf8');
    const lines = envContent.split('\n');
    for (const line of lines) {
      const [key, value] = line.split('=');
      if (key === 'BACKEND_PORT') {
        return parseInt(value) || 8001;
      }
    }
  }
  
  return 8001; // Default fallback
}

type TestFixtures = {
  context: BrowserContext;
  request: import('@playwright/test').APIRequestContext;
  backendUrl: string;
};

export const test = base.extend<TestFixtures>({
  backendUrl: async ({}, use) => {
    const basePort = getBackendPort();
    await use(`http://127.0.0.1:${basePort}`);
  },
  
  request: async ({ playwright, backendUrl }, use, testInfo) => {
    const workerId = String(testInfo.workerIndex);
    const request = await playwright.request.newContext({
      baseURL: backendUrl, // Use dynamic backend URL
      extraHTTPHeaders: {
        'X-Test-Worker': workerId,
      },
    });
    await use(request);
    await request.dispose();
  },
  
  context: async ({ browser }, use, testInfo) => {
    const workerId = String(testInfo.workerIndex);

    const context = await browser.newContext({
      extraHTTPHeaders: {
        'X-Test-Worker': workerId,
      },
    });

    const reactBaseUrl = process.env.PLAYWRIGHT_FRONTEND_BASE || 'http://localhost:3000';

    await context.addInitScript((config: { baseUrl: string, workerId: string }) => {
      try {
        const normalized = config.baseUrl.replace(/\/$/, '');
        window.localStorage.setItem('zerg_use_react_dashboard', '1');
        window.localStorage.setItem('zerg_use_react_chat', '1');
        window.localStorage.setItem('zerg_react_dashboard_url', `${normalized}/dashboard`);
        window.localStorage.setItem('zerg_react_chat_base', `${normalized}/chat`);

        // Add test JWT token for React authentication
        window.localStorage.setItem('zerg_jwt', 'test-jwt-token-for-e2e-tests');

        // Inject test worker ID for API request headers
        (window as any).__TEST_WORKER_ID__ = config.workerId;
        } catch (error) {
          // If localStorage is unavailable (unlikely), continue without failing tests.
          console.warn('Playwright init: unable to seed React flags', error);
        }
      }, { baseUrl: reactBaseUrl, workerId });
    } else {
      await context.addInitScript(() => {
        try {
          window.localStorage.removeItem('zerg_use_react_dashboard');
          window.localStorage.removeItem('zerg_react_dashboard_url');
          window.localStorage.removeItem('zerg_use_react_chat');
          window.localStorage.removeItem('zerg_react_chat_base');
        } catch (error) {
          console.warn('Playwright init: unable to clear React flags', error);
        }
      });
    }

    // -------------------------------------------------------------------
    // Monkey-patch *browser.newContext* so ad-hoc contexts created **inside**
    // a spec inherit the worker header automatically (see realtime_updates
    // tests that open multiple tabs).
    // -------------------------------------------------------------------
    const originalNewContext = browser.newContext.bind(browser);
    // Type-cast via immediate IIFE to keep TypeScript happy.
    browser.newContext = (async (options: any = {}) => {
      options.extraHTTPHeaders = {
        ...(options.extraHTTPHeaders || {}),
        'X-Test-Worker': workerId,
      };
      return originalNewContext(options);
    }) as any;

    // ---------------------------------------------------------------------
    // runtime patch – prepend `worker=<id>` to every WebSocket URL so the
    // backend can correlate the upgrade request to the correct database.
    // ---------------------------------------------------------------------
    await context.addInitScript((wid: string) => {
      const OriginalWebSocket = window.WebSocket;
      // @ts-ignore – internal helper wrapper
      // Wrap constructor in a type-asserted function expression so TS parser
      // accepts the cast.
      window.WebSocket = (function (url: string, protocols?: string | string[]) {
        try {
          const hasQuery = url.includes('?');
          const sep = hasQuery ? '&' : '?';
          url = `${url}${sep}worker=${wid}`;
        } catch {
          /* ignore – defensive */
        }
        // @ts-ignore – invoke original ctor
        return new OriginalWebSocket(url, protocols as any);
      }) as any;

      // Copy static properties (CONNECTING, OPEN, …)
      for (const key of Object.keys(OriginalWebSocket)) {
        // @ts-ignore – dynamic assignment
        (window.WebSocket as any)[key] = (OriginalWebSocket as any)[key];
      }
      (window.WebSocket as any).prototype = OriginalWebSocket.prototype;
    }, workerId);

    await use(context);

    await context.close();
  },

  // Re-export the *page* fixture so spec files work unchanged beyond the
  // import path.  “base” already provides the page linked to our custom
  // context.
});

export { expect } from '@playwright/test';
