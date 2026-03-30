// ============================================================
// NeuroGraph — G6 Graph Options Factory
// Constructs complete GraphOptions for the G6 Graph instance
// ============================================================

import type { GraphOptions } from '@antv/g6';
import type { G6GraphData } from '../types/graph';
import { STATE_STYLES, NODE_TYPE_COLORS, TIER_RING_COLORS, EDGE_TYPE_COLORS } from './theme';

export interface GraphConfigOptions {
  container: HTMLElement | string;
  data: G6GraphData;
  width?: number;
  height?: number;
  enableMinimap?: boolean;
  enableTimebar?: boolean;
  onNodeClick?: (nodeId: string) => void;
  onEdgeClick?: (edgeId: string) => void;
  onCanvasClick?: () => void;
}

/**
 * Map node type to color from the legend palette.
 */
function getNodeColor(type: string): string {
  const key = type?.toLowerCase() || 'entity';
  return NODE_TYPE_COLORS[key] || NODE_TYPE_COLORS['entity'];
}

/**
 * Map memory tier to ring color.
 */
function getTierRingColor(tier: string): string {
  const key = tier?.toLowerCase() || 'semantic';
  return TIER_RING_COLORS[key] || TIER_RING_COLORS['semantic'];
}

/**
 * Map edge relation type to color.
 */
function getEdgeColor(relationType: string): string {
  const key = relationType?.toLowerCase() || 'entity';
  return EDGE_TYPE_COLORS[key] || EDGE_TYPE_COLORS['entity'];
}

/**
 * Compute node size from importance (0-1) → 24-56px.
 */
function getNodeSize(importance: number): number {
  const min = 24;
  const max = 56;
  const clamped = Math.max(0, Math.min(1, importance || 0.5));
  return min + clamped * (max - min);
}

/**
 * Create complete G6 GraphOptions from NeuroGraph config.
 */
