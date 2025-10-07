// Jarvis PWA with OpenAI Agents SDK - Enhanced with conversation persistence
import { RealtimeAgent, RealtimeSession, OpenAIRealtimeWebRTC } from '@openai/agents/realtime';
import { tool } from '@openai/agents';
import { z } from 'zod';
import { SessionManager, logger, getJarvisClient, type JarvisAgentSummary } from '@jarvis/core';
import { ConversationUI } from './lib/conversation-ui';
import { ConversationRenderer } from './lib/conversation-renderer';
import { createTaskInbox, type TaskInbox } from './lib/task-inbox';
import type { ConversationTurn, ConversationManagerOptions, SyncTransport } from '@jarvis/data-local';
import { contextLoader } from './contexts/context-loader';
import type { VoiceAgentConfig } from './contexts/types';
import { uiEnhancements } from './lib/ui-enhancements';
import { RadialVisualizer } from './lib/radial-visualizer';

// Configuration
const CONFIG = {
  API_BASE: window.location.hostname === 'localhost' 
    ? 'http://localhost:8787' 
    : `${window.location.protocol}//${window.location.hostname}:8787`
};

function resolveSyncBaseUrl(raw?: string): string {
  const fallback = CONFIG.API_BASE;
  if (!raw) return fallback;

  const trimmed = raw.trim();
  if (trimmed === '' || trimmed.toLowerCase() === 'auto') {
    return fallback;
  }

  if (/^https?:\/\//i.test(trimmed)) {
    return trimmed.replace(/\/$/, '');
  }

  if (trimmed.startsWith('//')) {
    return `${window.location.protocol}${trimmed}`.replace(/\/$/, '');
  }

  if (/^[\w.-]+:\d+$/.test(trimmed)) {
    return `${window.location.protocol}//${trimmed}`.replace(/\/$/, '');
  }

  if (trimmed.startsWith('/')) {
    return `${window.location.origin}${trimmed}`.replace(/\/$/, '');
  }

  try {
    return new URL(trimmed, window.location.origin).toString().replace(/\/$/, '');
  } catch (error) {
    logger.warn('Invalid sync base URL, using fallback', { provided: trimmed, error });
    return fallback;
  }
}

function createSyncTransport(headers?: Record<string, string>): SyncTransport {
  return async (input, init = {}) => {
    if (typeof fetch === 'undefined') {
      throw new Error('Fetch is not available for sync transport');
    }

    const mergedHeaders = {
      ...(headers ?? {}),
      ...(init.headers ?? {})
    };

    return fetch(input, { ...init, headers: mergedHeaders });
  };
}

function buildConversationManagerOptions(config: VoiceAgentConfig): ConversationManagerOptions {
  const syncBaseUrl = resolveSyncBaseUrl(config.sync?.baseUrl);
  const syncTransport = createSyncTransport(config.sync?.headers);

  return {
    syncBaseUrl,
    syncTransport
  };
}

function createSessionManagerForContext(config: VoiceAgentConfig): SessionManager {
  return new SessionManager({}, {
    conversationManagerOptions: buildConversationManagerOptions(config),
    maxHistoryTurns: config.settings.maxHistoryTurns
  });
}

// Global state
let agent: RealtimeAgent | null = null;
let session: RealtimeSession | null = null;
let sessionManager: SessionManager | null = null;
let conversationUI: ConversationUI | null = null;
let conversationRenderer: ConversationRenderer | null = null;
// Removed: currentStreamingTurn - now using currentStreamingMessageId with ConversationRenderer
let currentStreamingText = '';
let currentConversationId: string | null = null;
let isConnected = false;
let isConnecting = false; // guard concurrent connects
let currentContext: VoiceAgentConfig | null = null;
// Track a pending user bubble while transcription completes
// Legacy DOM element placeholders removed in favor of renderer-based state
// (avoid multiple writers / race conditions)

// Jarvis-Zerg integration
let taskInbox: TaskInbox | null = null;
let jarvisClient = getJarvisClient(import.meta.env.VITE_ZERG_API_URL || 'http://localhost:47300');
let cachedAgents: JarvisAgentSummary[] = [];
// Track if transcript is showing status text (e.g., "Connecting…")
let statusActive = false;
// Accumulate partial transcription text when VAD is used (no PTT)
let pendingUserText = '';

// DOM elements
const transcriptEl = document.getElementById("transcript") as HTMLDivElement;

// Initialize conversation renderer
conversationRenderer = new ConversationRenderer(transcriptEl);
const toggleConnectionBtn = document.getElementById("toggleConnectionBtn") as HTMLButtonElement;
const pttBtn = document.getElementById("pttBtn") as HTMLButtonElement;
const newConversationBtn = document.getElementById("newConversationBtn") as HTMLButtonElement;
const clearConvosBtn = document.getElementById("clearConvosBtn") as HTMLButtonElement;
const syncNowBtn = document.getElementById("syncNowBtn") as HTMLButtonElement;
const sidebarToggle = document.getElementById("sidebarToggle") as HTMLButtonElement;
const sidebar = document.getElementById("sidebar") as HTMLDivElement;
const voiceButtonContainer = document.getElementById('voiceButtonContainer') as HTMLDivElement;

const remoteAudio = document.getElementById('remoteAudio') as HTMLAudioElement | null;
let sharedMicStream: MediaStream | null = null;

enum AudioState {
  IDLE = 0,
  MIC_ACTIVE = 1 << 0,
  ASSISTANT_SPEAKING = 1 << 1,
}

const AUDIO_STATE_CLASSNAMES = ['audio-state-0', 'audio-state-1', 'audio-state-2', 'audio-state-3'];

let audioState: AudioState = AudioState.IDLE;
let micLevel = 0;
let speakerLevel = 0;
let speakerAudioCtx: AudioContext | null = null;
let speakerAnalyser: AnalyserNode | null = null;
let speakerDataArray: Uint8Array | null = null;
let speakerSource: MediaStreamAudioSourceNode | MediaElementAudioSourceNode | null = null;
let speakerStream: MediaStream | null = null;
let speakerRafId: number | null = null;
let speakerSilenceFrames = 0;
let speakerMonitorUnavailable = false;

const MIC_COLOR = '#ec4899';
const ASSISTANT_COLOR = '#3b82f6';
const BOTH_COLOR = '#a855f7';
const IDLE_COLOR = '#475569';

// Audio visualizer (radial) around mic
const radialViz = voiceButtonContainer
  ? new RadialVisualizer(voiceButtonContainer, { onLevel: handleMicLevel })
  : null;

syncAudioStateClasses();
updateAudioVisualization();

