// ============================================================
// NeuroGraph — Entity Type Color Palette
// Maps entity types to curated colors for G6 visualization
// ============================================================

export interface EntityColor {
  fill: string;
  stroke: string;
  text: string;
  glow: string;
  badge: string;
  badgeText: string;
}

const PALETTE: Record<string, EntityColor> = {
  Person: {
    fill: 'hsl(258, 75%, 58%)',
    stroke: 'hsl(258, 60%, 45%)',
    text: '#ffffff',
    glow: 'hsla(258, 90%, 66%, 0.3)',
    badge: 'hsl(258, 50%, 28%)',
    badgeText: 'hsl(258, 90%, 72%)',
  },
  Organization: {
    fill: 'hsl(215, 80%, 55%)',
    stroke: 'hsl(215, 65%, 42%)',
    text: '#ffffff',
    glow: 'hsla(215, 90%, 60%, 0.3)',
    badge: 'hsl(215, 40%, 22%)',
    badgeText: 'hsl(215, 90%, 68%)',
  },
  Location: {
    fill: 'hsl(155, 65%, 48%)',
    stroke: 'hsl(155, 50%, 35%)',
    text: '#ffffff',
    glow: 'hsla(155, 75%, 50%, 0.3)',
    badge: 'hsl(155, 35%, 20%)',
    badgeText: 'hsl(155, 75%, 55%)',
  },
  Event: {
    fill: 'hsl(38, 85%, 55%)',
    stroke: 'hsl(38, 70%, 42%)',
    text: '#1a1a1a',
    glow: 'hsla(38, 92%, 58%, 0.3)',
    badge: 'hsl(38, 40%, 20%)',
    badgeText: 'hsl(38, 92%, 62%)',
  },
  Concept: {
    fill: 'hsl(185, 75%, 50%)',
    stroke: 'hsl(185, 60%, 38%)',
    text: '#1a1a1a',
    glow: 'hsla(185, 85%, 55%, 0.3)',
    badge: 'hsl(185, 40%, 22%)',
    badgeText: 'hsl(185, 85%, 58%)',
  },
  Technology: {
    fill: 'hsl(330, 70%, 55%)',
    stroke: 'hsl(330, 55%, 42%)',
    text: '#ffffff',
    glow: 'hsla(330, 80%, 60%, 0.3)',
    badge: 'hsl(330, 35%, 22%)',
    badgeText: 'hsl(330, 80%, 65%)',
  },
  Product: {
    fill: 'hsl(25, 82%, 52%)',
    stroke: 'hsl(25, 68%, 40%)',
    text: '#ffffff',
    glow: 'hsla(25, 90%, 55%, 0.3)',
    badge: 'hsl(25, 40%, 20%)',
    badgeText: 'hsl(25, 90%, 60%)',
  },
  Document: {
    fill: 'hsl(240, 60%, 58%)',
    stroke: 'hsl(240, 48%, 45%)',
    text: '#ffffff',
    glow: 'hsla(240, 70%, 62%, 0.3)',
    badge: 'hsl(240, 35%, 22%)',
    badgeText: 'hsl(240, 70%, 68%)',
  },
  Topic: {
    fill: 'hsl(170, 65%, 45%)',
    stroke: 'hsl(170, 50%, 34%)',
    text: '#ffffff',
    glow: 'hsla(170, 70%, 48%, 0.3)',
    badge: 'hsl(170, 35%, 20%)',
    badgeText: 'hsl(170, 70%, 55%)',
  },
  Skill: {
    fill: 'hsl(85, 60%, 48%)',
    stroke: 'hsl(85, 48%, 36%)',
    text: '#1a1a1a',
    glow: 'hsla(85, 70%, 50%, 0.3)',
    badge: 'hsl(85, 35%, 20%)',
    badgeText: 'hsl(85, 70%, 55%)',
  },
};

/** Fallback color for unknown entity types */
const DEFAULT_COLOR: EntityColor = {
  fill: 'hsl(225, 15%, 40%)',
  stroke: 'hsl(225, 12%, 30%)',
  text: '#ffffff',
  glow: 'hsla(225, 20%, 45%, 0.3)',
  badge: 'hsl(225, 12%, 18%)',
  badgeText: 'hsl(225, 20%, 55%)',
};

/**
 * Get the color palette for an entity type.
 * Generates consistent colors for unknown types via hashing.
 */
export function getEntityColor(entityType: string): EntityColor {
  if (PALETTE[entityType]) return PALETTE[entityType];

  // Generate consistent hue from type name hash
  let hash = 0;
  for (let i = 0; i < entityType.length; i++) {
    hash = entityType.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;

  return {
    fill: `hsl(${hue}, 65%, 52%)`,
    stroke: `hsl(${hue}, 50%, 40%)`,
    text: hue > 50 && hue < 200 ? '#1a1a1a' : '#ffffff',
    glow: `hsla(${hue}, 75%, 55%, 0.3)`,
    badge: `hsl(${hue}, 35%, 20%)`,
    badgeText: `hsl(${hue}, 75%, 60%)`,
  };
}

/** Get all known entity type names and their colors */
export function getAllEntityColors(): Record<string, EntityColor> {
  return { ...PALETTE };
}

/** Edge color by relationship type */
export function getEdgeColor(relType: string): string {
  const rel = relType.toUpperCase();
  if (rel.includes('WORK') || rel.includes('EMPLOY')) return 'hsl(215, 70%, 55%)';
  if (rel.includes('LIVE') || rel.includes('LOCAT'))  return 'hsl(155, 60%, 48%)';
  if (rel.includes('KNOW') || rel.includes('FRIEND')) return 'hsl(258, 65%, 58%)';
  if (rel.includes('CREATE') || rel.includes('BUILD')) return 'hsl(38, 80%, 55%)';
  if (rel.includes('PART') || rel.includes('BELONG')) return 'hsl(185, 70%, 50%)';
  if (rel.includes('USE') || rel.includes('TECH'))    return 'hsl(330, 65%, 55%)';
  return 'hsl(225, 25%, 45%)';
}

export { PALETTE, DEFAULT_COLOR };
