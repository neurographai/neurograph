import { create } from 'zustand';
import type {
  ChatMessage,
  ChatAgentResponse,
  IntentPreview,
  ChatSession,
  GraphAction,
} from '../types/chat';
import { useGraphStore } from './graphStore';

// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Chat Agent Store
// ════════════════════════════════════════════════════════════

const API_BASE = '/api/v1';

interface ChatState {
  // Messages
  messages: ChatMessage[];
  isProcessing: boolean;
  streamingTokens: string;

  // Session
  sessionId: string | null;
  sessions: ChatSession[];

  // Intent preview (live as user types)
  intentPreview: IntentPreview | null;
  isClassifying: boolean;

  // Pending graph actions from latest response
  pendingActions: GraphAction[];

  // UI state
  chatOpen: boolean;
  inputValue: string;

  // Actions
  sendMessage: (message: string) => Promise<void>;
  classifyIntent: (message: string) => Promise<void>;
  loadSessions: () => Promise<void>;
  switchSession: (sessionId: string | null) => void;
  clearChat: () => void;
  applyGraphActions: (actions: GraphAction[]) => void;
  toggleChat: () => void;
  setInputValue: (value: string) => void;
}

export const useChatStore = create<ChatState>((set, get) => ({
  messages: [],
  isProcessing: false,
  streamingTokens: '',
  sessionId: null,
  sessions: [],
  intentPreview: null,
  isClassifying: false,
  pendingActions: [],
  chatOpen: false,
  inputValue: '',

  sendMessage: async (message: string) => {
    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: message,
      timestamp: new Date().toISOString(),
    };

    set((s) => ({
      messages: [...s.messages, userMsg],
      isProcessing: true,
      intentPreview: null,
      inputValue: '',
    }));

    try {
      const res = await fetch(`${API_BASE}/chat/agent`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message,
          session_id: get().sessionId,
        }),
      });

      const wrapper = await res.json();
      const data: ChatAgentResponse = wrapper.data ?? wrapper;

      // Update session ID from response
      const newSessionId = data.meta?.session_id ?? get().sessionId;

      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: data.answer,
        timestamp: new Date().toISOString(),
        response: data,
      };

      set((s) => ({
        messages: [...s.messages, assistantMsg],
        isProcessing: false,
        sessionId: newSessionId,
        pendingActions: data.graph_actions ?? [],
      }));

      // Auto-apply graph actions
      if (data.graph_actions?.length) {
        get().applyGraphActions(data.graph_actions);
      }
    } catch {
      const errorMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content:
          'Sorry, I encountered an error processing your message. Please check that the server is running.',
        timestamp: new Date().toISOString(),
      };

      set((s) => ({
        messages: [...s.messages, errorMsg],
        isProcessing: false,
      }));
    }
  },

  classifyIntent: async (message: string) => {
    if (message.trim().length < 3) {
      set({ intentPreview: null });
      return;
    }

    set({ isClassifying: true });
    try {
      const res = await fetch(`${API_BASE}/chat/intent`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message }),
      });
      const wrapper = await res.json();
      const data = wrapper.data ?? wrapper;
      set({
        intentPreview: {
          intent: data.intent,
          confidence: data.confidence,
          label: data.label,
          entities: data.entities ?? [],
        },
        isClassifying: false,
      });
    } catch {
      set({ isClassifying: false });
    }
  },

  loadSessions: async () => {
    try {
      const res = await fetch(`${API_BASE}/chat/sessions`);
      const wrapper = await res.json();
      set({ sessions: wrapper.data ?? [] });
    } catch {
      // ignore
    }
  },

  switchSession: (sessionId: string | null) => {
    set({ sessionId, messages: [] });
  },

  clearChat: () => {
    set({
      messages: [],
      sessionId: null,
      intentPreview: null,
      pendingActions: [],
    });
  },

  applyGraphActions: (actions: GraphAction[]) => {
    const graphStore = useGraphStore.getState();
    for (const action of actions) {
      if (typeof action.action_type === 'string') {
        // reset_view
        // Could reset graph view if we had that method
        continue;
      }
      if ('highlight_nodes' in action.action_type) {
        // Highlight nodes by selecting them sequentially
        const ids = action.action_type.highlight_nodes.node_ids;
        const node = graphStore.nodes.find((n) => ids.includes(n.id));
        if (node) graphStore.selectNode(node);
      }
      if ('switch_view' in action.action_type) {
        const view = action.action_type.switch_view.view;
        graphStore.setActiveGraphView(view as 'all' | 'semantic' | 'temporal' | 'causal' | 'entity' | 'embeddings');
      }
    }
  },

  toggleChat: () => set((s) => ({ chatOpen: !s.chatOpen })),
  setInputValue: (value: string) => set({ inputValue: value }),
}));