// Small UI helpers (dedup repeated blocks)
const MIC_ICON = `
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"/>
    <path d="M19 10v2a7 7 0 01-14 0v-2M12 19v4M8 23h8"/>
  </svg>
`;
const STOP_ICON = `
  <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
    <rect x="6" y="6" width="12" height="12" rx="2"/>
  </svg>
`;

function setMicIcon(listening: boolean) {
  pttBtn.innerHTML = listening ? STOP_ICON : MIC_ICON;
}

function hasAudioState(flag: AudioState): boolean {
  return (audioState & flag) === flag;
}

function syncAudioStateClasses(): void {
  const classList = document.body.classList;
  for (const cls of AUDIO_STATE_CLASSNAMES) {
    classList.remove(cls);
  }
  classList.add(`audio-state-${audioState}`);
}

function updateAudioVisualization(): void {
  const micActive = hasAudioState(AudioState.MIC_ACTIVE);
  const assistantActive = hasAudioState(AudioState.ASSISTANT_SPEAKING);

  let level = 0;
  if (micActive && assistantActive) {
    level = Math.max(micLevel, speakerLevel);
  } else if (micActive) {
    level = micLevel;
  } else if (assistantActive) {
    level = speakerLevel;
  }

  const color = micActive && assistantActive
    ? BOTH_COLOR
    : micActive
      ? MIC_COLOR
      : assistantActive
        ? ASSISTANT_COLOR
        : IDLE_COLOR;

  if (radialViz) {
    radialViz.render(level, color, audioState);
  }

  if (voiceButtonContainer) {
    voiceButtonContainer.dataset.state = `${audioState}`;
    voiceButtonContainer.classList.toggle('audio-mic-active', micActive);
    voiceButtonContainer.classList.toggle('audio-assistant-active', assistantActive);
  }
}

function setAudioStateValue(next: AudioState): void {
  if (audioState === next) return;
  audioState = next;
  syncAudioStateClasses();
  updateAudioVisualization();
}

function setAudioStateFlag(flag: AudioState, enabled: boolean): void {
  const next = enabled ? (audioState | flag) : (audioState & ~flag);
  setAudioStateValue(next as AudioState);
}

function handleMicLevel(level: number): void {
  micLevel = level;
  updateAudioVisualization();
}

function handleSpeakerLevel(level: number): void {
  speakerLevel = level;
  updateAudioVisualization();
}

function setAssistantSpeakingState(active: boolean): void {
  setAudioStateFlag(AudioState.ASSISTANT_SPEAKING, active);
}

async function startSpeakerMonitor(): Promise<void> {
  if (!remoteAudio || speakerMonitorUnavailable) return;

  try {
    if (!speakerAudioCtx) {
      speakerAudioCtx = new (window.AudioContext || (window as any).webkitAudioContext)();
    }

    if (speakerAudioCtx.state === 'suspended') {
      await speakerAudioCtx.resume().catch(() => undefined);
    }

    if (!speakerSource) {
      try {
        if (typeof (remoteAudio as any).captureStream === 'function') {
          speakerStream = (remoteAudio as any).captureStream();
        }
      } catch (error) {
        logger.warn('Failed to capture remote audio stream', error);
        speakerStream = null;
      }

      if (speakerStream) {
        speakerSource = speakerAudioCtx.createMediaStreamSource(speakerStream);
      } else {
        speakerSource = speakerAudioCtx.createMediaElementSource(remoteAudio);
      }
    }

    if (!speakerAnalyser) {
      speakerAnalyser = speakerAudioCtx.createAnalyser();
      speakerAnalyser.fftSize = 1024;
      speakerAnalyser.smoothingTimeConstant = 0.7;
      speakerSource.connect(speakerAnalyser);
      speakerDataArray = new Uint8Array(speakerAnalyser.fftSize);
    }

    if (speakerRafId == null) {
      speakerSilenceFrames = 0;
      monitorSpeakerLevel();
    }
  } catch (error) {
    speakerMonitorUnavailable = true;
    logger.warn('Speaker visualizer unavailable', error);
  }
}

function stopSpeakerMonitor(): void {
  if (speakerRafId != null) {
    cancelAnimationFrame(speakerRafId);
    speakerRafId = null;
  }
  speakerSilenceFrames = 0;
  speakerLevel = 0;

  if (!hasAudioState(AudioState.MIC_ACTIVE)) {
    setAssistantSpeakingState(false);
  } else {
    updateAudioVisualization();
  }
}

function monitorSpeakerLevel(): void {
  if (!speakerAnalyser || !speakerDataArray) {
    speakerRafId = null;
    return;
  }

  speakerAnalyser.getByteTimeDomainData(speakerDataArray as Uint8Array<ArrayBuffer>);

  let sumSquares = 0;
  for (let i = 0; i < speakerDataArray.length; i++) {
    const centered = (speakerDataArray[i] - 128) / 128;
    sumSquares += centered * centered;
  }

  const rms = Math.sqrt(sumSquares / speakerDataArray.length);
  const level = Math.min(1, rms * 2.8);
  handleSpeakerLevel(level);

  if (level > 0.035) {
    speakerSilenceFrames = 0;
    if (!hasAudioState(AudioState.ASSISTANT_SPEAKING)) {
      setAssistantSpeakingState(true);
    }
  } else if (speakerSilenceFrames < 24) {
    speakerSilenceFrames += 1;
    if (speakerSilenceFrames === 24 && hasAudioState(AudioState.ASSISTANT_SPEAKING)) {
      setAssistantSpeakingState(false);
    }
  }

  speakerRafId = requestAnimationFrame(monitorSpeakerLevel);
}

if (remoteAudio) {
  const handleSpeakerStart = () => { void startSpeakerMonitor(); };
  const handleSpeakerStop = () => { stopSpeakerMonitor(); };

  remoteAudio.addEventListener('play', handleSpeakerStart);
  remoteAudio.addEventListener('playing', handleSpeakerStart);
  remoteAudio.addEventListener('loadeddata', handleSpeakerStart);
  remoteAudio.addEventListener('pause', handleSpeakerStop);
  remoteAudio.addEventListener('ended', handleSpeakerStop);
  remoteAudio.addEventListener('emptied', handleSpeakerStop);
  remoteAudio.addEventListener('suspend', handleSpeakerStop);
  remoteAudio.addEventListener('stalled', handleSpeakerStop);
}

async function setListeningMode(active: boolean) {
  if (!active) {
    micLevel = 0;
  }

  setAudioStateFlag(AudioState.MIC_ACTIVE, active);

  if (active) {
    document.body.classList.add('listening-mode');
    try {
      await radialViz?.start();
    } catch (error) {
      logger.warn('Failed to start radial visualizer', error);
    }
  } else {
    document.body.classList.remove('listening-mode');
    radialViz?.stop();
    updateAudioVisualization();
  }
}

