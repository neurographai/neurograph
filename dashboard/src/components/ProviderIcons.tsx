import React from 'react';

// ════════════════════════════════════════════════════════════
// Official LLM Provider SVG Icons
// Based on each provider's official branding guidelines
// ════════════════════════════════════════════════════════════

interface IconProps {
  size?: number;
  className?: string;
}

/** OpenAI logomark — hexagonal iris */
export function OpenAIIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494zM3.6 18.304a4.47 4.47 0 0 1-.535-3.014l.142.085 4.783 2.759a.771.771 0 0 0 .78 0l5.843-3.369v2.332a.08.08 0 0 1-.033.062L9.74 19.95a4.5 4.5 0 0 1-6.14-1.646zM2.34 7.896a4.485 4.485 0 0 1 2.366-1.973V11.6a.766.766 0 0 0 .388.676l5.815 3.355-2.02 1.168a.076.076 0 0 1-.071 0l-4.83-2.786A4.504 4.504 0 0 1 2.34 7.872zm16.597 3.855l-5.833-3.387L15.119 7.2a.076.076 0 0 1 .071 0l4.83 2.791a4.494 4.494 0 0 1-.676 8.105v-5.678a.79.79 0 0 0-.407-.667zm2.01-3.023l-.141-.085-4.774-2.782a.776.776 0 0 0-.785 0L9.409 9.23V6.897a.066.066 0 0 1 .028-.061l4.83-2.787a4.5 4.5 0 0 1 6.68 4.66zm-12.64 4.135l-2.02-1.164a.08.08 0 0 1-.038-.057V6.075a4.5 4.5 0 0 1 7.375-3.453l-.142.08L8.704 5.46a.795.795 0 0 0-.393.681zm1.097-2.365l2.602-1.5 2.607 1.5v2.999l-2.597 1.5-2.607-1.5z"
        fill="#10a37f"
      />
    </svg>
  );
}

/** Anthropic logomark — stylized A */
export function AnthropicIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M13.827 3.52h3.603L24 20.48h-3.603l-6.57-16.96zm-7.257 0h3.603L16.744 20.48h-3.603L6.57 3.52zM0 20.48h3.603L10.174 3.52H6.57L0 20.48z"
        fill="#d4a574"
      />
    </svg>
  );
}

/** Google Gemini logomark — 4-point star */
export function GeminiIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M12 0C12 6.627 6.627 12 0 12c6.627 0 12 5.373 12 12 0-6.627 5.373-12 12-12-6.627 0-12-5.373-12-12z"
        fill="url(#gemini-grad)"
      />
      <defs>
        <linearGradient id="gemini-grad" x1="0" y1="0" x2="24" y2="24" gradientUnits="userSpaceOnUse">
          <stop stopColor="#4285f4" />
          <stop offset="0.5" stopColor="#9b72cb" />
          <stop offset="1" stopColor="#d96570" />
        </linearGradient>
      </defs>
    </svg>
  );
}

/** xAI / Grok logomark — stylized X */
export function XAIIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M2 2l8.586 10L2 22h2.414L12 14.414 19.586 22H22l-8.586-10L22 2h-2.414L12 9.586 4.414 2H2z"
        fill="#e7e9ea"
      />
    </svg>
  );
}

/** Groq logomark — lightning bolt shape */
export function GroqIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2z"
        fill="#f55036"
      />
      <path
        d="M12 6.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm0 2a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7z"
        fill="#fff"
      />
      <circle cx="16" cy="8" r="1.5" fill="#fff" />
    </svg>
  );
}

/** Ollama logomark — stylized llama head */
export function OllamaIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" className={className}>
      <path
        d="M12 2C8.686 2 6 4.686 6 8v2c-1.105 0-2 .895-2 2v4c0 1.105.895 2 2 2v2c0 1.105.895 2 2 2h8c1.105 0 2-.895 2-2v-2c1.105 0 2-.895 2-2v-4c0-1.105-.895-2-2-2V8c0-3.314-2.686-6-6-6z"
        fill="#f5f5f5"
        stroke="#999"
        strokeWidth="0.5"
      />
      <circle cx="9.5" cy="10" r="1.25" fill="#333" />
      <circle cx="14.5" cy="10" r="1.25" fill="#333" />
      <ellipse cx="12" cy="13.5" rx="2" ry="1.25" fill="#ccc" />
    </svg>
  );
}

/** Lookup map: provider key -> React component */
export const PROVIDER_ICONS: Record<string, React.FC<IconProps>> = {
  openai: OpenAIIcon,
  anthropic: AnthropicIcon,
  gemini: GeminiIcon,
  xai: XAIIcon,
  groq: GroqIcon,
  ollama: OllamaIcon,
};
