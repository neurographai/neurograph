// ============================================================
// NeuroGraph — G6 Theme Configuration
// Color palettes, type mappings, and state styles
// ============================================================

/**
 * Node type → fill color mapping.
 * Must match the sidebar legend exactly.
 */
export const NODE_TYPE_COLORS: Record<string, string> = {
  person: '#6366f1',       // Indigo
  organization: '#3b82f6', // Blue
  location: '#10b981',     // Emerald
  event: '#f59e0b',        // Amber
  fact: '#10b981',         // Emerald (same family as location — facts are green)
  concept: '#ec4899',      // Pink
  technology: '#8b5cf6',   // Violet
  product: '#f97316',      // Orange
  document: '#6366f1',     // Indigo
  entity: '#6366f1',       // Default fallback
};

/**
 * Memory tier → ring/stroke color mapping.
 * L1 (working) = brightest, L4 (procedural) = subtlest
 */
export const TIER_RING_COLORS: Record<string, string> = {
  working: '#f59e0b',     // Amber — hot, active
  episodic: '#3b82f6',    // Blue — recent memories
  semantic: '#10b981',    // Emerald — stable knowledge
  procedural: '#6b7280',  // Gray — deep, implicit
};

/**
 * Edge relation type → stroke color mapping.
 */
export const EDGE_TYPE_COLORS: Record<string, string> = {
  entity: '#6366f1',      // Indigo — structural relationships
  semantic: '#8b5cf6',    // Violet — meaning similarity
  temporal: '#f59e0b',    // Amber — time-ordered
  causal: '#ef4444',      // Red — cause-effect
};

/**
 * Custom dark theme for G6 graph canvas.
 * Applied via graph.setTheme() or GraphOptions.theme
 */
export const NEUROGRAPH_THEME = {
  type: 'dark' as const,

  /** Default node colors */
  node: {
    palette: Object.values(NODE_TYPE_COLORS),
  },

  /** Default edge colors */
  edge: {
    palette: Object.values(EDGE_TYPE_COLORS),
  },
};

/** State styles for interactive elements */
export const STATE_STYLES = {
  node: {
    selected: {
      lineWidth: 4,
      shadowBlur: 28,
      halo: true,
      haloStrokeOpacity: 0.4,
      haloLineWidth: 20,
    },
    active: {
      lineWidth: 3,
      shadowBlur: 20,
      halo: true,
      haloStrokeOpacity: 0.25,
      haloLineWidth: 14,
    },
    inactive: {
      opacity: 0.2,
      labelOpacity: 0.3,
    },
    highlight: {
      lineWidth: 4,
      shadowBlur: 32,
      halo: true,
      haloStroke: 'hsl(258, 90%, 66%)',
      haloStrokeOpacity: 0.45,
      haloLineWidth: 22,
    },
  },
  edge: {
    selected: {
      lineWidth: 3.5,
      opacity: 1,
      halo: true,
      haloStrokeOpacity: 0.35,
      haloLineWidth: 12,
    },
    active: {
      lineWidth: 3,
      opacity: 1,
      halo: true,
      haloStrokeOpacity: 0.2,
      haloLineWidth: 8,
      labelOpacity: 1,
    },
    inactive: {
      opacity: 0.08,
      labelOpacity: 0,
    },
    highlight: {
      lineWidth: 3.5,
      opacity: 1,
      lineDash: [8, 4],
      halo: true,
      haloStroke: 'hsl(258, 90%, 66%)',
      haloStrokeOpacity: 0.3,
    },
  },
  combo: {
    selected: {
      lineWidth: 2,
      stroke: 'hsl(258, 60%, 50%)',
    },
    active: {
      lineWidth: 1.5,
      fill: 'hsla(225, 18%, 16%, 0.3)',
    },
  },
};
