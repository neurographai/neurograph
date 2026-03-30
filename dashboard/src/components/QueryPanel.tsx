import React, { useCallback } from 'react';
import { useGraphStore } from '../store/graphStore';
import { Search, Loader2, Zap, DollarSign, Clock, Brain } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

// ════════════════════════════════════════════════════════════
// Query Panel — NL query with reasoning trace animation
// ════════════════════════════════════════════════════════════

export const QueryPanel: React.FC = () => {
  const {
    queryInput, queryResults, isQuerying,
    setQueryInput, executeQuery, reasoningStep,
    isAnimatingReasoning, stepReasoning,
  } = useGraphStore();

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (queryInput.trim()) executeQuery(queryInput.trim());
  }, [queryInput, executeQuery]);

  return (
    <div className="ng-panel">
      <form onSubmit={handleSubmit}>
        <div className="ng-search-wrap">
          <Search size={14} className="ng-search-icon" />
          <input
            type="text" value={queryInput}
            onChange={(e) => setQueryInput(e.target.value)}
            placeholder="Ask anything... (e.g., 'Where does Alice work?')"
            className="ng-search-input"
          />
          {isQuerying && <Loader2 size={14} className="ng-spinner" />}
        </div>
      </form>

      <AnimatePresence>
        {queryResults && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="ng-query-result"
          >
            <div className="ng-answer">
              <Brain size={14} className="ng-answer-icon" />
              <p>{queryResults.answer}</p>
            </div>

            {queryResults.reasoning_path.length > 0 && (
              <div className="ng-reasoning">
                <p className="ng-section-label">Reasoning Path</p>
                <div className="ng-reasoning-steps">
                  {queryResults.reasoning_path.map((step, i) => (
                    <React.Fragment key={i}>
                      <span className={`ng-step ${i <= reasoningStep && isAnimatingReasoning ? 'active' : ''}`}>
                        {step.node}
                        <span className="ng-step-conf">{(step.confidence * 100).toFixed(0)}%</span>
                      </span>
                      {i < queryResults.reasoning_path.length - 1 && (
                        <span className="ng-step-arrow">→</span>
                      )}
                    </React.Fragment>
                  ))}
                </div>
                {isAnimatingReasoning && (
                  <button onClick={stepReasoning} className="ng-step-btn">
                    <Zap size={12} /> Step through reasoning
                  </button>
                )}
              </div>
            )}

            {queryResults.cost && (
              <div className="ng-cost-bar">
                <span><Zap size={12} /> {queryResults.cost.model}</span>
                <span><DollarSign size={12} /> ${queryResults.cost.usd.toFixed(4)}</span>
                <span><Clock size={12} /> {queryResults.cost.latency_ms}ms</span>
                <span>{queryResults.cost.tokens} tokens</span>
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
