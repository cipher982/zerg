# 🌐 Swarm Platform

Unified agent orchestration platform combining **Jarvis** (voice/text interface) with **Zerg** (backend agent orchestration).

## Vision

- **Jarvis**: ChatGPT-style PWA (voice + text) providing instant responses, context memory, and a Task Inbox summarizing automated activity
- **Zerg**: FastAPI + LangGraph backend running durable workflows, cron schedules, tool integrations (MCP + custom), history, and audit logs
- **Unified control**: Jarvis issues commands, Zerg executes and reports back—no manual terminals required

## Quick Start

```bash
# Install dependencies
npm install
cd apps/jarvis && npm install && cd ../..
cd apps/zerg/backend && uv sync && cd ../../..

# Copy and configure environment
cp .env.example.swarm .env
# Edit .env with your API keys and secrets

# Start full swarm (Jarvis + Zerg)
make swarm-dev
```

Access:
- **Jarvis UI**: http://localhost:8080
- **Zerg Backend API**: http://localhost:47300
- **Zerg Frontend**: http://localhost:47200

## Monorepo Structure

```
/
├── apps/
│   ├── jarvis/           # Voice/text PWA interface
│   │   ├── apps/
│   │   │   ├── server/   # Node.js server (MCP bridge, token minting)
│   │   │   ├── web/      # Vite PWA (voice UI, text input, Task Inbox)
│   │   │   └── native/   # Electron wrapper (optional)
│   │   └── packages/
│   │       ├── core/     # Session management, voice engine
│   │       └── data/     # Local storage (IndexedDB)
│   │
│   └── zerg/             # Agent orchestration backend
│       ├── backend/      # FastAPI + LangGraph + APScheduler
│       ├── frontend/     # Rust/WASM UI (legacy)
│       ├── frontend-web/ # React UI (primary)
│       └── e2e/          # E2E tests
│
├── packages/
│   ├── contracts/        # Generated OpenAPI/AsyncAPI clients
│   └── tool-manifest/    # Shared MCP tool definitions
│
├── docs/
│   ├── jarvis_integration.md
│   └── architecture.md
│
├── Makefile              # Unified dev commands
├── package.json          # npm workspace configuration
└── .env                  # Unified environment config
```

## Development Commands

### Individual Apps
```bash
make jarvis-dev    # Start Jarvis only (PWA + node server)
make zerg-dev      # Start Zerg only (FastAPI backend + frontend)
make swarm-dev     # Start BOTH Jarvis and Zerg
make stop          # Stop all services
```

### Testing
```bash
make test          # Run all tests (Jarvis + Zerg)
make test-jarvis   # Test Jarvis only
make test-zerg     # Test Zerg only
```

### Code Generation
```bash
make generate-sdk          # Generate TypeScript clients from OpenAPI
make seed-jarvis-agents    # Seed baseline Zerg agents
make validate-contracts    # Validate API contracts
```

## Integration Flow

1. **User** speaks or types to Jarvis
2. **Jarvis** handles quick LLM replies locally
3. For tasks requiring scheduling or multi-step action, Jarvis → `POST /api/jarvis/dispatch`
4. **Zerg** enqueues run via `execute_agent_task` (locking, run rows, events)
5. **Event bus** pushes updates; `/api/jarvis/events` SSE streams statuses
6. **Jarvis Task Inbox** updates UI and optionally speaks the result
7. **Scheduled agents** run autonomously; SSE stream updates Jarvis automatically

## API Endpoints

### Jarvis → Zerg Integration
- `POST /api/jarvis/auth` - Authenticate device, get JWT
- `POST /api/jarvis/dispatch` - Dispatch agent task
- `GET /api/jarvis/agents` - List available agents
- `GET /api/jarvis/runs` - Get recent agent runs
- `GET /api/jarvis/events` - SSE feed of agent updates

## Environment Variables

See `.env.example.swarm` for full configuration options.

Key variables:
- `JARVIS_OPENAI_API_KEY` - OpenAI API key for Jarvis
- `JARVIS_DEVICE_SECRET` - Device auth secret
- `JARVIS_ZERG_API_URL` - Zerg backend URL
- `OPENAI_API_KEY` - OpenAI API key for Zerg agents
- `DATABASE_URL` - Database connection string
- `JWT_SECRET` - JWT signing secret

## Architecture

### Jarvis Features
- **Voice mode**: OpenAI Realtime WebRTC
- **Text mode**: REST fallback for typed queries
- **Task Inbox**: Side panel showing AgentRun entries (status, summary)
- **Context switcher**: Personal/work persona support
- **PWA features**: Offline shell, home-screen icon, notifications

### Zerg Features
- **FastAPI routers**: Agents, runs, threads, jarvis dispatch
- **LangGraph workflows**: Complex multi-step tasks
- **APScheduler**: Cron for daily digests, health snapshots
- **Event bus**: Real-time run updates
- **MCP aggregator**: Tool integrations (WHOOP, Traccar, Gmail, Slack, etc.)
- **Historical storage**: Cost/usage metrics, audit logs

## Contributing

See individual app READMEs for detailed development instructions:
- [Jarvis README](apps/jarvis/README.md)
- [Zerg README](apps/zerg/README.md)

## License

See LICENSE file.
