import React, { useEffect, useRef, useCallback } from 'react';
import { useChatStore } from '../store/chatStore';
import {
  INTENT_LABELS,
  INTENT_COLORS,
  type ChatMessage,
  type EvidenceChunk,
  type GraphAction,
} from '../types/chat';
import {
  MessageSquare,
  Send,
  Loader2,
  ChevronDown,
  ChevronRight,
  Sparkles,
  Brain,
  Zap,
  Clock,
  DollarSign,
  Trash2,
  X,
} from 'lucide-react';

// ════════════════════════════════════════════════════════════
// NeuroGraph ChatPanel — Full Agent Interface
// ════════════════════════════════════════════════════════════

export function ChatPanel() {
  const {
    messages,
    isProcessing,
    intentPreview,
    chatOpen,
    inputValue,
    sendMessage,
    classifyIntent,
    toggleChat,
    setInputValue,
    clearChat,
  } = useChatStore();

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input when panel opens
  useEffect(() => {
    if (chatOpen) inputRef.current?.focus();
  }, [chatOpen]);

  // Debounced intent classification
  const handleInputChange = useCallback(
    (value: string) => {
      setInputValue(value);
      clearTimeout(debounceTimer.current);
      debounceTimer.current = setTimeout(() => {
        classifyIntent(value);
      }, 300);
    },
    [setInputValue, classifyIntent]
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputValue.trim() || isProcessing) return;
    sendMessage(inputValue.trim());
  };

  const handleFollowUp = (question: string) => {
    sendMessage(question);
  };

  if (!chatOpen) {
    return (
      <button
        className="ng-chat-fab"
        onClick={toggleChat}
        title="Open Chat Agent"
      >
        <MessageSquare size={22} />
        <span className="ng-chat-fab-pulse" />
      </button>
    );
  }

  return (
    <aside className="ng-chat-panel">
      {/* Header */}
      <div className="ng-chat-header">
        <div className="ng-chat-header-left">
          <Brain size={18} />
          <span>NeuroGraph Agent</span>
        </div>
        <div className="ng-chat-header-actions">
          <button onClick={clearChat} title="Clear chat" className="ng-chat-icon-btn">
            <Trash2 size={14} />
          </button>
          <button onClick={toggleChat} title="Close" className="ng-chat-icon-btn">
            <X size={14} />
          </button>
        </div>
      </div>

      {/* Message List */}
      <div className="ng-chat-messages">
        {messages.length === 0 && (
          <div className="ng-chat-empty">
            <Sparkles size={32} strokeWidth={1.2} />
            <h4>Ask anything about your knowledge graph</h4>
            <p>I can explain concepts, trace relationships, find contradictions, and more.</p>
            <div className="ng-chat-suggestions">
              {[
                'What are the main topics?',
                'Summarize the key findings',
                'Show me entity connections',
                'Find contradictions',
              ].map((q) => (
                <button key={q} onClick={() => handleFollowUp(q)} className="ng-chat-suggestion">
                  {q}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} onFollowUp={handleFollowUp} />
        ))}

        {isProcessing && (
          <div className="ng-chat-typing">
            <Loader2 size={14} className="ng-spinner" />
            <span>
              {intentPreview
                ? `${intentPreview.label}...`
                : 'Processing...'}
            </span>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input Bar */}
      <form className="ng-chat-input-bar" onSubmit={handleSubmit}>
        {/* Intent preview badge */}
        {intentPreview && inputValue.trim().length > 2 && !isProcessing && (
          <div
            className="ng-intent-badge"
            style={{ '--intent-color': INTENT_COLORS[intentPreview.intent] } as React.CSSProperties}
          >
            <Zap size={10} />
            {intentPreview.label}
            <span className="ng-intent-conf">
              {Math.round(intentPreview.confidence * 100)}%
            </span>
          </div>
        )}
        <div className="ng-chat-input-row">
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => handleInputChange(e.target.value)}
            placeholder="Ask about your knowledge graph..."
            disabled={isProcessing}
            className="ng-chat-input"
          />
          <button
            type="submit"
            disabled={!inputValue.trim() || isProcessing}
            className="ng-chat-send"
          >
            {isProcessing ? <Loader2 size={16} className="ng-spinner" /> : <Send size={16} />}
          </button>
        </div>
      </form>
    </aside>
  );
}

