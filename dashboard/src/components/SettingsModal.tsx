import React, { useEffect, useState } from 'react';
import {
  Settings,
  X,
  Loader2,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Key,
  Zap,
  BarChart3,
  Eye,
  EyeOff,
} from 'lucide-react';
import { PROVIDER_ICONS } from './ProviderIcons';
import type { ProviderConfig, UsageReport } from '../types/settings';

// ════════════════════════════════════════════════════════════
// NeuroGraph Settings Modal — Provider Management
// ════════════════════════════════════════════════════════════

const API_BASE = '/api/v1';

const PROVIDER_META: Record<string, { color: string; keyEnv: string; keyPrefix: string }> = {
  openai: { color: '#10a37f', keyEnv: 'OPENAI_API_KEY', keyPrefix: 'sk-' },
  anthropic: { color: '#d4a574', keyEnv: 'ANTHROPIC_API_KEY', keyPrefix: 'sk-ant-' },
  gemini: { color: '#4285f4', keyEnv: 'GEMINI_API_KEY', keyPrefix: 'AI' },
  xai: { color: '#1da1f2', keyEnv: 'XAI_API_KEY', keyPrefix: 'xai-' },
  groq: { color: '#f55036', keyEnv: 'GROQ_API_KEY', keyPrefix: 'gsk_' },
  ollama: { color: '#f5f5f5', keyEnv: '', keyPrefix: '' },
};

export function SettingsModal({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<'providers' | 'usage'>('providers');
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [usage, setUsage] = useState<UsageReport | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadProviders();
    loadUsage();
  }, []);

  const loadProviders = async () => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/llm/providers`);
      const wrapper = await res.json();
      setProviders(wrapper.data ?? []);
    } catch { /* ignore */ }
    setLoading(false);
  };

  const loadUsage = async () => {
    try {
      const res = await fetch(`${API_BASE}/llm/usage`);
      const wrapper = await res.json();
      setUsage(wrapper.data ?? null);
    } catch { /* ignore */ }
  };

  return (
    <div className="ng-modal-overlay" onClick={onClose}>
      <div className="ng-modal" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="ng-modal-header">
          <div className="ng-modal-title">
            <Settings size={18} />
            <span>Settings</span>
          </div>
          <button onClick={onClose} className="ng-chat-icon-btn">
            <X size={16} />
          </button>
        </div>

        {/* Tabs */}
        <div className="ng-modal-tabs">
          <button
            className={`ng-modal-tab ${tab === 'providers' ? 'ng-modal-tab--active' : ''}`}
            onClick={() => setTab('providers')}
          >
            <Key size={14} /> Providers
          </button>
          <button
            className={`ng-modal-tab ${tab === 'usage' ? 'ng-modal-tab--active' : ''}`}
            onClick={() => setTab('usage')}
          >
            <BarChart3 size={14} /> Usage
          </button>
        </div>

        {/* Content */}
        <div className="ng-modal-content">
          {loading ? (
            <div className="ng-modal-loading">
              <Loader2 size={24} className="ng-spinner" />
              <span>Loading...</span>
            </div>
          ) : tab === 'providers' ? (
            <ProvidersTab providers={providers} onRefresh={loadProviders} />
          ) : (
            <UsageTab usage={usage} />
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Providers Tab ──────────────────────────────────────────

function ProvidersTab({
  providers,
  onRefresh,
}: {
  providers: ProviderConfig[];
  onRefresh: () => void;
}) {
  return (
    <div className="ng-providers-grid">
      {providers.map((p) => (
        <ProviderCard key={p.provider} provider={p} onRefresh={onRefresh} />
      ))}
    </div>
  );
}

function ProviderCard({
  provider,
  onRefresh,
}: {
  provider: ProviderConfig;
  onRefresh: () => void;
}) {
  const meta = PROVIDER_META[provider.provider] ?? PROVIDER_META.openai;
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    healthy: boolean;
    latency_ms?: number;
    error?: string;
  } | null>(null);

  const isOllama = provider.provider === 'ollama';
  const ProviderIconComponent = PROVIDER_ICONS[provider.provider];

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const res = await fetch(`${API_BASE}/llm/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: provider.provider,
          api_key: apiKey,
        }),
      });
      const wrapper = await res.json();
      const data = wrapper.data ?? wrapper;
      setTestResult({
        healthy: data.healthy,
        latency_ms: data.latency_ms,
        error: data.error,
      });
      if (data.healthy) {
        onRefresh();
      }
    } catch (e) {
      setTestResult({ healthy: false, error: 'Network error' });
    }
    setTesting(false);
  };

  const StatusIcon = provider.healthy
    ? CheckCircle2
    : provider.configured
      ? XCircle
      : AlertCircle;

  const statusColor = provider.healthy
    ? '#10b981'
    : provider.configured
      ? '#ef4444'
      : '#6b7280';

  return (
    <div className="ng-provider-card" style={{ '--provider-color': meta.color } as React.CSSProperties}>
      <div className="ng-provider-header">
        <span className="ng-provider-icon">
          {ProviderIconComponent ? <ProviderIconComponent size={18} /> : null}
        </span>
        <span className="ng-provider-name">{provider.display_name}</span>
        <StatusIcon size={16} color={statusColor} />
      </div>

      <div className="ng-provider-model">{provider.model}</div>

      {provider.healthy && provider.latency_ms && (
        <div className="ng-provider-latency">
          <Zap size={10} /> {provider.latency_ms}ms
        </div>
      )}

      {!isOllama && (
        <div className="ng-provider-key-row">
          <div className="ng-api-key-input">
            <input
              type={showKey ? 'text' : 'password'}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={`${meta.keyPrefix}...`}
              className="ng-key-input"
            />
            <button onClick={() => setShowKey(!showKey)} className="ng-key-toggle">
              {showKey ? <EyeOff size={12} /> : <Eye size={12} />}
            </button>
          </div>
          <button
            onClick={handleTest}
            disabled={!apiKey.trim() || testing}
            className="ng-test-btn"
          >
            {testing ? (
              <Loader2 size={12} className="ng-spinner" />
            ) : (
              'Test'
            )}
          </button>
        </div>
      )}

      {testResult && (
        <div className={`ng-test-result ${testResult.healthy ? 'ng-test-result--ok' : 'ng-test-result--err'}`}>
          {testResult.healthy ? (
            <>
              <CheckCircle2 size={12} /> Connected
              {testResult.latency_ms && ` (${testResult.latency_ms}ms)`}
            </>
          ) : (
            <>
              <XCircle size={12} /> {testResult.error ?? 'Failed'}
            </>
          )}
        </div>
      )}

      {provider.error && !testResult && (
        <div className="ng-test-result ng-test-result--err">
          <XCircle size={12} /> {provider.error}
        </div>
      )}
    </div>
  );
}

