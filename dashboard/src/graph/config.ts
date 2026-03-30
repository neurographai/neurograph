// ============================================================
// NeuroGraph — G6 Graph Options Factory
// Constructs complete GraphOptions for the G6 Graph instance
// ============================================================

import type { GraphOptions } from '@antv/g6';
import type { G6GraphData } from '../types/graph';
import { STATE_STYLES } from './theme';

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
      const data = item.data || {};

      if (item.source !== undefined) {
        // Edge
        return `
          <div style="padding:8px 12px;max-width:280px;font-size:12px;line-height:1.5;color:#e0e0e0;">
            <div style="font-weight:600;margin-bottom:4px;color:hsl(258,90%,72%);">${data.relationshipType || 'Relationship'}</div>
            <div style="color:#aaa;">${data.fact || ''}</div>
            <div style="margin-top:6px;font-size:11px;color:#666;">
              Weight: ${data.weight?.toFixed(2) || '—'} · Confidence: ${(data.confidence * 100)?.toFixed(0) || '—'}%
            </div>
          </div>
        `;
      }

      // Node
      return `
        <div style="padding:8px 12px;max-width:300px;font-size:12px;line-height:1.5;color:#e0e0e0;">
          <div style="font-weight:600;font-size:14px;margin-bottom:4px;">${data.name || item.id}</div>
          <div style="display:inline-block;padding:1px 6px;border-radius:10px;font-size:10px;background:hsl(258,50%,28%);color:hsl(258,90%,72%);margin-bottom:6px;">
            ${data.entityType || ''}
          </div>
          <div style="color:#aaa;font-size:11px;">${data.summary ? data.summary.slice(0, 150) + (data.summary.length > 150 ? '…' : '') : ''}</div>
          <div style="margin-top:6px;font-size:10px;color:#555;">
            Importance: ${(data.importance * 100)?.toFixed(0) || '—'}% · Accessed: ${data.accessCount || 0}×
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

    // ── Node options ──
    node: {
      type: 'circle',
      style: (d: any) => ({
        ...d.style,
      }),
      state: STATE_STYLES.node as any,
    },

    // ── Edge options ──
    edge: {
      type: 'quadratic',
      style: (d: any) => ({
        ...d.style,
      }),
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

    // ── Layout ──
    layout: {
      type: 'force',
      preventOverlap: true,
      nodeSize: 50,
      linkDistance: 150,
      nodeStrength: -600,
      edgeStrength: 0.3,
      collideStrength: 0.8,
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