// ─── Message Bubble ─────────────────────────────────────────

function MessageBubble({
  message,
  onFollowUp,
}: {
  message: ChatMessage;
  onFollowUp: (q: string) => void;
}) {
  if (message.role === 'user') {
    return (
      <div className="ng-msg ng-msg--user">
        <div className="ng-msg-content">{message.content}</div>
      </div>
    );
  }

  const resp = message.response;

  return (
    <div className="ng-msg ng-msg--assistant">
      {/* Intent badge */}
      {resp?.meta?.intent && (
        <div
          className="ng-intent-badge ng-intent-badge--msg"
          style={
            { '--intent-color': INTENT_COLORS[resp.meta.intent.intent] } as React.CSSProperties
          }
        >
          <Zap size={10} />
          {INTENT_LABELS[resp.meta.intent.intent]}
        </div>
      )}

      {/* Answer */}
      <div className="ng-msg-content">{message.content}</div>

      {/* Evidence */}
      {resp?.evidence && resp.evidence.length > 0 && (
        <EvidenceDrawer evidence={resp.evidence} />
      )}

      {/* Graph Actions Summary */}
      {resp?.graph_actions && resp.graph_actions.length > 0 && (
        <GraphActionBar actions={resp.graph_actions} />
      )}

      {/* Follow-ups */}
      {resp?.follow_ups && resp.follow_ups.length > 0 && (
        <div className="ng-follow-ups">
          {resp.follow_ups.map((fu, i) => (
            <button
              key={i}
              className="ng-follow-up-chip"
              onClick={() => onFollowUp(fu.question)}
              title={fu.rationale}
            >
              <ChevronRight size={12} />
              {fu.question}
            </button>
          ))}
        </div>
      )}

      {/* Meta footer */}
      {resp?.meta && (
        <div className="ng-msg-meta">
          <span><Brain size={10} /> {resp.meta.model_used}</span>
          <span><Clock size={10} /> {resp.meta.latency_ms}ms</span>
          <span><Zap size={10} /> {resp.meta.input_tokens + resp.meta.output_tokens} tok</span>
          {resp.meta.cost_usd > 0 && (
            <span><DollarSign size={10} /> ${resp.meta.cost_usd.toFixed(4)}</span>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Evidence Drawer ────────────────────────────────────────

function EvidenceDrawer({ evidence }: { evidence: EvidenceChunk[] }) {
  const [open, setOpen] = React.useState(false);

  return (
    <div className="ng-evidence">
      <button className="ng-evidence-toggle" onClick={() => setOpen(!open)}>
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        {evidence.length} source{evidence.length !== 1 ? 's' : ''}
      </button>
      {open && (
        <div className="ng-evidence-list">
          {evidence.map((e, i) => (
            <div key={i} className="ng-evidence-item">
              <div className="ng-evidence-score">
                {Math.round(e.relevance_score * 100)}%
              </div>
              <div className="ng-evidence-text">
                <span className="ng-evidence-source">
                  {e.source.type === 'entity'
                    ? `Entity: ${e.source.entity_name}`
                    : e.source.type === 'paper'
                      ? `Paper: ${e.source.title}`
                      : e.source.type}
                </span>
                <p>{e.text.slice(0, 200)}{e.text.length > 200 ? '…' : ''}</p>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// React import already at top

// ─── Graph Action Bar ───────────────────────────────────────

function GraphActionBar({ actions }: { actions: GraphAction[] }) {
  return (
    <div className="ng-graph-action-bar">
      <Sparkles size={12} />
      <span>{actions.length} graph action{actions.length !== 1 ? 's' : ''} applied</span>
      <span className="ng-graph-action-labels">
        {actions.map((a, i) => (
          <span key={i} className="ng-graph-action-label">{a.description}</span>
        ))}
      </span>
    </div>
  );
}
