import { Sun, Moon } from 'lucide-react';
import { useGraphStore } from '../store/graphStore';

// ════════════════════════════════════════════════════════════
// Theme Toggle — Animated Sun / Moon button
// ════════════════════════════════════════════════════════════

export function ThemeToggle() {
  const theme = useGraphStore((s) => s.theme);
  const toggleTheme = useGraphStore((s) => s.toggleTheme);

  return (
    <button
      id="theme-toggle"
      className="ng-theme-toggle"
      onClick={toggleTheme}
      aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
      title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
    >
      <span className={`ng-theme-icon ${theme === 'dark' ? 'active' : ''}`}>
        <Moon size={14} />
      </span>
      <span className={`ng-theme-icon ${theme === 'light' ? 'active' : ''}`}>
        <Sun size={14} />
      </span>
      <span
        className="ng-theme-indicator"
        style={{ transform: theme === 'light' ? 'translateX(24px)' : 'translateX(0)' }}
      />
    </button>
  );
}
