import { test as base, expect, BrowserContext } from '@playwright/test';

// ---------------------------------------------------------------------------
// Shared Playwright *test* object that injects the `X-Test-Worker` header *and*
// appends `worker=<id>` to every WebSocket URL opened by the front-end.  All
// existing spec files can simply switch their import to:
//
//   import { test, expect } from './fixtures';
//
// No other code changes are required.
// ---------------------------------------------------------------------------

type TestFixtures = {
  context: BrowserContext;
  request: import('@playwright/test').APIRequestContext;
};

export const test = base.extend<TestFixtures>({
  context: async ({ browser }, use, testInfo) => {
    const workerId = String(testInfo.workerIndex);

    const context = await browser.newContext({
      extraHTTPHeaders: {
        'X-Test-Worker': workerId,
      },

  request: async ({ playwright, baseURL }, use, testInfo) => {
    const workerId = String(testInfo.workerIndex);
    const request = await playwright.request.newContext({
      baseURL, // inherits from config
      extraHTTPHeaders: {
        'X-Test-Worker': workerId,
      },
    });
    await use(request);
    await request.dispose();
  },
    });

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

export { expect, Page } from '@playwright/test';
