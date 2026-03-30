// ============================================================
// NeuroGraph — GraphToolbar Component
// Floating toolbar with layout controls and graph actions
// ============================================================

import { useState } from 'react';
import type { LayoutType } from '../types/graph';
import { LAYOUT_CONFIGS } from '../types/graph';

interface GraphToolbarProps {
  currentLayout: LayoutType;
  onLayoutChange: (layout: LayoutType) => void;
  onFitView: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onToggleCommunities: () => void;
  showCommunities: boolean;
}

export default function GraphToolbar({
  currentLayout,
  onLayoutChange,
  onFitView,
  onZoomIn,
  onZoomOut,
  onToggleCommunities,
  showCommunities,
}: GraphToolbarProps) {
  const [showLayoutMenu, setShowLayoutMenu] = useState(false);

  return (
    <div className="ng-toolbar glass-card" id="graph-toolbar">
      {/* Zoom controls */}
      <button
        className="icon-btn"
        onClick={onZoomIn}
        title="Zoom In"
        id="btn-zoom-in"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
          <line x1="11" y1="8" x2="11" y2="14" />
          <line x1="8" y1="11" x2="14" y2="11" />
        </svg>
      </button>

      <button
        className="icon-btn"
        onClick={onZoomOut}
        title="Zoom Out"
        id="btn-zoom-out"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
          <line x1="8" y1="11" x2="14" y2="11" />
        </svg>
      </button>

      <button
        className="icon-btn"
        onClick={onFitView}
        title="Fit to View"
        id="btn-fit-view"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
        </svg>
      </button>

      <div className="toolbar-divider" />

      {/* Layout picker */}
      <div className="layout-picker">
        <button
          className={`icon-btn ${showLayoutMenu ? 'active' : ''}`}
          onClick={() => setShowLayoutMenu(!showLayoutMenu)}
          title="Change Layout"
          id="btn-layout-toggle"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="3" width="7" height="7" />
            <rect x="14" y="3" width="7" height="7" />
            <rect x="14" y="14" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" />
          </svg>
        </button>

        {showLayoutMenu && (
          <div className="layout-menu glass-card animate-fadeInScale" id="layout-menu">
            <div className="layout-menu-title">Graph Layout</div>
            {LAYOUT_CONFIGS.map((config) => (
              <button
                key={config.type}
                className={`layout-option ${currentLayout === config.type ? 'active' : ''}`}
                onClick={() => {
                  onLayoutChange(config.type);
                  setShowLayoutMenu(false);
                }}
                id={`layout-${config.type}`}
              >
                <span className="layout-option-icon">{config.icon}</span>
                <span className="layout-option-label">{config.label}</span>
                {currentLayout === config.type && (
                  <span className="layout-option-check">✓</span>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="toolbar-divider" />

      {/* Community toggle */}
      <button
        className={`icon-btn ${showCommunities ? 'active' : ''}`}
        onClick={onToggleCommunities}
        title={showCommunities ? 'Hide Communities' : 'Show Communities'}
        id="btn-toggle-communities"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="10" />
          <circle cx="12" cy="12" r="6" />
          <circle cx="12" cy="12" r="2" />
        </svg>
      </button>
    </div>
  );
}
