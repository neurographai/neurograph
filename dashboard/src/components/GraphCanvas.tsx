// ============================================================
// NeuroGraph — GraphCanvas Component
// Main G6 graph visualization, handles lifecycle & interaction
// ============================================================

import { useEffect, useRef, useCallback } from 'react';
import { Graph } from '@antv/g6';
import type { G6GraphData, LayoutType } from '../types/graph';
import { createGraphConfig } from '../graph/config';

interface GraphCanvasProps {
  data: G6GraphData;
  layout: LayoutType;
  onNodeClick?: (nodeId: string) => void;
  onEdgeClick?: (edgeId: string) => void;
  onCanvasClick?: () => void;
  highlightNodeIds?: string[];
  className?: string;
}

// Global generation counter — survives React StrictMode double-invoke
let graphGeneration = 0;

export default function GraphCanvas({
  data,
  layout,
  onNodeClick,
  onEdgeClick,
  onCanvasClick,
  highlightNodeIds,
  className = '',
}: GraphCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const graphRef = useRef<Graph | null>(null);
  const genRef = useRef(0);
  const readyRef = useRef(false);

  // Build layout options from type
  const getLayoutOptions = useCallback((layoutType: LayoutType) => {
    switch (layoutType) {
      case 'force':
        return {
          type: 'force' as const,
          preventOverlap: true,
          nodeSize: 50,
          linkDistance: 150,
          nodeStrength: -600,
          edgeStrength: 0.3,
          collideStrength: 0.8,
          alphaDecay: 0.02,
          forceSimulation: null,
        };
      case 'circular':
        return {
          type: 'circular' as const,
          radius: 250,
          divisions: 1,
          ordering: 'degree' as const,
        };
      case 'radial':
        return {
          type: 'radial' as const,
          unitRadius: 100,
          linkDistance: 200,
          preventOverlap: true,
          nodeSize: 50,
        };
      case 'dagre':
        return {
          type: 'antv-dagre' as const,
          rankdir: 'TB' as const,
          nodesep: 40,
          ranksep: 60,
        };
      case 'grid':
        return {
          type: 'grid' as const,
          rows: 4,
          cols: 5,
          sortBy: 'degree',
        };
      case 'concentric':
        return {
          type: 'concentric' as const,
          maxLevelDiff: 0.5,
          preventOverlap: true,
          nodeSize: 50,
        };
      default:
        return { type: 'force' as const };
    }
  }, []);

  // Initialize graph — guards against StrictMode double-invoke
  useEffect(() => {
    if (!containerRef.current) return;

    const container = containerRef.current;
    const { width, height } = container.getBoundingClientRect();

    // Capture current generation
    const myGen = ++graphGeneration;
    genRef.current = myGen;
    readyRef.current = false;

    const config = createGraphConfig({
      container,
      data,
      width,
      height,
      enableMinimap: true,
      onNodeClick,
      onEdgeClick,
      onCanvasClick,
    });

    config.layout = getLayoutOptions(layout);

    const graph = new Graph(config);
    graphRef.current = graph;

    // Render asynchronously, but only mark ready if generation still matches
    graph.render().then(() => {
      if (genRef.current === myGen) {
        readyRef.current = true;
      }
    }).catch(() => {});

    // Resize observer with generation guard
    let resizeObs: ResizeObserver | null = new ResizeObserver((entries) => {
      if (genRef.current !== myGen) return;
      for (const entry of entries) {
        const { width: w, height: h } = entry.contentRect;
        if (w > 0 && h > 0) {
          try { graph.resize(w, h); } catch { /* graph may be destroyed */ }
        }
      }
    });
    resizeObs.observe(container);

    return () => {
      // Mark this generation as stale
      if (genRef.current === myGen) {
        genRef.current = -1;
      }
      readyRef.current = false;
      graphRef.current = null;

      resizeObs?.disconnect();
      resizeObs = null;

      // Delay destruction slightly so in-flight async ops can check generation first
      setTimeout(() => {
        try { graph.destroy(); } catch { /* already destroyed or errored */ }
      }, 0);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Update data when it changes
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || !readyRef.current) return;

    try {
      graph.setData(data as any);
      graph.render().catch(() => {});
    } catch { /* graph destroyed */ }
  }, [data]);

  // Update layout when it changes
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || !readyRef.current) return;

    try {
      graph.setLayout(getLayoutOptions(layout) as any);
      graph.layout().catch(() => {});
    } catch { /* graph destroyed */ }
  }, [layout, getLayoutOptions]);

  // Highlight nodes — fully guarded
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || !readyRef.current) return;

    try {
      const allNodes = graph.getNodeData();
      const allEdges = graph.getEdgeData();

      // Clear all highlights first
      for (const n of allNodes) {
        try { graph.setElementState(n.id, []); } catch { break; }
      }
      for (const e of allEdges) {
        if (e.id) {
          try { graph.setElementState(e.id, []); } catch { break; }
        }
      }

      if (highlightNodeIds && highlightNodeIds.length > 0) {
        const highlightSet = new Set(highlightNodeIds);

        for (const n of allNodes) {
          try {
            graph.setElementState(
              n.id,
              highlightSet.has(n.id as string) ? ['highlight'] : ['inactive'],
            );
          } catch { break; }
        }

        for (const e of allEdges) {
          if (!e.id) continue;
          try {
            const highlighted =
              highlightSet.has(e.source as string) && highlightSet.has(e.target as string);
            graph.setElementState(e.id, highlighted ? ['highlight'] : ['inactive']);
          } catch { break; }
        }
      }
    } catch {
      // Graph not ready or destroyed
    }
  }, [highlightNodeIds]);

  return (
    <div
      ref={containerRef}
      className={`ng-canvas-container ${className}`}
      id="graph-canvas"
      style={{ width: '100%', height: '100%', position: 'relative' }}
    />
  );
}