export function createGraphConfig(opts: GraphConfigOptions): GraphOptions {
  const {
    container,
    data,
    width,
    height,
    enableMinimap = true,
    onNodeClick,
    onEdgeClick,
    onCanvasClick,
  } = opts;

  const plugins: GraphOptions['plugins'] = [];

  // Minimap plugin
  if (enableMinimap) {
    plugins.push({
      type: 'minimap',
      key: 'minimap',
      position: 'right-bottom' as const,
      size: [160, 100],
      style: {
        background: 'hsla(225, 25%, 8%, 0.9)',
        border: '1px solid hsla(225, 14%, 22%, 0.5)',
      },
    });
  }

  // Tooltip plugin
  plugins.push({
    type: 'tooltip',
    key: 'tooltip',
    trigger: 'hover',
    enable: (e: any) => e.targetType === 'node' || e.targetType === 'edge',
    getContent: (_: any, items: any) => {
      if (!items || items.length === 0) return '';
      const item = items[0];
      const d = item.data || {};

      if (item.source !== undefined) {
        // Edge tooltip
        const color = getEdgeColor(d.relationType || '');
        return `
          <div style="padding:8px 12px;max-width:280px;font-size:12px;line-height:1.5;color:#e0e0e0;">
            <div style="font-weight:600;margin-bottom:4px;color:${color};">${d.label || d.relationType || 'Relationship'}</div>
            <div style="color:#aaa;font-size:11px;">${d.fact || ''}</div>
            <div style="margin-top:6px;font-size:10px;color:#666;">
              Type: ${d.relationType || '—'} · Weight: ${d.weight?.toFixed(2) || '—'}
            </div>
          </div>
        `;
      }

      // Node tooltip
      const typeColor = getNodeColor(d.type || '');
      return `
        <div style="padding:10px 14px;max-width:300px;font-size:12px;line-height:1.5;color:#e0e0e0;">
          <div style="font-weight:700;font-size:14px;margin-bottom:4px;">${d.label || item.id}</div>
          <div style="display:flex;gap:6px;margin-bottom:6px;">
            <span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:10px;font-weight:600;background:${typeColor}22;color:${typeColor};border:1px solid ${typeColor}44;">
              ${d.type || 'Entity'}
            </span>
            <span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:10px;color:#888;border:1px solid #333;">
              ${d.tier || 'semantic'}
            </span>
          </div>
          <div style="display:flex;gap:12px;font-size:10px;color:#777;margin-top:4px;">
            <span>Importance: <b style="color:#ccc;">${((d.importance || 0) * 100).toFixed(0)}%</b></span>
            <span>Accessed: <b style="color:#ccc;">${d.accessCount || 0}×</b></span>
          </div>
        </div>
      `;
    },
    style: {
      '.g6-component-tooltip': {
        background: 'hsla(225, 25%, 10%, 0.95)',
        backdropFilter: 'blur(12px)',
        border: '1px solid hsla(225, 20%, 25%, 0.4)',
        borderRadius: '10px',
        boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
        padding: '0',
      },
    },
  });

  const config: GraphOptions = {
    container,
    width,
    height,
    data: data as any,
    animation: true,
    theme: 'dark',

    // ── Node options — COLOR BY TYPE, SIZE BY IMPORTANCE, LABELS ALWAYS ON ──
    node: {
      type: 'circle',
      style: (d: any) => {
        const nodeData = d.data || {};
        const color = getNodeColor(nodeData.type || '');
        const tierColor = getTierRingColor(nodeData.tier || '');
        const size = getNodeSize(nodeData.importance || 0.5);

        return {
          size,
          fill: color,
          stroke: tierColor,
          lineWidth: 2.5,
          opacity: 0.95,
          shadowColor: `${color}66`,
          shadowBlur: 12,
          shadowOffsetX: 0,
          shadowOffsetY: 4,
          // Label
          labelText: nodeData.label || d.id,
          labelFill: '#e2e8f0',
          labelFontSize: Math.max(10, size * 0.22),
          labelFontWeight: 600,
          labelPlacement: 'bottom',
          labelOffsetY: 4,
          labelBackground: true,
          labelBackgroundFill: 'rgba(2, 6, 23, 0.75)',
          labelBackgroundRadius: 4,
          labelBackgroundStroke: 'rgba(51, 65, 85, 0.3)',
          labelBackgroundLineWidth: 0.5,
          // Icon/badge for tier
          badges: [
            {
              text: nodeData.tier === 'working' ? 'W' : nodeData.tier === 'episodic' ? 'E' : nodeData.tier === 'procedural' ? 'P' : '',
              placement: 'right-top',
              fill: tierColor,
              fontSize: 8,
              fontWeight: 700,
              backgroundFill: 'rgba(2, 6, 23, 0.85)',
              backgroundRadius: 6,
              backgroundStroke: tierColor,
            },
          ],
          ...d.style,
        };
      },
      state: STATE_STYLES.node as any,
    },

    // ── Edge options — COLORED BY TYPE, ARROWS, LABELS ──
    edge: {
      type: 'quadratic',
      style: (d: any) => {
        const edgeData = d.data || {};
        const color = getEdgeColor(edgeData.relationType || '');
        const weight = edgeData.weight || 0.5;

        return {
          stroke: color,
          lineWidth: 1 + weight * 2,
          opacity: 0.6 + weight * 0.3,
          endArrow: true,
          endArrowSize: 6,
          endArrowFill: color,
          // Edge label
          labelText: edgeData.label || '',
          labelFill: '#94a3b8',
          labelFontSize: 9,
          labelFontWeight: 500,
          labelBackground: true,
          labelBackgroundFill: 'rgba(2, 6, 23, 0.8)',
          labelBackgroundRadius: 3,
          labelBackgroundStroke: 'rgba(51, 65, 85, 0.2)',
          labelBackgroundLineWidth: 0.5,
          labelOpacity: 0.7,
          ...d.style,
        };
      },
      state: STATE_STYLES.edge as any,
    },

    // ── Combo options ──
    combo: {
      type: 'circle',
      style: (d: any) => ({
        ...d.style,
      }),
      state: STATE_STYLES.combo as any,
    },

    // ── Layout — tighter for small graphs ──
    layout: {
      type: 'force',
      preventOverlap: true,
      nodeSize: 60,
      linkDistance: 120,
      nodeStrength: -400,
      edgeStrength: 0.5,
      collideStrength: 0.9,
      alphaDecay: 0.02,
      forceSimulation: null,
    },

    // ── Behaviors ──
    behaviors: [
      'drag-canvas',
      'zoom-canvas',
      'drag-element',
      {
        type: 'click-select',
        key: 'click-select',
        multiple: false,
        trigger: ['shift'],
        onClick: (event: any) => {
          const { targetType, target } = event;
          if (targetType === 'node' && onNodeClick) {
            onNodeClick(target.id);
          } else if (targetType === 'edge' && onEdgeClick) {
            onEdgeClick(target.id);
          } else if (targetType === 'canvas' && onCanvasClick) {
            onCanvasClick();
          }
        },
      },
      {
        type: 'hover-activate',
        key: 'hover-activate',
        degree: 1,
        inactiveState: 'inactive',
        activeState: 'active',
      },
    ],

    // ── Transforms ──
    transforms: [
      {
        type: 'process-parallel-edges',
        key: 'process-parallel-edges',
      },
    ],

    // ── Plugins ──
    plugins,
  };

  return config;
}
