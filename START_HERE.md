# 👋 Welcome to the Swarm Platform

**Branch**: `jarvis-integration`
**Status**: Backend Complete ✅ | Ready for UI Integration 🚀

---

## 🎯 What Is This?

The **Swarm Platform** unifies **Jarvis** (your voice/text PWA) with **Zerg** (agent orchestration backend) into a single integrated system.

```
You speak → Jarvis listens → Zerg executes → Results stream back
```

---

## 🚀 Quick Start (3 Commands)

```bash
# 1. Validate everything is set up
./scripts/validate-setup.sh

# 2. Configure environment
cp .env.example.swarm .env && nano .env

# 3. Start the platform
make swarm-dev
```

Then open: http://localhost:8080 (Jarvis) and http://localhost:47300 (Zerg API)

---

## 📚 Documentation Map

### **Start Here** (First Time)
1. **HANDOFF.md** ← Read this first for this session's work
2. **SWARM_INTEGRATION_COMPLETE.md** ← Comprehensive overview

### **Development** (Daily Use)
- **README.swarm.md** - Quick start, architecture, commands
- **Makefile** - Run `make help` to see all commands

### **API Integration** (Implementing UI)
- **docs/jarvis_integration.md** - Complete API reference
- **apps/jarvis/apps/web/lib/task-inbox-integration-example.ts** - Code examples

### **Operations** (Deployment)
- **docs/DEPLOYMENT.md** - Production deployment guide
- **.env.example.swarm** - Environment variable template

### **Tool Management** (Adding MCP Tools)
- **docs/tool_manifest_workflow.md** - Complete workflow guide

---

## 🧪 Testing

```bash
# Validate setup
./scripts/validate-setup.sh

# Test backend APIs
./scripts/test-jarvis-integration.sh

# Run all tests
make test
```

---

## 🎯 Next Steps (2-3 Hours to MVP)

### Hour 1: Task Inbox
Add to `apps/jarvis/apps/web/main.ts`:
```typescript
import { createTaskInbox } from './lib/task-inbox';

await createTaskInbox(document.getElementById('task-inbox'), {
  apiURL: import.meta.env.VITE_ZERG_API_URL,
  deviceSecret: import.meta.env.VITE_JARVIS_DEVICE_SECRET,
});
```

### Hour 2: Text Mode
Add text input field, wire to dispatch

### Hour 3: Voice Commands
Map voice intent → agent dispatch

**See HANDOFF.md for detailed instructions with code examples.**

---

## 🆘 If Something's Wrong

1. **Setup issues**: Run `./scripts/validate-setup.sh`
2. **API errors**: Run `./scripts/test-jarvis-integration.sh`
3. **Need examples**: Check `docs/jarvis_integration.md`
4. **Deployment help**: Check `docs/DEPLOYMENT.md`

---

## 📊 What's Complete

- ✅ Monorepo structure (Jarvis + Zerg in `/apps`)
- ✅ 5 REST API endpoints
- ✅ Real-time SSE event streaming
- ✅ 4 baseline agents ready to use
- ✅ TypeScript API client
- ✅ Task Inbox component (ready to integrate)
- ✅ Tool manifest system
- ✅ Integration tests
- ✅ Deployment guide
- ✅ 2,000+ lines of documentation

---

## 🎓 Key Concepts

### Commands
- `make swarm-dev` - Start Jarvis + Zerg
- `make jarvis-dev` - Start Jarvis only
- `make zerg-dev` - Start Zerg only
- `make seed-jarvis-agents` - Create baseline agents
- `make generate-tools` - Regenerate tool manifest

### Structure
- `apps/jarvis/` - Voice/text PWA
- `apps/zerg/` - Agent orchestration backend
- `packages/` - Shared code (API clients, tools)
- `scripts/` - Automation and testing
- `docs/` - All documentation

### Agents (After Seeding)
1. **Morning Digest** (7 AM) - Health + calendar + weather
2. **Health Watch** (8 PM) - WHOOP trends
3. **Weekly Planning** (Sundays) - Week ahead
4. **Quick Status** (on-demand) - Fast status check

---

## 💡 Pro Tip

**Read HANDOFF.md first** - it has:
- Complete session summary
- Step-by-step UI integration guide
- Code examples you can copy-paste
- Testing instructions
- Troubleshooting tips

Everything else branches from there.

---

**Ready to continue? Open HANDOFF.md and let's finish this! 🚀**