function setInputAudio(enabled: boolean) {
  if (session && (session as any).inputAudioEnabled !== undefined) {
    (session as any).inputAudioEnabled = enabled;
  }
}

async function setMicState(active: boolean) {
  if (active) {
    pttBtn.classList.add('listening');
  } else {
    pttBtn.classList.remove('listening');
  }
  setMicIcon(active);
  pttBtn.setAttribute('aria-pressed', active ? 'true' : 'false');
  pttBtn.setAttribute('aria-label', active ? 'Stop listening' : 'Push to talk');
  await setListeningMode(active);
  setInputAudio(active);
}

// Remove PWA service worker registration; proactively unregister any past SWs
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.getRegistrations()
    .then(regs => regs.forEach(r => r.unregister()))
    .catch(() => {});
}

// Utility functions: delegate status messages to ConversationRenderer
function setTranscript(msg: string, muted = false): void {
  if (currentStreamingMessageId) return; // Don't override streaming content
  if (conversationRenderer) conversationRenderer.setStatus(msg, muted);
  statusActive = true;
}

function clearStatusIfActive(): void {
  if (!statusActive) return;
  if (conversationRenderer) conversationRenderer.clearStatus();
  statusActive = false;
}

function updateSidebarToggleState(isOpen: boolean): void {
  sidebarToggle.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
  sidebarToggle.setAttribute('aria-label', isOpen ? 'Close conversation sidebar' : 'Open conversation sidebar');
}

// Show a banner only when there are no turns rendered
function ensureReadyBanner(message: string): void {
  const hasTurns = conversationRenderer?.hasTurns() ?? false;
  if (!hasTurns) {
    setTranscript(message, true);
  }
}

// Ensure a pending "You" bubble exists using ConversationRenderer
let pendingUserMessageId: string | null = null;

function ensurePendingUserBubble(): void {
  if (!conversationRenderer) return;

  clearStatusIfActive();

  if (!pendingUserMessageId) {
    pendingUserMessageId = `pending-user-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    // Reset pending text for a fresh placeholder
    pendingUserText = '';

    conversationRenderer.addMessage({
      id: pendingUserMessageId,
      role: 'user',
      content: 'Listening…',
      timestamp: new Date()
    });
  }
}

// Update the pending bubble with partial transcript
function updatePendingUserPlaceholder(delta: string): void {
  if (!conversationRenderer) return;

  ensurePendingUserBubble();
  pendingUserText += String(delta);

  if (pendingUserMessageId) {
    conversationRenderer.updateMessage(pendingUserMessageId, {
      content: pendingUserText.trim()
    });
  }
}

async function getSessionToken(): Promise<string> {
  console.log('📡 Fetching session token from:', `${CONFIG.API_BASE}/session`);

  try {
    const r = await fetch(`${CONFIG.API_BASE}/session`);
    console.log('📡 Response status:', r.status);

    if (!r.ok) {
      const errorText = await r.text();
      console.error('Session request failed:', r.status, errorText);
      if (r.status === 0 || !r.status) {
        throw new Error('Cannot connect to voice server - is it running?');
      }
      throw new Error(`Failed to get session token: ${r.status} ${errorText}`);
    }

    const js = await r.json();
    console.log('📡 Session response:', js);

    const token = js.value || js.client_secret?.value;
    if (!token) {
      console.error('No token found in response:', js);
      throw new Error('Invalid session response format - missing token value');
    }

    console.log('✅ Session token obtained');
    return token;
  } catch (error) {
    if (error instanceof TypeError && error.message.includes('fetch')) {
      console.error('❌ Network error - server unreachable');
      throw new Error('Cannot connect to voice server - check if server is running on port 8787');
    }
    throw error;
  }
}

// Context-specific tool creation
function createContextTools(config: VoiceAgentConfig): any[] {
  const tools: any[] = [];
  
  for (const toolConfig of config.tools) {
    if (!toolConfig.enabled) continue;
    
    let tool = null;
    if (toolConfig.mcpServer && toolConfig.mcpFunction) {
      // MCP tools (can be in any context)
      tool = createMCPTool(toolConfig);
    } else if (toolConfig.ragDatabase && toolConfig.ragCollection) {
      // RAG tools (work context specific)  
      tool = createRAGTool(toolConfig);
    }
    
    if (tool) {
      tools.push(tool);
    }
  }
  
  return tools;
}

function createMCPTool(toolConfig: any): any {
  if (toolConfig.name === 'get_current_location') {
    return locationTool;
  } else if (toolConfig.name === 'get_whoop_data') {
    return whoopTool;
  }
  // Add more MCP tools as needed
  return null;
}

function createRAGTool(toolConfig: any): any {
  const baseExecute = async ({ query, category }: { query: string, category?: string }) => {
    console.log(`🔍 ${toolConfig.name}:`, query, category);
    try {
      if (!sessionManager) {
        return 'RAG search not available - session not initialized';
      }
      
      const searchOptions: any = { limit: 3 };
      if (category && category !== 'any') {
        searchOptions.type = category as 'financial' | 'product' | 'policy' | 'organizational' | 'strategic';
      }
      
      const results = await sessionManager.searchDocuments(query, searchOptions);
      
      if (results.length === 0) {
        return `No company information found for "${query}"`;
      }
      
      let response = `Found ${results.length} relevant company documents:\n\n`;
      results.forEach((result: any, i: number) => {
        const doc = result.document;
        response += `${i + 1}. **${doc.metadata.type.toUpperCase()}** (relevance: ${(result.score * 100).toFixed(1)}%)\n`;
        response += `   ${doc.content}\n`;
        response += `   Source: ${doc.metadata.source}\n\n`;
      });
      
      return response;
    } catch (error) {
      console.error(`${toolConfig.name} failed:`, error);
      return `Search failed: ${error instanceof Error ? error.message : 'Unknown error'}`;
    }
  };

  // Create tool based on config name
  if (toolConfig.name === 'search_company_knowledge') {
    return tool({
      name: 'search_company_knowledge',
      description: 'Search company documentation, policies, and business data',
      parameters: z.object({
        query: z.string().describe('Search query for company information'),
        category: z.string().describe('Category to filter by: any, financial, product, policy, organizational, strategic').default('any')
      }),
      execute: baseExecute
    });
  } else if (toolConfig.name === 'get_financial_data') {
    return tool({
      name: 'get_financial_data',
      description: 'Access financial reports and business metrics',
      parameters: z.object({
        query: z.string().describe('Query for financial data (revenue, profits, Q3 results, etc.)'),
        category: z.string().describe('Category: any, financial').default('financial')
      }),
      execute: baseExecute
    });
  } else if (toolConfig.name === 'search_team_info') {
    return tool({
      name: 'search_team_info', 
      description: 'Find team member information and organizational data',
      parameters: z.object({
        query: z.string().describe('Query for team/organizational info'),
        category: z.string().describe('Category: any, organizational').default('organizational')
      }),
      execute: baseExecute
    });
  }
  
  console.warn(`Unknown RAG tool: ${toolConfig.name}`);
  return null;
}

// MCP Tool Definitions using Agents SDK tool() function
const locationTool = tool({
  name: 'get_current_location', 
  description: 'Get current GPS location with coordinates and address. Call this whenever the user asks about their location.',
  parameters: z.object({}),
  async execute() {
    console.log('📍 Calling location tool');
    try {
      const response = await fetch(`${CONFIG.API_BASE}/tool`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          name: 'location.get_current', 
          args: { include_address: true } 
        })
      });
      
      if (!response.ok) {
        throw new Error(`Location API failed: ${response.status}`);
      }
      
      const data = await response.json();
      console.log('📍 Location data received:', data);
      
      // Format the response for the voice agent
      if (data.error) {
        return `Location error: ${data.error}`;
      }
      
      const loc = Array.isArray(data) ? data[0] : data;
      if (!loc) {
        return "No location data available";
      }
      
      let result = `Current location: ${loc.lat?.toFixed(4)}, ${loc.lon?.toFixed(4)}`;
      if (loc.address) {
        result += ` (${loc.address})`;
      }
      if (loc.accuracy) {
        result += `, accuracy: ${loc.accuracy}m`;
      }
      if (loc.timestamp) {
        const time = new Date(loc.timestamp).toLocaleTimeString();
        result += `, updated: ${time}`;
      }
      
      return result;
    } catch (error) {
      console.error('Location tool error:', error);
      return `Failed to get location: ${error instanceof Error ? error.message : 'Unknown error'}`;
    }
  }
});

const whoopTool = tool({
  name: 'get_whoop_recovery',
  description: 'Get current WHOOP recovery score and sleep data',
  parameters: z.object({
    date: z.string().describe('Date in YYYY-MM-DD format, defaults to today')
  }),
  async execute({ date }) {
    console.log('🏃 Calling WHOOP tool with date:', date);
    try {
      const response = await fetch(`${CONFIG.API_BASE}/tool`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          name: 'whoop.get_daily', 
          args: { date } 
        })
      });
      
      if (!response.ok) {
        throw new Error(`WHOOP API failed: ${response.status}`);
      }
      
      const data = await response.json();
      console.log('💪 WHOOP data received:', data);
      
      return `Your WHOOP data for ${data.date}: Recovery Score: ${data.recovery_score}%, Sleep: ${data.sleep.duration} hours, HRV: ${data.sleep.hrv}, RHR: ${data.sleep.rhr}`;
    } catch (error) {
      console.error('WHOOP tool error:', error);
      return `Sorry, couldn't get your WHOOP data: ${error instanceof Error ? error.message : 'Unknown error'}`;
    }
  }
});

