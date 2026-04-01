import React, { useState, useRef, useCallback, useEffect } from "react";

// ════════════════════════════════════════════════════════════════════════════
// EmbeddingPanel — Real-Time PDF → Embedding → Graph Visualization
// v2: Zoom/Pan, Focus Mode, LOD, Importance Scaling
// ════════════════════════════════════════════════════════════════════════════

/* ── Types ───────────────────────────────────────────────────────────────── */

interface GraphNode {
  id: string;
  label: string;
  category: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  degree: number;       // connection count
  importance: number;   // normalized 0-1
}

interface GraphEdge {
  source: string;
  target: string;
  similarity: number;
}

interface PipelineStats {
  total_chunks: number;
  total_nodes: number;
  concept_nodes: number;
  chunk_nodes: number;
  total_edges: number;
  model: string;
  threshold: number;
  text_length: number;
}

type PipelineStep =
  | "idle"
  | "uploading"
  | "extracting"
  | "chunking"
  | "embedding"
  | "graphing"
  | "done"
  | "error";

const STEP_LABELS: Record<PipelineStep, string> = {
  idle: "Ready",
  uploading: "Uploading PDF…",
  extracting: "Extracting text…",
  chunking: "Chunking…",
  embedding: "Embedding concepts…",
  graphing: "Building graph…",
  done: "Complete ✓",
  error: "Error ✗",
};

const CATEGORY_COLORS: Record<string, string> = {
  entity: "#818cf8",
  acronym: "#fbbf24",
  concept: "#34d399",
  chunk: "#64748b",
};

// Use relative URLs — Vite proxy handles /api and /ws in dev,
// same-origin in production (dashboard served from core server).
const API_BASE = "/api/v1";
function wsBase() {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  return `${proto}://${location.host}`;
}

/* ── Camera (zoom/pan) ──────────────────────────────────────────────────── */

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

function screenToWorld(sx: number, sy: number, cam: Camera): [number, number] {
  return [(sx - cam.x) / cam.zoom, (sy - cam.y) / cam.zoom];
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function worldToScreen(wx: number, wy: number, cam: Camera): [number, number] {
  return [wx * cam.zoom + cam.x, wy * cam.zoom + cam.y];
}

/* ── Force simulation ────────────────────────────────────────────────────── */

function simulateForces(
  nodes: GraphNode[],
  edges: GraphEdge[],
  w: number,
  h: number,
  draggingId: string | null,
) {
  const cx = w / 2, cy = h / 2;
  const nodeMap = new Map(nodes.map((n) => [n.id, n]));

  for (const n of nodes) {
    if (n.id === draggingId) continue;
    n.vx *= 0.88;
    n.vy *= 0.88;
  }

  // Repulsion — stronger for important nodes
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const a = nodes[i], b = nodes[j];
      let dx = b.x - a.x, dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = 2000 / (dist * dist);
      dx /= dist; dy /= dist;
      if (a.id !== draggingId) { a.vx -= dx * force; a.vy -= dy * force; }
      if (b.id !== draggingId) { b.vx += dx * force; b.vy += dy * force; }
    }
  }

  // Attraction
  for (const e of edges) {
    const a = nodeMap.get(e.source);
    const b = nodeMap.get(e.target);
    if (!a || !b) continue;
    let dx = b.x - a.x, dy = b.y - a.y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    const ideal = 100 + (1 - e.similarity) * 150;
    const force = (dist - ideal) * 0.008 * e.similarity;
    dx /= dist; dy /= dist;
    if (a.id !== draggingId) { a.vx += dx * force; a.vy += dy * force; }
    if (b.id !== draggingId) { b.vx -= dx * force; b.vy -= dy * force; }
  }

  // Center gravity
  for (const n of nodes) {
    if (n.id === draggingId) continue;
    n.vx += (cx - n.x) * 0.001;
    n.vy += (cy - n.y) * 0.001;
  }

  // Integrate
  for (const n of nodes) {
    if (n.id === draggingId) continue;
    n.x += n.vx;
    n.y += n.vy;
    const margin = 50;
    n.x = Math.max(margin, Math.min(w - margin, n.x));
    n.y = Math.max(margin, Math.min(h - margin, n.y));
  }
}

