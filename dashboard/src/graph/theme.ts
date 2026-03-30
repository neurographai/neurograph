// ============================================================
// NeuroGraph — G6 Theme Configuration
// Premium dark theme matching the design system
// ============================================================

/**
 * Custom dark theme for G6 graph canvas.
 * Applied via graph.setTheme() or GraphOptions.theme
 */
export const NEUROGRAPH_THEME = {
  type: 'dark' as const,

  /** Default node colors */
  node: {
    palette: [
      'hsl(258, 75%, 58%)',  // Person
      'hsl(215, 80%, 55%)',  // Organization
      'hsl(155, 65%, 48%)',  // Location
      'hsl(38, 85%, 55%)',   // Event
      'hsl(185, 75%, 50%)',  // Concept
      'hsl(330, 70%, 55%)',  // Technology
      'hsl(25, 82%, 52%)',   // Product
      'hsl(240, 60%, 58%)',  // Document
    ],
  },

  /** Default edge colors */
  edge: {
    palette: [
      'hsl(225, 25%, 45%)',
      'hsl(215, 70%, 55%)',
      'hsl(258, 65%, 58%)',
      'hsl(155, 60%, 48%)',
    ],
  },
};

/** State styles for interactive elements */
export const STATE_STYLES = {
  node: {
    selected: {
      lineWidth: 3,
      shadowBlur: 24,
      halo: true,
      haloStrokeOpacity: 0.35,
      haloLineWidth: 16,
    },
    active: {
      lineWidth: 2.5,
      shadowBlur: 18,
      halo: true,
      haloStrokeOpacity: 0.2,
      haloLineWidth: 12,
    },
    inactive: {
      opacity: 0.25,
    },
    highlight: {
      lineWidth: 3,
      shadowBlur: 30,
      halo: true,
      haloStroke: 'hsl(258, 90%, 66%)',
      haloStrokeOpacity: 0.4,
      haloLineWidth: 20,
    },
  },
  edge: {
    selected: {
      lineWidth: 3,
      opacity: 1,
      halo: true,
      haloStrokeOpacity: 0.3,
      haloLineWidth: 10,
    },
    active: {
      lineWidth: 2.5,
      opacity: 0.9,
      halo: true,
      haloStrokeOpacity: 0.15,
      haloLineWidth: 8,
    },
    inactive: {
      opacity: 0.1,
    },
    highlight: {
      lineWidth: 3,
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