// Enhanced streaming UI functions - now using ConversationRenderer
let currentStreamingMessageId: string | null = null;

function startStreamingResponse(): void {
  logger.debug('Starting streaming response');

  if (!conversationRenderer) return;

  // Clear status if currently showing (e.g., "Connecting…")
  clearStatusIfActive();

  // Create streaming message through renderer instead of direct DOM manipulation
  currentStreamingMessageId = `streaming-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  currentStreamingText = '';

  conversationRenderer.addMessage({
    id: currentStreamingMessageId,
    role: 'assistant',
    content: '',
    timestamp: new Date(),
    isStreaming: true
  });
}

function updateStreamingText(): void {
  if (!conversationRenderer || !currentStreamingMessageId) return;

  // Update streaming message content through renderer
  conversationRenderer.updateMessage(currentStreamingMessageId, {
    content: currentStreamingText,
    isStreaming: true
  });
}

function finalizeStreamingResponse(): void {
  if (!conversationRenderer || !currentStreamingMessageId) return;

  logger.streamingResponse(currentStreamingText, true);

  // Finalize streaming message through renderer
  conversationRenderer.updateMessage(currentStreamingMessageId, {
    content: currentStreamingText,
    isStreaming: false
  });

  // Record the finalized streaming response to IndexedDB
  if (currentStreamingText) {
    recordConversationTurn('assistant', currentStreamingText);
  }

  // Clean up
  currentStreamingMessageId = null;
  currentStreamingText = '';
}


function addToolCallToUI(toolName: string, result: any): void {
  if (!conversationRenderer) return;

  const messageId = `tool-${Date.now()}-${Math.random().toString(36).slice(2)}`;

  conversationRenderer.addMessage({
    id: messageId,
    role: 'assistant',
    content: `🔧 Tool: ${toolName}\n${result}`,
    timestamp: new Date()
  });
}

async function recordConversationTurn(type: string, content: string): Promise<void> {
  if (!sessionManager) return;
  
  try {
    const turn: ConversationTurn = {
      id: crypto.randomUUID(),
      timestamp: new Date(),
      conversationId: currentConversationId || undefined,
      ...(type === 'user' 
        ? { userTranscript: content } 
        : { assistantResponse: content }
      )
    };
    
    await sessionManager.addConversationTurn(turn);
    logger.debug('Recorded conversation turn', `${type}: ${content.substring(0, 50)}...`);
  } catch (error) {
    console.error('Failed to record conversation turn:', error);
  }
}

// Load and display conversation history in UI using ConversationRenderer
async function loadConversationHistoryIntoUI(): Promise<void> {
  if (!sessionManager || !currentConversationId || !conversationRenderer) {
    if (conversationRenderer) {
      conversationRenderer.clear();
    }
    if (conversationRenderer) conversationRenderer.setStatus('Click Connect to start', true);
    return;
  }

  try {
    const history = await sessionManager.getConversationHistory();

    if (history.length === 0) {
      conversationRenderer.clear();
      conversationRenderer.setStatus('No messages yet - click Connect to start', true);
      return;
    }

    // Use renderer's loadFromHistory method - this ensures proper ordering
    conversationRenderer.loadFromHistory(history);

    logger.conversation('Loaded conversation turns into UI', history.length);
  } catch (error) {
    console.error('Failed to load conversation history:', error);
    if (conversationRenderer) {
      conversationRenderer.clear();
      conversationRenderer.setStatus('Failed to load conversation history', true);
    }
  }
}

// Enhanced UI functions - now using ConversationRenderer (single DOM mutation point)
function addUserTurnToUI(transcript: string, timestamp?: Date): void {
  if (!conversationRenderer) return;

  // Remove status placeholder if present
  clearStatusIfActive();

  const messageTimestamp = timestamp || new Date();
  const messageId = `user-${Date.now()}-${Math.random().toString(36).slice(2)}`;

  conversationRenderer.addMessage({
    id: messageId,
    role: 'user',
    content: transcript,
    timestamp: messageTimestamp
  });

  // Record to IndexedDB if not from history loading (no timestamp provided)
  if (!timestamp) {
    recordConversationTurn('user', transcript);
  }
}

function addAssistantTurnToUI(response: string, timestamp?: Date): void {
  if (!conversationRenderer) return;

  const messageTimestamp = timestamp || new Date();
  const messageId = `assistant-${Date.now()}-${Math.random().toString(36).slice(2)}`;

  conversationRenderer.addMessage({
    id: messageId,
    role: 'assistant',
    content: response,
    timestamp: messageTimestamp
  });

  // Record to IndexedDB if not from history loading (no timestamp provided)
  if (!timestamp) {
    recordConversationTurn('assistant', response);
  }
}

// Main connection function using Agents SDK
async function connect(): Promise<void> {
  if (isConnecting) return;
  console.log('🔗 Connect starting...');

  let loadingOverlay: HTMLDivElement | null = null;
  try {
    isConnecting = true;
    toggleConnectionBtn.disabled = true;
    loadingOverlay = uiEnhancements.showLoading('Connecting to voice service...');

    // Use the context-aware agent created during initialization
    if (!agent) {
      console.error('❌ Agent not initialized');
      throw new Error('Agent not initialized - context system may have failed');
    }

    console.log('✅ Agent found:', agent.name || 'unnamed agent');

    // Request mic once and share it between transport and visualizer
    if (!sharedMicStream) {
      sharedMicStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          channelCount: 1
        }
      });
    }
    radialViz?.provideStream(sharedMicStream);

    // Create session
    const transport = new OpenAIRealtimeWebRTC({
      mediaStream: sharedMicStream || undefined,
      audioElement: remoteAudio || undefined,
    });

    session = new RealtimeSession(agent, {
      transport,
      model: 'gpt-realtime',
      config: {
        inputAudioTranscription: { model: 'whisper-1' },
        audio: {
          output: {
            voice: 'verse',
            speed: 1.3  // 30% faster speech
          }
        },
        turnDetection: {
          type: 'server_vad',
          threshold: 0.5,
          prefix_padding_ms: 300,
          silence_duration_ms: 500
        }
      }
    });

    // Setup event listeners
    setupSessionEvents();

    // Get ephemeral token and connect
    console.log('🎫 Getting session token...');
    const token = await getSessionToken();
    console.log('🎫 Token received, connecting to OpenAI...');
    await session?.connect({ apiKey: token });
    console.log('🎉 Connection established!');
    
    logger.success('Connected successfully with Agents SDK');
    // Clear any lingering status text so UI is ready
    clearStatusIfActive();

    pttBtn.disabled = false;
    pttBtn.classList.remove('disabled');
    toggleConnectionBtn.classList.add('connected');
    isConnected = true;

    // Hide loading and show success
    if (loadingOverlay) uiEnhancements.hideLoading(loadingOverlay);
    uiEnhancements.showToast('Connected successfully', 'success');

  } catch (error) {
    logger.error('Connection failed', error);
    // Hide loading if it was created
    if (loadingOverlay) uiEnhancements.hideLoading(loadingOverlay);
    uiEnhancements.showToast(`Connection failed: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    toggleConnectionBtn.disabled = false;
  } finally {
    isConnecting = false;
  }
}