// ─── Usage Tab ──────────────────────────────────────────────

function UsageTab({ usage }: { usage: UsageReport | null }) {
  if (!usage) {
    return (
      <div className="ng-usage-empty">
        <BarChart3 size={32} strokeWidth={1} />
        <p>No usage data yet. Start chatting to see token usage and costs.</p>
      </div>
    );
  }

  return (
    <div className="ng-usage">
      {/* Summary cards */}
      <div className="ng-usage-summary">
        <div className="ng-usage-card">
          <span className="ng-usage-label">Total Tokens</span>
          <span className="ng-usage-value">
            {(usage.total_input_tokens + usage.total_output_tokens).toLocaleString()}
          </span>
        </div>
        <div className="ng-usage-card">
          <span className="ng-usage-label">API Calls</span>
          <span className="ng-usage-value">{usage.total_calls}</span>
        </div>
        <div className="ng-usage-card">
          <span className="ng-usage-label">Total Cost</span>
          <span className="ng-usage-value">${usage.total_cost_usd.toFixed(4)}</span>
        </div>
      </div>

      {/* Breakdown table */}
      {usage.by_prompt_type.length > 0 && (
        <div className="ng-usage-table">
          <div className="ng-usage-table-header">
            <span>Type</span>
            <span>Input</span>
            <span>Output</span>
            <span>Calls</span>
            <span>Cost</span>
          </div>
          {usage.by_prompt_type.map((pt) => (
            <div key={pt.prompt_type} className="ng-usage-table-row">
              <span>{pt.prompt_type.replace(/_/g, ' ')}</span>
              <span>{pt.input_tokens.toLocaleString()}</span>
              <span>{pt.output_tokens.toLocaleString()}</span>
              <span>{pt.call_count}</span>
              <span>${pt.cost_usd.toFixed(4)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Settings Trigger Button ────────────────────────────────

export function SettingsTrigger({ onClick }: { onClick: () => void }) {
  return (
    <button onClick={onClick} className="ng-settings-trigger" title="Settings">
      <Settings size={14} />
    </button>
  );
}
