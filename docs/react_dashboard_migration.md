# React Frontend Migration Pilots

This document tracks the pilot migrations from the Rust/WASM frontend to a React + TypeScript stack located in `frontend-web/`.

## Local Development

1. Install dependencies via the existing root workspace:

   ```bash
   npm install
   ```

2. Start the React dev server:

   ```bash
   cd frontend-web
   npm run dev
   ```

   The Vite server runs on `http://localhost:3000` and proxies API calls to the FastAPI backend on `:8001`.

3. Enable the strangler flag in the legacy UI. In the browser console run:

   ```javascript
   localStorage.setItem("zerg_use_react_dashboard", "1");
   localStorage.setItem("zerg_react_dashboard_url", "http://localhost:3000/dashboard");
   ```

   Reload the dashboard route; the legacy app will redirect to the React version using the configured URL.

4. Disable the flag by clearing the keys:

   ```javascript
   localStorage.removeItem("zerg_use_react_dashboard");
   localStorage.removeItem("zerg_react_dashboard_url");
   ```

## React Chat Pilot

To test the chat experience in React, toggle the chat-specific flag:

```javascript
localStorage.setItem("zerg_use_react_chat", "1");
localStorage.setItem("zerg_react_chat_base", "http://localhost:3000/chat");
```

When the legacy SPA navigates to the chat view it will redirect to `/chat/<agentId>/<threadId?>` at the provided base URL. Remove the keys to return to the Rust implementation.

## Next Steps

- Flesh out the React dashboard feature set, replicating the current agent list, actions, and live updates.
- Add automated tests (Vitest + Playwright) targeting the new stack and wire them into CI.
- Expand strangler routing to other pages (chat, canvas) once each reaches parity.