function setupSessionEvents(): void {
  // Transport events (raw Realtime API events)
  session?.on('transport_event', async (event: any) => {
    logger.transport(event.type);
    const t = event.type || '';

    // VAD speech start/stop → toggle listening visuals
    if (t.includes('input_audio_buffer') && t.includes('speech_started')) {
      await setListeningMode(true);
      ensurePendingUserBubble();
      return;
    }
    if (t.includes('input_audio_buffer') && (t.includes('speech_stopped') || t.includes('speech_ended') || t.includes('speech_end'))) {
      await setListeningMode(false);
      return;
    }
    // Partial transcription deltas
    if (t === 'conversation.item.input_audio_transcription.delta') {
      updatePendingUserPlaceholder(event.delta || '');
      return;
    }

    if (t.startsWith('response.output_audio')) {
      void startSpeakerMonitor();
    }

    switch (t) {
      case 'response.output_audio_transcript.delta':
      case 'response.output_text.delta':
        handleStreamingDelta(event.delta || '');
        break;
        
      case 'response.done':
        handleResponseComplete(event);
        break;
        
      case 'conversation.item.added':
        handleConversationItemAdded(event);
        break;
        
      case 'conversation.item.done':
        handleConversationItemDone(event);
        break;
        
      case 'conversation.item.input_audio_transcription.completed':
        handleUserTranscript(event.transcript || '');
        break;
    }
  });

  // History updates for conversation persistence
  session?.on('history_updated', async (history: any) => {
    logger.conversation('History updated', history.length);
    // Could sync with our conversation manager here
  });

  // Error handling
  session?.on('error', (error: any) => {
    logger.error('Session error', error);
    setTranscript(`Voice error: ${error.message}`, true);
    toggleConnectionBtn.disabled = false;
    toggleConnectionBtn.classList.remove('connected');
    pttBtn.disabled = true;
    isConnected = false;
  });
}

// Enhanced event handlers (preserve our streaming UI)
function handleStreamingDelta(delta: any): void {
  if (!delta) return;

  logger.delta(delta);
  
  if (!currentStreamingMessageId) {
    startStreamingResponse();
  }
  
  currentStreamingText += delta;
  updateStreamingText();
}

function handleResponseComplete(event: any): void {
  logger.debug('Response completed', event);
  
  if (currentStreamingMessageId && currentStreamingText) {
    finalizeStreamingResponse();
    
    // Note: recordConversationTurn is called inside finalizeStreamingResponse via addAssistantTurnToUI
  }
}

function handleConversationItemAdded(event: any): void {
  logger.debug('Conversation item added', event.item);
}

function handleConversationItemDone(event: any): void {
  logger.debug('Conversation item completed', event.item);
}

