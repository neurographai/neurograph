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
  const resizeObserverRef = useRef<ResizeObserver | null>(null);

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

  // Initialize graph
  useEffect(() => {
    if (!containerRef.current) return;

    const container = containerRef.current;
    const { width, height } = container.getBoundingClientRect();

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

    // Override layout with the selected type
    config.layout = getLayoutOptions(layout);

    const graph = new Graph(config);
    graphRef.current = graph;

    graph.render();

    // Resize observer
    resizeObserverRef.current = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width: w, height: h } = entry.contentRect;
        if (w > 0 && h > 0) {
          graph.resize(w, h);
        }
      }
    });
    resizeObserverRef.current.observe(container);

    return () => {
      resizeObserverRef.current?.disconnect();
      graph.destroy();
      graphRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Update data when it changes
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph) return;

    graph.setData(data as any);
    graph.render();
  }, [data]);

  // Update layout when it changes
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph) return;

    graph.setLayout(getLayoutOptions(layout) as any);
    graph.layout();
  }, [layout, getLayoutOptions]);

  // Highlight nodes
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph) return;

    // Clear all highlights first
    try {
      const allNodes = graph.getNodeData();
      const allEdges = graph.getEdgeData();
      allNodes.forEach((n: any) => graph.setElementState(n.id, []));
      allEdges.forEach((e: any) => {
        if (e.id) graph.setElementState(e.id, []);
      });

      if (highlightNodeIds && highlightNodeIds.length > 0) {
        const highlightSet = new Set(highlightNodeIds);

        allNodes.forEach((n: any) => {
          if (highlightSet.has(n.id as string)) {
            graph.setElementState(n.id, ['highlight']);
          } else {
            graph.setElementState(n.id, ['inactive']);
          }
        });

        allEdges.forEach((e: any) => {
          if (e.id && highlightSet.has(e.source as string) && highlightSet.has(e.target as string)) {
            graph.setElementState(e.id, ['highlight']);
          } else if (e.id) {
            graph.setElementState(e.id, ['inactive']);
          }
        });
      }
    } catch {
      // Graph may not be ready yet
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
