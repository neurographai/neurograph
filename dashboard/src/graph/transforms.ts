// ============================================================
// NeuroGraph → G6 Data Transforms
// Converts Rust-style NeuroGraph data to G6's GraphData format
// ============================================================

import type {
  NeuroGraphData,
  NeuroEntity,
  NeuroRelationship,
  NeuroCommunity,
  G6GraphData,
  G6NodeDatum,
  G6EdgeDatum,
  G6ComboDatum,
} from '../types/graph';
import { getEntityColor, getEdgeColor } from './palette';

/**
 * Transform NeuroGraph entity → G6 node data
 */
function entityToNode(entity: NeuroEntity): G6NodeDatum {
  const color = getEntityColor(entity.entity_type);
  const size = mapImportanceToSize(entity.importance_score);

  return {
    id: entity.id,
    combo: Object.values(entity.community_ids)[0] || undefined,
    data: {
      entityType: entity.entity_type,
      name: entity.name,
      summary: entity.summary,
      importance: entity.importance_score,
      accessCount: entity.access_count,
      labels: entity.labels,
      createdAt: entity.created_at,
      updatedAt: entity.updated_at,
      attributes: entity.attributes,
      timestamp: new Date(entity.created_at).getTime(),
    },
    style: {
      size,
      fill: color.fill,
      stroke: color.stroke,
      lineWidth: 2,
      labelText: entity.name,
      labelFill: '#e0e0e0',
      labelFontSize: 11,
      labelFontWeight: 500,
      labelOffsetY: size / 2 + 14,
      labelBackground: true,
      labelBackgroundFill: 'rgba(10, 12, 18, 0.8)',
      labelBackgroundRadius: 4,
      labelBackgroundPadding: [2, 6, 2, 6],
      iconText: getEntityIcon(entity.entity_type),
      iconFontSize: size * 0.4,
      iconFill: color.text,
      shadowColor: color.glow,
      shadowBlur: 12,
      shadowOffsetX: 0,
      shadowOffsetY: 0,
      badges: [
        {
          text: entity.entity_type,
          placement: 'right-top' as const,
          fill: color.badge,
          textFill: color.badgeText,
          fontSize: 8,
          padding: [1, 4, 1, 4],
        },
      ],
    },
  };
}

/**
 * Transform NeuroGraph relationship → G6 edge data
 */
function relationshipToEdge(rel: NeuroRelationship): G6EdgeDatum {
  const edgeColor = getEdgeColor(rel.relationship_type);
  const isExpired = rel.valid_until !== null;

  return {
    id: rel.id,
    source: rel.source_entity_id,
    target: rel.target_entity_id,
    data: {
      relationshipType: rel.relationship_type,
      fact: rel.fact,
      weight: rel.weight,
      confidence: rel.confidence,
      validFrom: rel.valid_from,
      validUntil: rel.valid_until,
      timestamp: rel.valid_from ? new Date(rel.valid_from).getTime() : Date.now(),
    },
    style: {
      stroke: edgeColor,
      lineWidth: Math.max(1, rel.weight * 2),
      opacity: isExpired ? 0.3 : 0.7,
      lineDash: isExpired ? [6, 4] : undefined,
      endArrow: true,
      endArrowSize: 6,
      endArrowFill: edgeColor,
      labelText: formatRelType(rel.relationship_type),
      labelFontSize: 9,
      labelFill: 'hsl(225, 10%, 55%)',
      labelBackground: true,
      labelBackgroundFill: 'rgba(10, 12, 18, 0.85)',
      labelBackgroundRadius: 3,
      labelBackgroundPadding: [1, 4, 1, 4],
      halo: false,
      haloStroke: edgeColor,
      haloStrokeOpacity: 0.2,
      haloLineWidth: 8,
    },
  };
}

/**
 * Transform NeuroGraph community → G6 combo data
 */
function communityToCombo(community: NeuroCommunity): G6ComboDatum {
  return {
    id: community.id,
    data: {
      name: community.name,
      summary: community.summary,
      level: community.level,
      memberCount: community.member_entity_ids.length,
    },
    style: {
      fill: 'hsla(225, 18%, 14%, 0.2)',
      stroke: 'hsla(225, 14%, 25%, 0.5)',
      lineWidth: 1,
      lineDash: [4, 4],
      collapsedFill: 'hsla(225, 18%, 14%, 0.4)',
      labelText: community.name,
      labelFill: 'hsl(225, 10%, 50%)',
      labelFontSize: 11,
      labelFontWeight: 600,
      labelBackground: true,
      labelBackgroundFill: 'rgba(10, 12, 18, 0.7)',
      labelBackgroundRadius: 4,
      labelBackgroundPadding: [2, 8, 2, 8],
    },
  };
}

/**
 * Full transform: NeuroGraphData → G6GraphData
 */
export function transformToG6(data: NeuroGraphData): G6GraphData {
  return {
    nodes: data.entities.map(entityToNode),
    edges: data.relationships.map(relationshipToEdge),
    combos: data.communities.map(communityToCombo),
  };
}

// ── Helpers ──

/** Map importance score (0-1) to node pixel size */
function mapImportanceToSize(importance: number): number {
  const MIN_SIZE = 28;
  const MAX_SIZE = 56;
  return MIN_SIZE + (MAX_SIZE - MIN_SIZE) * Math.pow(importance, 0.7);
}

/** Format relationship type for edge label */
function formatRelType(type: string): string {
  return type
    .replace(/_/g, ' ')
    .toLowerCase()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Get emoji icon for entity type */
function getEntityIcon(type: string): string {
  const icons: Record<string, string> = {
    Person: '👤',
    Organization: '🏢',
    Location: '📍',
    Event: '📅',
    Concept: '💡',
    Technology: '⚙️',
    Product: '📦',
    Document: '📄',
    Topic: '🏷️',
    Skill: '🎯',
  };
  return icons[type] || '◉';
}

export { entityToNode, relationshipToEdge, communityToCombo };