async function handleUserTranscript(transcript: any): Promise<void> {
  logger.debug('User transcript received', transcript);
  // If a renderer-based placeholder exists, finalize it; otherwise add new
  const finalText = String(transcript || pendingUserText).trim();

  // First, update the UI with the transcript
  if (pendingUserMessageId && conversationRenderer) {
    conversationRenderer.updateMessage(pendingUserMessageId, {
      content: finalText || '—',
    });
    // Persist finalized user turn
    recordConversationTurn('user', finalText);
    // Clear pending state
    pendingUserMessageId = null;
    pendingUserText = '';
  } else {
    addUserTurnToUI(finalText);
  }

  // Check if this is an agent dispatch command
  const agent = findAgentByIntent(finalText);
  if (agent && cachedAgents.length > 0) {
    console.log(`🎯 Voice command matched agent: ${agent.name}`);

    try {
      const result = await jarvisClient.dispatch({ agent_id: agent.id });
      console.log('✅ Agent dispatched from voice command:', result);

      // Add confirmation to UI
      addAssistantTurnToUI(`Started ${agent.name}. Check Task Inbox for results.`);
      uiEnhancements.showToast(`Running ${agent.name}...`, 'success');
    } catch (error: any) {
      console.error('Voice dispatch failed:', error);
      addAssistantTurnToUI(`Failed to start ${agent.name}: ${error.message}`);
      uiEnhancements.showToast('Agent dispatch failed', 'error');
    }
  }
}

// Disconnect function
async function disconnect() {
  console.log('🔌 Disconnecting from voice service...');

  await setMicState(false);
  pttBtn.disabled = true;

  try {
    if (session) {
      await (session as any).disconnect?.();
      console.log('✅ Disconnected successfully');
      uiEnhancements.showToast('Disconnected', 'info');
    }
  } catch (error) {
    logger.error('Disconnect error', error);
    console.error('❌ Disconnect error:', error);
  } finally {
    toggleConnectionBtn.disabled = false;
    toggleConnectionBtn.classList.remove('connected');
    isConnected = false;
    // Stop shared stream for privacy
    if (sharedMicStream) {
      sharedMicStream.getTracks().forEach(t => t.stop());
      sharedMicStream = null;
      radialViz?.provideStream(null);
    }
    stopSpeakerMonitor();
    if (speakerAudioCtx) {
      speakerAudioCtx.close().catch(() => undefined);
      speakerAudioCtx = null;
    }
    speakerAnalyser = null;
    speakerDataArray = null;
    speakerSource = null;
    speakerStream = null;
    speakerRafId = null;
    speakerMonitorUnavailable = false;
    speakerLevel = 0;
    micLevel = 0;
    setAudioStateValue(AudioState.IDLE);
  }
}

// Push-to-talk with SDK
let pushing = false;

// Voice button with modern animations
pttBtn.onpointerdown = async () => {
  if (!isConnected) {
    uiEnhancements.showToast('Click Connect to enable the microphone', 'info');
    return;
  }
  if (pttBtn.disabled) return;
  pushing = true;
  await setMicState(true);

  // Use ConversationRenderer for pending user bubble
  ensurePendingUserBubble();
};

pttBtn.onpointerup = pttBtn.onpointerleave = () => {
  if (!isConnected || pttBtn.disabled) return;
  pushing = false;
  setMicState(false);
};

// Event handlers will be set up after DOM is loaded

// New conversation handler (will be assigned later)
const handleNewConversation = async () => {
  if (!sessionManager || !conversationUI) return;
  
  try {
    // Disconnect current session if connected
    if (isConnected) {
      await disconnect();
    }
    
    if (conversationRenderer) {
      conversationRenderer.clear();
      conversationRenderer.setStatus('Starting new conversation...', true);
    }
    currentConversationId = await sessionManager.createNewConversation();
    console.log('📝 Started new conversation:', currentConversationId);
    
    // Update conversation UI
    await conversationUI.addNewConversation(currentConversationId);
    
    // Clear UI state for fresh start
    currentStreamingMessageId = null;
    currentStreamingText = '';
    
    // Load empty conversation history (will show "No messages yet")
    await loadConversationHistoryIntoUI();
  } catch (error) {
    console.error('Failed to create new conversation:', error);
    setTranscript("Failed to create new conversation.", true);
  }
};

// Clear all conversations handler (will be assigned later)
const handleClearConversations = async () => {
  if (!sessionManager || !conversationUI || !conversationRenderer) return;
  const confirmed = window.confirm('Delete ALL conversations? This cannot be undone. Documents will be preserved.');
  if (!confirmed) return;

  try {
    await sessionManager.clearAllConversations();
    await conversationUI.loadConversations();
    conversationUI.clearConversations();
    currentConversationId = null;
    currentStreamingMessageId = null;
    currentStreamingText = '';
    conversationRenderer.clear();
    conversationRenderer.setStatus('No conversations yet - start a new one', true);
    logger.success('All conversations cleared');
    uiEnhancements.showToast('All conversations cleared', 'success');
  } catch (error) {
    console.error('Failed to clear conversations:', error);
    setTranscript('Failed to clear conversations', true);
  }
};

// Initialize context-aware application
async function initializeApp(): Promise<void> {
  try {
    console.log('🚀 Initializing Jarvis voice agent...');
    console.log('📊 DOM elements found:', {
      toggleConnectionBtn: !!toggleConnectionBtn,
      pttBtn: !!pttBtn,
      transcript: !!transcriptEl,
      sidebar: !!sidebar,
      voiceButtonContainer: !!voiceButtonContainer
    });
    
    // Load the appropriate context
    const contextName = await contextLoader.autoDetectContext();
    currentContext = await contextLoader.loadContext(contextName);
    
    console.log(`✅ Loaded context: ${currentContext.name}`);
    console.log(`📋 Instructions: ${currentContext.instructions.substring(0, 100)}...`);
    
    // Update UI with context branding
    updateUIForContext(currentContext);
    
    // Initialize session manager with context-specific data loading
    sessionManager = createSessionManagerForContext(currentContext);
    currentConversationId = await sessionManager.initializeSession(currentContext, contextName);
    
    // Initialize conversation UI
    conversationUI = new ConversationUI();
    conversationUI.setSessionManager(sessionManager);
    await conversationUI.loadConversations();
    
    if (currentConversationId) {
      console.log(`🔄 Resumed conversation: ${currentConversationId}`);
      conversationUI.setActiveConversation(currentConversationId);
      await loadConversationHistoryIntoUI();
    } else {
      console.log(`🆕 No existing conversation - ready for new one`);
      setTranscript(`Ready to start new conversation - click Connect`, true);
    }
    
    console.log(`🧠 Session initialized with data loading complete`);
    
    // Create agent with context-specific configuration
    const contextTools = createContextTools(currentContext);
    agent = new RealtimeAgent({
      name: currentContext.name,
      instructions: currentContext.instructions,
      tools: contextTools
    });
    
    console.log(`🔧 Added ${contextTools.length} tools for ${currentContext.name}:`, contextTools.map(t => t.name));
    
    // Setup UI state
    pttBtn.disabled = true;
    toggleConnectionBtn.classList.remove('connected');
    newConversationBtn.disabled = false;
    
    // Create context selector in UI
    contextLoader.createContextSelector('context-selector-container');
    
    console.log(`✅ ${currentContext.name} voice agent ready`);
    ensureReadyBanner(`${currentContext.name} ready - click Connect to start`);
    
  } catch (error) {
    console.error('❌ Failed to initialize app:', error);
    setTranscript('Initialization failed - using default configuration', true);
    
    // Fallback to basic configuration
    agent = new RealtimeAgent({
      name: 'Voice Agent',
      instructions: 'You are a helpful AI assistant.'
    });
  }
}