/* ── Degree / importance calculation ─────────────────────────────────────── */

function computeDegrees(nodes: GraphNode[], edges: GraphEdge[]) {
  const degMap = new Map<string, number>();
  for (const e of edges) {
    degMap.set(e.source, (degMap.get(e.source) || 0) + 1);
    degMap.set(e.target, (degMap.get(e.target) || 0) + 1);
  }
  const maxDeg = Math.max(1, ...degMap.values());
  for (const n of nodes) {
    n.degree = degMap.get(n.id) || 0;
    n.importance = n.degree / maxDeg;
    // Scale radius by importance: 4–18px
    const base = n.category === "chunk" ? 3 : 6;
    n.radius = base + n.importance * 14;
  }
}

/* ── Canvas Renderer (LOD-aware) ─────────────────────────────────────────── */

function drawGraph(
  ctx: CanvasRenderingContext2D,
  nodes: GraphNode[],
  edges: GraphEdge[],
  cam: Camera,
  hoveredId: string | null,
  focusedId: string | null,
  w: number,
  h: number,
) {
  ctx.clearRect(0, 0, w, h);
  ctx.save();
  ctx.translate(cam.x, cam.y);
  ctx.scale(cam.zoom, cam.zoom);

  const nodeMap = new Map(nodes.map((n) => [n.id, n]));

  // Build focus set (1-hop neighbors)
  let focusSet: Set<string> | null = null;
  let focusEdges: Set<string> | null = null;
  if (focusedId) {
    focusSet = new Set([focusedId]);
    focusEdges = new Set<string>();
    for (const e of edges) {
      if (e.source === focusedId || e.target === focusedId) {
        focusSet.add(e.source);
        focusSet.add(e.target);
        focusEdges.add(`${e.source}-${e.target}`);
      }
    }
  }

  // LOD: only show labels above a certain zoom threshold
  const showAllLabels = cam.zoom > 0.8;
  const showImportantLabels = cam.zoom > 0.4;

  // ── EDGES ──────────────────────────────────────
  for (const e of edges) {
    const s = nodeMap.get(e.source);
    const t = nodeMap.get(e.target);
    if (!s || !t) continue;

    const isFocusEdge = focusEdges?.has(`${e.source}-${e.target}`);
    const dimmed = focusedId && !isFocusEdge;
    const isHoverEdge = hoveredId && (hoveredId === s.id || hoveredId === t.id);

    if (dimmed && !isHoverEdge) {
      ctx.strokeStyle = "rgba(50, 60, 80, 0.06)";
      ctx.lineWidth = 0.3;
    } else if (isFocusEdge) {
      const alpha = 0.4 + e.similarity * 0.5;
      ctx.strokeStyle = `rgba(0, 245, 212, ${alpha})`;
      ctx.lineWidth = 1 + e.similarity * 3;
    } else if (isHoverEdge) {
      ctx.strokeStyle = `rgba(120, 170, 255, ${0.3 + e.similarity * 0.4})`;
      ctx.lineWidth = 0.8 + e.similarity * 2;
    } else {
      const alpha = 0.04 + e.similarity * 0.12;
      ctx.strokeStyle = `rgba(100, 130, 170, ${alpha})`;
      ctx.lineWidth = 0.3 + e.similarity * 1.2;
    }

    ctx.beginPath();
    ctx.moveTo(s.x, s.y);
    ctx.lineTo(t.x, t.y);
    ctx.stroke();

    // Show similarity % on focus/hover edges at medium+ zoom
    if ((isFocusEdge || isHoverEdge) && cam.zoom > 0.5) {
      const mx = (s.x + t.x) / 2, my = (s.y + t.y) / 2;
      ctx.font = `${9 / cam.zoom}px Inter, sans-serif`;
      ctx.fillStyle = "rgba(200, 220, 240, 0.6)";
      ctx.textAlign = "center";
      ctx.fillText(`${(e.similarity * 100).toFixed(0)}%`, mx, my - 4 / cam.zoom);
    }
  }

  // ── NODES ──────────────────────────────────────
  for (const n of nodes) {
    const isH = n.id === hoveredId;
    const isFocus = n.id === focusedId;
    const inFocusSet = focusSet?.has(n.id);
    const dimmedNode = focusedId && !inFocusSet;

    const r = (isH || isFocus) ? n.radius * 1.3 : n.radius;
    const color = CATEGORY_COLORS[n.category] ?? "#94a3b8";

    if (dimmedNode) {
      // Faded node
      ctx.beginPath();
      ctx.arc(n.x, n.y, r * 0.7, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(30, 40, 55, 0.3)";
      ctx.fill();
      continue;
    }

    // Glow for hovered / focused
    if (isH || isFocus) {
      ctx.shadowColor = isFocus ? "#00f5d4" : color;
      ctx.shadowBlur = isFocus ? 25 : 16;
    }

    // Outer ring — scales with importance
    const ringAlpha = Math.floor(100 + n.importance * 155).toString(16).padStart(2, "0");
    ctx.beginPath();
    ctx.arc(n.x, n.y, r + 2, 0, Math.PI * 2);
    ctx.fillStyle = color.slice(0, 7) + ringAlpha;
    ctx.fill();

    // Inner circle
    const innerAlpha = (isH || isFocus) ? "ff" : Math.floor(160 + n.importance * 95).toString(16).padStart(2, "0");
    ctx.beginPath();
    ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
    ctx.fillStyle = color.slice(0, 7) + innerAlpha;
    ctx.fill();
    ctx.strokeStyle = "#0d111780";
    ctx.lineWidth = 1;
    ctx.stroke();

    ctx.shadowBlur = 0;

    // ── LABELS (LOD) ──────────────────────────────
    const shouldLabel =
      isH || isFocus || inFocusSet ||
      (showAllLabels && n.category !== "chunk") ||
      (showImportantLabels && n.importance > 0.4);

    if (shouldLabel) {
      const fontSize = Math.max(9, Math.min(13, 10 + n.importance * 4));
      const isBold = isH || isFocus || n.importance > 0.6;
      ctx.font = `${isBold ? "bold " : ""}${fontSize}px Inter, sans-serif`;
      ctx.textAlign = "center";
      ctx.fillStyle = isH || isFocus
        ? "#ffffff"
        : `rgba(220, 230, 240, ${0.5 + n.importance * 0.5})`;
      const lbl = n.label.length > 22 ? n.label.slice(0, 20) + "…" : n.label;
      ctx.fillText(lbl, n.x, n.y + r + 14);
    }
  }

  // ── TOOLTIP for hovered node ──────────────────────
  if (hoveredId) {
    const n = nodeMap.get(hoveredId);
    if (n) {
      const lines = [
        n.label,
        `Category: ${n.category}`,
        `Connections: ${n.degree}`,
        `Importance: ${(n.importance * 100).toFixed(0)}%`,
      ];
      const fs = 11;
      ctx.font = `${fs}px Inter, sans-serif`;
      const maxW = Math.max(...lines.map((l) => ctx.measureText(l).width));

      const tx = n.x + n.radius + 14;
      const ty = n.y - 35;
      const tooltipColor = CATEGORY_COLORS[n.category] ?? "#94a3b8";

      ctx.fillStyle = "rgba(10, 15, 30, 0.94)";
      ctx.strokeStyle = tooltipColor + "60";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.roundRect(tx - 8, ty - 4, maxW + 24, lines.length * 18 + 12, 8);
      ctx.fill();
      ctx.stroke();

      lines.forEach((line, i) => {
        ctx.font = i === 0 ? `bold ${fs + 1}px Inter, sans-serif` : `${fs}px Inter, sans-serif`;
        ctx.fillStyle = i === 0 ? tooltipColor : "#aabbcc";
        ctx.textAlign = "left";
        ctx.fillText(line, tx, ty + 14 + i * 18);
      });
    }
  }

  ctx.restore();
}

/* ── Component ───────────────────────────────────────────────────────────── */

export const EmbeddingPanel: React.FC = () => {
  const [step, setStep] = useState<PipelineStep>("idle");
  const [progress, setProgress] = useState(0);
  const [statusMsg, setStatusMsg] = useState("");
  const [fileId, setFileId] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [model, setModel] = useState(
    localStorage.getItem("ng_model") || "qwen3-embedding:8b",
  );
  const [threshold, setThreshold] = useState(0.82);
  const [models, setModels] = useState<string[]>([]);
  const [stats, setStats] = useState<PipelineStats | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const [focusedNode, setFocusedNode] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [dragging, setDragging] = useState<string | null>(null);
  const [panning, setPanning] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const nodesRef = useRef<GraphNode[]>([]);
  const edgesRef = useRef<GraphEdge[]>([]);
  const [nodeCount, setNodeCount] = useState(0);
  const [edgeCount, setEdgeCount] = useState(0);
  const animRef = useRef(0);
  const sizeRef = useRef({ w: 900, h: 600 });
  const mouseRef = useRef({ x: 0, y: 0 });
  const camRef = useRef<Camera>({ x: 0, y: 0, zoom: 1 });
  const panStartRef = useRef({ x: 0, y: 0, cx: 0, cy: 0 });

  /* ── Fetch Ollama models ── */
  useEffect(() => {
    fetch(`${API_BASE}/models`)
      .then((r) => r.json())
      .then((d) => {
        if (d.models?.length) setModels(d.models);
      })
      .catch(() => setModels(["qwen3-embedding:8b"]));
  }, []);

  /* ── Resize observer ── */
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const obs = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      if (width > 0 && height > 0) {
        sizeRef.current = { w: width, h: height };
      }
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  /* ── Animation loop ── */
  useEffect(() => {
    const loop = () => {
      const canvas = canvasRef.current;
      if (!canvas) { animRef.current = requestAnimationFrame(loop); return; }
      const ctx = canvas.getContext("2d");
      if (!ctx) { animRef.current = requestAnimationFrame(loop); return; }

      const { w, h } = sizeRef.current;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      ctx.scale(dpr, dpr);

      const nodes = nodesRef.current;
      const edges = edgesRef.current;

      if (nodes.length > 0) {
        simulateForces(nodes, edges, w, h, dragging);
      }

      drawGraph(ctx, nodes, edges, camRef.current, hoveredNode, focusedNode, w, h);

      animRef.current = requestAnimationFrame(loop);
    };
    animRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(animRef.current);
  }, [hoveredNode, focusedNode, dragging]);

  /* ── Find node at screen position ── */
  const findNodeAt = useCallback((sx: number, sy: number) => {
    const cam = camRef.current;
    const [wx, wy] = screenToWorld(sx, sy, cam);
    const nodes = nodesRef.current;
    for (let i = nodes.length - 1; i >= 0; i--) {
      const n = nodes[i];
      const dx = wx - n.x, dy = wy - n.y;
      const hitR = (n.radius + 5) / cam.zoom;
      if (dx * dx + dy * dy <= hitR * hitR) return n;
    }
    return null;
  }, []);

  /* ── File upload ── */
  const handleFile = useCallback(async (file: File) => {
    if (!file.name.toLowerCase().endsWith(".pdf")) {
      setErrorMsg("Please upload a PDF file.");
      return;
    }
    setStep("uploading");
    setProgress(5);
    setStatusMsg("Uploading…");
    nodesRef.current = [];
    edgesRef.current = [];
    setNodeCount(0);
    setEdgeCount(0);
    setStats(null);
    setErrorMsg(null);
    setFileName(file.name);
    setFocusedNode(null);

    try {
      const fd = new FormData();
      fd.append("file", file);
      const resp = await fetch(`${API_BASE}/upload`, { method: "POST", body: fd });
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({ error: "Upload failed" }));
        throw new Error(err.error ?? "Upload failed");
      }
      const data = await resp.json();
      setFileId(data.file_id);
      setStatusMsg(`Uploaded ${file.name} (${(data.size_bytes / 1024).toFixed(1)} KB)`);
      setProgress(10);
      setStep("idle");
    } catch (e: any) {
      setStep("error");
      setErrorMsg(e.message);
    }
  }, []);

  /* ── Drag & drop ── */
  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const f = e.dataTransfer.files?.[0];
      if (f) handleFile(f);
    },
    [handleFile],
  );

  /* ── WebSocket pipeline ── */
  const startProcessing = useCallback(() => {
    if (!fileId) return;
    setStep("extracting");
    setProgress(0);
    nodesRef.current = [];
    edgesRef.current = [];
    setNodeCount(0);
    setEdgeCount(0);
    setStats(null);
    setErrorMsg(null);
    setFocusedNode(null);
    localStorage.setItem("ng_model", model);

    // Reset camera
    camRef.current = { x: 0, y: 0, zoom: 1 };

    const url =
      `${wsBase()}/ws/process` +
      `?file_id=${fileId}` +
      `&model=${encodeURIComponent(model)}` +
      `&threshold=${threshold}`;

    const ws = new WebSocket(url);

    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      switch (msg.type) {
        case "status":
          setStep(msg.step as PipelineStep);
          if (msg.progress != null) setProgress(msg.progress);
          if (msg.message) setStatusMsg(msg.message);
          break;
        case "node": {
          const { w, h } = sizeRef.current;
          const newNode: GraphNode = {
            id: msg.id,
            label: msg.label,
            category: msg.category,
            x: msg.x ?? w / 2 + (Math.random() - 0.5) * 400,
            y: msg.y ?? h / 2 + (Math.random() - 0.5) * 400,
            vx: 0,
            vy: 0,
            radius: msg.category === "chunk" ? 4 : 8,
            degree: 0,
            importance: 0,
          };
          nodesRef.current = [...nodesRef.current, newNode];
          setNodeCount((c) => c + 1);
          break;
        }
        case "edge":
          edgesRef.current = [
            ...edgesRef.current,
            { source: msg.source, target: msg.target, similarity: msg.similarity },
          ];
          setEdgeCount((c) => c + 1);
          // Recompute degrees periodically
          if (edgesRef.current.length % 20 === 0) {
            computeDegrees(nodesRef.current, edgesRef.current);
          }
          break;
        case "done":
          setStep("done");
          setProgress(100);
          setStatusMsg(msg.message ?? "Done!");
          if (msg.stats) setStats(msg.stats);
          // Final degree computation
          computeDegrees(nodesRef.current, edgesRef.current);
          break;
        case "error":
          setStep("error");
          setErrorMsg(msg.message);
          break;
      }
    };

    ws.onerror = () => {
      setStep("error");
      setErrorMsg("WebSocket error. Is the NeuroGraph server running?");
    };
  }, [fileId, model, threshold]);

  /* ── Canvas mouse handlers ── */
  const onMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const sx = e.clientX - rect.left;
      const sy = e.clientY - rect.top;
      mouseRef.current = { x: sx, y: sy };

      if (panning) {
        const dx = sx - panStartRef.current.x;
        const dy = sy - panStartRef.current.y;
        camRef.current = {
          ...camRef.current,
          x: panStartRef.current.cx + dx,
          y: panStartRef.current.cy + dy,
        };
        return;
      }

      if (dragging) {
        const [wx, wy] = screenToWorld(sx, sy, camRef.current);
        const node = nodesRef.current.find((n) => n.id === dragging);
        if (node) {
          node.x = wx;
          node.y = wy;
          node.vx = 0;
          node.vy = 0;
        }
      } else {
        const found = findNodeAt(sx, sy);
        setHoveredNode(found?.id ?? null);
      }
    },
    [dragging, panning, findNodeAt],
  );

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const sx = e.clientX - rect.left;
      const sy = e.clientY - rect.top;

      const node = findNodeAt(sx, sy);
      if (node) {
        setDragging(node.id);
      } else {
        // Start panning
        setPanning(true);
        panStartRef.current = {
          x: sx,
          y: sy,
          cx: camRef.current.x,
          cy: camRef.current.y,
        };
      }
    },
    [findNodeAt],
  );

  const onMouseUp = useCallback(() => {
    setDragging(null);
    setPanning(false);
  }, []);

  const onClick = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const node = findNodeAt(e.clientX - rect.left, e.clientY - rect.top);
      if (node) {
        setFocusedNode((prev) => (prev === node.id ? null : node.id));
      } else {
        setFocusedNode(null);
      }
    },
    [findNodeAt],
  );

  const onDblClick = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const node = findNodeAt(e.clientX - rect.left, e.clientY - rect.top);
      if (node) {
        // Center + zoom to node
        const { w, h } = sizeRef.current;
        camRef.current = {
          x: w / 2 - node.x * 2,
          y: h / 2 - node.y * 2,
          zoom: 2,
        };
        setFocusedNode(node.id);
      }
    },
    [findNodeAt],
  );

  const onWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const cam = camRef.current;

    const factor = e.deltaY < 0 ? 1.12 : 0.89;
    const newZoom = Math.max(0.15, Math.min(6, cam.zoom * factor));

    // Zoom towards cursor
    camRef.current = {
      x: sx - (sx - cam.x) * (newZoom / cam.zoom),
      y: sy - (sy - cam.y) * (newZoom / cam.zoom),
      zoom: newZoom,
    };
  }, []);

  const resetView = useCallback(() => {
    camRef.current = { x: 0, y: 0, zoom: 1 };
    setFocusedNode(null);
  }, []);

  const isProcessing = ["extracting", "chunking", "embedding", "graphing"].includes(step);
  const canProcess = fileId != null && !isProcessing;

  /* ── Render ── */
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "hidden",
        fontFamily: "'Inter', system-ui, sans-serif",
      }}
    >
      {/* ═══ Toolbar ═══ */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "8px 14px",
          background: "rgba(30, 41, 59, 0.6)",
          borderBottom: "1px solid rgba(255,255,255,0.06)",
          flexWrap: "wrap",
          flexShrink: 0,
        }}
      >
        {/* Upload */}
        <label
          onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
          onDragLeave={() => setDragOver(false)}
          onDrop={onDrop}
          style={{
            padding: "5px 12px",
            border: dragOver ? "2px dashed #818cf8" : "2px dashed rgba(255,255,255,0.15)",
            borderRadius: 8,
            cursor: "pointer",
            fontSize: 12,
            background: dragOver ? "rgba(129, 140, 248, 0.1)" : "transparent",
            transition: "all .2s",
            whiteSpace: "nowrap",
            color: "#e0e1dd",
          }}
        >
          {fileName ? `📄 ${fileName}` : "📎 Drop PDF or click"}
          <input
            type="file"
            accept=".pdf"
            hidden
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) handleFile(f);
            }}
          />
        </label>

        {/* Model selector */}
        <select
          value={model}
          onChange={(e) => setModel(e.target.value)}
          disabled={isProcessing}
          style={{
            background: "rgba(15, 23, 42, 0.8)",
            color: "#e0e1dd",
            border: "1px solid rgba(255,255,255,0.12)",
            borderRadius: 6,
            padding: "5px 8px",
            fontSize: 12,
          }}
        >
          {(models.length > 0 ? models : ["qwen3-embedding:8b"]).map((m) => (
            <option key={m} value={m}>{m}</option>
          ))}
        </select>

        {/* Threshold */}
        <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11 }}>
          <span style={{ color: "#778da9" }}>Sim</span>
          <input
            type="range"
            min={0.5}
            max={0.98}
            step={0.02}
            value={threshold}
            onChange={(e) => setThreshold(parseFloat(e.target.value))}
            disabled={isProcessing}
            style={{ width: 60, accentColor: "#818cf8" }}
          />
          <span style={{ fontVariantNumeric: "tabular-nums", minWidth: 28, color: "#aab" }}>
            {threshold.toFixed(2)}
          </span>
        </div>

        {/* Process button */}
        <button
          onClick={startProcessing}
          disabled={!canProcess}
          style={{
            padding: "5px 16px",
            borderRadius: 6,
            border: "none",
            background: canProcess
              ? "linear-gradient(135deg, #818cf8, #7c3aed)"
              : "rgba(51, 65, 85, 0.5)",
            color: canProcess ? "#fff" : "#64748b",
            fontWeight: 600,
            fontSize: 12,
            cursor: canProcess ? "pointer" : "not-allowed",
            transition: "all 0.2s",
          }}
        >
          {isProcessing ? "⏳ Processing…" : "▶ Process & Visualize"}
        </button>

        {/* Reset view */}
        {nodeCount > 0 && (
          <button
            onClick={resetView}
            style={{
              padding: "4px 10px",
              borderRadius: 5,
              border: "1px solid rgba(255,255,255,0.1)",
              background: "transparent",
              color: "#94a3b8",
              fontSize: 11,
              cursor: "pointer",
            }}
          >
            ⟳ Reset View
          </button>
        )}

        {/* Status + progress */}
        <div style={{ flex: 1, minWidth: 140 }}>
          <div style={{ fontSize: 11, color: "#778da9", marginBottom: 2 }}>
            {STEP_LABELS[step]}{statusMsg ? ` — ${statusMsg}` : ""}
          </div>
          <div
            style={{
              height: 3,
              borderRadius: 2,
              background: "rgba(255,255,255,0.06)",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                height: "100%",
                width: `${progress}%`,
                background:
                  step === "error" ? "#ef4444"
                    : step === "done" ? "#10b981"
                    : "linear-gradient(90deg, #818cf8, #00f5d4)",
                transition: "width .3s ease",
                borderRadius: 2,
              }}
            />
          </div>
        </div>

        {/* Live counters */}
        {nodeCount > 0 && (
          <div style={{ fontSize: 11, color: "#64748b", whiteSpace: "nowrap" }}>
            <span style={{ color: "#34d399" }}>{nodeCount}</span> nodes ·{" "}
            <span style={{ color: "#818cf8" }}>{edgeCount}</span> edges ·{" "}
            <span style={{ color: "#94a3b8" }}>{Math.round(camRef.current.zoom * 100)}%</span>
          </div>
        )}
      </div>

      {/* ═══ Error ═══ */}
      {errorMsg && (
        <div style={{
          padding: "6px 14px",
          background: "rgba(127, 29, 29, 0.6)",
          color: "#fca5a5",
          fontSize: 12,
          borderBottom: "1px solid rgba(239, 68, 68, 0.2)",
          flexShrink: 0,
        }}>
          ⚠ {errorMsg}
        </div>
      )}

      {/* ═══ Stats ═══ */}
      {stats && (
        <div style={{
          display: "flex",
          gap: 18,
          padding: "6px 14px",
          fontSize: 11,
          color: "#778da9",
          borderBottom: "1px solid rgba(255,255,255,0.04)",
          flexWrap: "wrap",
          flexShrink: 0,
        }}>
          <span><b style={{ color: "#00f5d4" }}>{stats.total_nodes}</b> nodes</span>
          <span><b style={{ color: "#00f5d4" }}>{stats.total_edges}</b> edges</span>
          <span><b style={{ color: "#e0e1dd" }}>{stats.total_chunks}</b> chunks</span>
          <span>Model: <b style={{ color: "#e0e1dd" }}>{stats.model}</b></span>
          <span>Threshold: <b style={{ color: "#e0e1dd" }}>{stats.threshold}</b></span>
          <span>Text: <b style={{ color: "#e0e1dd" }}>{(stats.text_length / 1024).toFixed(1)} KB</b></span>
        </div>
      )}

      {/* ═══ Legend ═══ */}
      <div style={{
        display: "flex",
        gap: 14,
        padding: "5px 14px",
        fontSize: 10,
        color: "#778da9",
        flexShrink: 0,
        alignItems: "center",
      }}>
        {Object.entries(CATEGORY_COLORS).map(([cat, col]) => (
          <span key={cat} style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <span style={{
              display: "inline-block", width: 7, height: 7,
              borderRadius: "50%", background: col,
            }} />
            {cat}
          </span>
        ))}
        <span style={{ marginLeft: "auto", fontSize: 9, color: "#475569" }}>
          Scroll=Zoom · Drag=Pan · Click=Focus · DblClick=Zoom In
        </span>
      </div>

      {/* ═══ Canvas ═══ */}
      <div
        ref={containerRef}
        style={{
          flex: 1,
          position: "relative",
          minHeight: 300,
          cursor: panning ? "grabbing" : dragging ? "grabbing" : hoveredNode ? "grab" : "crosshair",
        }}
      >
        <canvas
          ref={canvasRef}
          style={{ width: "100%", height: "100%", display: "block" }}
          onMouseMove={onMouseMove}
          onMouseDown={onMouseDown}
          onMouseUp={onMouseUp}
          onMouseLeave={() => { setHoveredNode(null); setDragging(null); setPanning(false); }}
          onClick={onClick}
          onDoubleClick={onDblClick}
          onWheel={onWheel}
        />

        {/* Empty state */}
        {nodeCount === 0 && step === "idle" && !fileId && (
          <div style={{
            position: "absolute", inset: 0,
            display: "flex", flexDirection: "column",
            alignItems: "center", justifyContent: "center",
            color: "#475569", fontSize: 15, pointerEvents: "none",
          }}>
            <div style={{ fontSize: 48, marginBottom: 8 }}>🧠</div>
            <div>Upload a research paper to get started</div>
            <div style={{ fontSize: 12, marginTop: 4, color: "#334155" }}>
              Knowledge graph appears here in real-time
            </div>
          </div>
        )}

        {/* File uploaded, ready to process */}
        {nodeCount === 0 && step === "idle" && fileId && (
          <div style={{
            position: "absolute", inset: 0,
            display: "flex", flexDirection: "column",
            alignItems: "center", justifyContent: "center",
            color: "#94a3b8", fontSize: 14, pointerEvents: "none",
          }}>
            <div style={{ fontSize: 36, marginBottom: 8 }}>📄</div>
            <div>Paper uploaded — click <b>▶ Process & Visualize</b></div>
          </div>
        )}

        {/* Live counter during processing */}
        {isProcessing && (
          <div style={{
            position: "absolute", bottom: 10, right: 14,
            background: "rgba(10, 15, 30, 0.85)",
            border: "1px solid rgba(255,255,255,0.08)",
            padding: "5px 12px", borderRadius: 6, fontSize: 11,
            backdropFilter: "blur(4px)",
            color: "#e0e1dd",
          }}>
            ⏱ {nodeCount} nodes · {edgeCount} edges
          </div>
        )}

        {/* Focus indicator */}
        {focusedNode && (
          <div style={{
            position: "absolute", top: 10, left: 14,
            background: "rgba(10, 15, 30, 0.9)",
            border: "1px solid rgba(0, 245, 212, 0.3)",
            padding: "5px 12px", borderRadius: 6, fontSize: 11,
            color: "#00f5d4",
            display: "flex", alignItems: "center", gap: 8,
          }}>
            🎯 Focus: {nodesRef.current.find((n) => n.id === focusedNode)?.label ?? focusedNode}
            <button
              onClick={() => setFocusedNode(null)}
              style={{
                background: "transparent", border: "none", color: "#64748b",
                cursor: "pointer", fontSize: 12, padding: "0 4px",
              }}
            >✕</button>
          </div>
        )}

        {/* Zoom indicator */}
        {nodeCount > 0 && (
          <div style={{
            position: "absolute", bottom: 10, left: 14,
            background: "rgba(10, 15, 30, 0.7)",
            padding: "3px 8px", borderRadius: 4, fontSize: 10,
            color: "#64748b",
          }}>
            {Math.round(camRef.current.zoom * 100)}%
          </div>
        )}
      </div>
    </div>
  );
};