function updateUIForContext(config: VoiceAgentConfig): void {
  // Update page title and branding
  document.title = config.branding.title;
  
  const titleEl = document.getElementById('appTitle');
  if (titleEl) {
    titleEl.textContent = config.branding.title;
  }
  
  // Update favicon if specified
  if (config.branding.favicon) {
    const favicon = document.querySelector('link[rel="icon"]') as HTMLLinkElement;
    if (favicon) {
      favicon.href = config.branding.favicon;
    }
  }
  
  // Update default prompts (could add quick action buttons)
  console.log(`💡 Default prompts for ${config.name}:`, config.settings.defaultPrompts);
}

// Conversation switching handler
window.addEventListener('conversationSwitched', async (event: any) => {
  const { conversationId } = event.detail;
  console.log(`🔄 Switching to conversation: ${conversationId}`);
  
  // Disconnect current session if connected
  if (isConnected) {
    await disconnect();
  }
  
  currentConversationId = conversationId;
  
  // Clear transcript and show loading via renderer
  if (conversationRenderer) {
    conversationRenderer.clear();
    conversationRenderer.setStatus('Loading conversation...', true);
  }
  
  // Load conversation history into UI
  await loadConversationHistoryIntoUI();
  
  console.log(`✅ Conversation ${conversationId} loaded and ready`);
});

// Context switching handler
window.addEventListener('contextChanged', async (event: any) => {
  const { contextName, config } = event.detail;
  console.log(`🔄 Context changed to: ${contextName}`);
  
  currentContext = config;
  updateUIForContext(config);
  
  // End current session and reinitialize with new context
  if (sessionManager) {
    await sessionManager.endSession();
  }

  sessionManager = createSessionManagerForContext(config);
  currentConversationId = await sessionManager.initializeSession(config, contextName);

  if (conversationUI) {
    conversationUI.setSessionManager(sessionManager);
    await conversationUI.loadConversations();

    if (currentConversationId) {
      conversationUI.setActiveConversation(currentConversationId);
      await loadConversationHistoryIntoUI();
      console.log(`🔄 Session reinitialized for ${config.name}, resumed conversation: ${currentConversationId}`);
    } else {
      setTranscript(`${config.name} - click Connect to start`, true);
      console.log(`🔄 Session reinitialized for ${config.name}, ready for new conversation`);
    }
  }
  
  // Update agent configuration with context-specific tools
  if (agent) {
    const contextTools = createContextTools(config);
    agent = new RealtimeAgent({
      name: config.name,
      instructions: config.instructions,
      tools: contextTools
    });
    console.log(`🔧 Updated tools for ${config.name}:`, contextTools.map(t => t.name));
  }
  
  ensureReadyBanner(`${config.name} - click Connect to start`);
});

// Mobile menu toggle
sidebarToggle.onclick = () => {
  const isOpen = sidebar.classList.toggle('open');
  updateSidebarToggleState(isOpen);
};

// Close sidebar on mobile when clicking outside
document.addEventListener('click', (e) => {
  if (window.innerWidth <= 768 &&
      !sidebar.contains(e.target as Node) &&
      e.target !== sidebarToggle) {
    sidebar.classList.remove('open');
    updateSidebarToggleState(false);
  }
});

// Check if mobile on load and show toggle button
function checkMobile() {
  if (window.innerWidth <= 768) {
    sidebarToggle.style.display = 'flex';
    updateSidebarToggleState(sidebar.classList.contains('open'));
  } else {
    sidebarToggle.style.display = 'none';
    sidebar.classList.remove('open');
    updateSidebarToggleState(false);
  }
}

window.addEventListener('resize', checkMobile);

// Initialize page
document.addEventListener("DOMContentLoaded", () => {
  checkMobile();
  setupEventHandlers();
  initializeApp();
  // Optional autoconnect via ?autoconnect=1 or localStorage 'jarvis.autoconnect' = 'true'
  const params = new URLSearchParams(window.location.search);
  const autoParam = params.get('autoconnect');
  const autoStored = localStorage.getItem('jarvis.autoconnect');
  const shouldAuto = (autoParam === '1' || autoParam === 'true') || (autoStored === '1' || autoStored === 'true');
  if (shouldAuto) {
    setTimeout(() => connect(), 0);
  }
});

// Setup all event handlers after DOM is ready
function setupEventHandlers(): void {
  // Main action buttons
  toggleConnectionBtn.onclick = () => {
    if (isConnected) {
      disconnect();
    } else {
      connect();
    }
  };

  // Clicking mic area while disconnected will auto-connect
  const micContainer = document.getElementById('voiceButtonContainer');
  // Capture pointer before it hits the disabled button so we can autoconnect
  micContainer?.addEventListener('pointerdown', (e) => {
    if (!isConnected && !isConnecting) {
      e.preventDefault();
      connect();
    }
  }, true);

  // Sync button
  syncNowBtn.onclick = async () => {
    if (!sessionManager) return;
    syncNowBtn.disabled = true;
    try {
      const { pushed, pulled } = await sessionManager.syncNow();
      console.log(`🔄 Synced (pushed ${pushed}, pulled ${pulled})`);
      uiEnhancements.showToast(`Synced ${pushed + pulled} items`, 'success');
    } catch (e) {
      console.error('Sync failed:', e);
      uiEnhancements.showToast('Sync failed', 'error');
    } finally {
      syncNowBtn.disabled = false;
    }
  };

  // Conversation management
  newConversationBtn.onclick = handleNewConversation;
  clearConvosBtn.onclick = handleClearConversations;

  console.log('✅ Event handlers set up successfully');

  // Add visual feedback that event handlers are ready
  uiEnhancements.showToast('Application ready - click Connect to start', 'info');
}

// Expose functions for testing
declare global {
  interface Window {
    addUserTurnToUI: typeof addUserTurnToUI;
    addAssistantTurnToUI: typeof addAssistantTurnToUI;
    loadConversationHistoryIntoUI: typeof loadConversationHistoryIntoUI;
    currentStreamingMessageId: string | null;
    debugConversations: () => Promise<void>;
    syncNow: () => Promise<{ pushed: number; pulled: number }>;
    flushLocal: () => Promise<void>;
    ConversationRenderer: typeof ConversationRenderer;
  }
}

window.addUserTurnToUI = addUserTurnToUI;
window.addAssistantTurnToUI = addAssistantTurnToUI;
window.loadConversationHistoryIntoUI = loadConversationHistoryIntoUI;
window.ConversationRenderer = ConversationRenderer;
window.syncNow = async () => {
  if (!sessionManager) return { pushed: 0, pulled: 0 };
  return await sessionManager.syncNow();
};
window.flushLocal = async () => {
  if (!sessionManager) return;
  await sessionManager.flush();
};

// Debug function to check IndexedDB contents
window.debugConversations = async () => {
  if (!sessionManager) {
    console.log('❌ No session manager');
    return;
  }
  
  const conversations = await sessionManager.getAllConversations();
  console.log('📋 All conversations:', conversations);
  
  if (currentConversationId) {
    const history = await sessionManager.getConversationHistory();
    console.log(`💬 Current conversation (${currentConversationId}) history:`, history);
  }
};

// Initialize Jarvis-Zerg integration
async function initializeJarvisIntegration() {
  try {
    const zergApiURL = import.meta.env.VITE_ZERG_API_URL || 'http://localhost:47300';
    const deviceSecret = import.meta.env.VITE_JARVIS_DEVICE_SECRET;

    if (!deviceSecret) {
      console.warn('⚠️ VITE_JARVIS_DEVICE_SECRET not configured - Zerg integration disabled');
      return;
    }

    console.log('🔌 Initializing Jarvis-Zerg integration...');

    // Authenticate with Zerg
    if (!jarvisClient.isAuthenticated()) {
      await jarvisClient.authenticate(deviceSecret);
      console.log('✅ Authenticated with Zerg backend');
    }

    // Load available agents for voice/text commands
    cachedAgents = await jarvisClient.listAgents();
    console.log(`✅ Loaded ${cachedAgents.length} agents from Zerg`);

    // Initialize Task Inbox
    const taskInboxContainer = document.getElementById('task-inbox-container');
    if (taskInboxContainer) {
      taskInbox = await createTaskInbox(taskInboxContainer, {
        apiURL: zergApiURL,
        deviceSecret: deviceSecret,
        onError: (error) => {
          console.error('Task Inbox error:', error);
          uiEnhancements.showToast('Task Inbox error - check console', 'error');
        },
        onRunUpdate: (run) => {
          // Speak result when agent completes successfully
          if (run.status === 'success' && run.summary) {
            console.log(`✅ Agent "${run.agent_name}" completed:`, run.summary);
            // Optional: Add TTS here when ready
            // const utterance = new SpeechSynthesisUtterance(run.summary);
            // speechSynthesis.speak(utterance);
          }
        },
      });
      console.log('✅ Task Inbox initialized');

      // Show Task Inbox on wide screens
      if (window.innerWidth >= 1440) {
        taskInboxContainer.classList.add('visible');
      }
    }

    // Set up text input handler
    const textInput = document.getElementById('textInput') as HTMLInputElement;
    const sendTextBtn = document.getElementById('sendTextBtn');

    const handleTextCommand = async () => {
      const text = textInput?.value.trim();
      if (!text) return;

      console.log('💬 Text command:', text);

      // Try to map to agent dispatch
      const agent = findAgentByIntent(text);

      if (agent) {
        console.log(`🎯 Dispatching agent: ${agent.name}`);
        textInput.value = '';

        try {
          const result = await jarvisClient.dispatch({ agent_id: agent.id });
          console.log('✅ Agent dispatched:', result);
          uiEnhancements.showToast(`Running ${agent.name}...`, 'success');

          // Add message to UI
          addUserTurnToUI(text);
          addAssistantTurnToUI(`Started ${agent.name}. Check Task Inbox for updates.`);
        } catch (error: any) {
          console.error('Dispatch failed:', error);
          uiEnhancements.showToast(error.message || 'Dispatch failed', 'error');
          addAssistantTurnToUI(`Failed to start ${agent.name}: ${error.message}`);
        }
      } else {
        // Handle as regular conversation (if connected)
        if (isConnected && session) {
          textInput.value = '';
          addUserTurnToUI(text);
          session.send({ type: 'input_text', text });
        } else {
          uiEnhancements.showToast('Not connected - click Connect first', 'error');
        }
      }
    };

    sendTextBtn?.addEventListener('click', handleTextCommand);
    textInput?.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        handleTextCommand();
      }
    });

    console.log('✅ Text input mode enabled');

  } catch (error) {
    console.error('❌ Jarvis-Zerg integration failed:', error);
    // Non-fatal - voice features still work
  }
}

// Map user text/voice to agent intent
function findAgentByIntent(text: string): JarvisAgentSummary | null {
  const lower = text.toLowerCase();

  // Agent keywords mapping
  if (lower.includes('morning') || lower.includes('digest')) {
    return cachedAgents.find(a => a.name === 'Morning Digest') || null;
  }
  if (lower.includes('health') || lower.includes('recovery') || lower.includes('whoop')) {
    return cachedAgents.find(a => a.name === 'Health Watch') || null;
  }
  if (lower.includes('status') || lower.includes('quick check')) {
    return cachedAgents.find(a => a.name === 'Quick Status Check') || null;
  }
  if (lower.includes('planning') || lower.includes('week ahead')) {
    return cachedAgents.find(a => a.name === 'Weekly Planning Assistant') || null;
  }

  // Check if text starts with "run" or "execute"
  if (lower.startsWith('run ') || lower.startsWith('execute ')) {
    // Extract agent name after "run"
    const agentNamePart = lower.replace(/^(run|execute)\s+/, '').trim();
    return cachedAgents.find(a =>
      a.name.toLowerCase().includes(agentNamePart)
    ) || null;
  }

  return null;
}

// Initialize integration after DOM ready
document.addEventListener('DOMContentLoaded', () => {
  initializeJarvisIntegration().catch(err => {
    console.error('Integration initialization failed:', err);
  });
});

console.log('Jarvis PWA with Agents SDK ready', {
  toggleConnectionBtn: !!toggleConnectionBtn,
  pttBtn: !!pttBtn,
  transcript: !!transcriptEl,
  newConversationBtn: !!newConversationBtn
});
