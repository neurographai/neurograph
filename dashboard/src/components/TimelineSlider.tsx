import React, { useCallback, useEffect, useRef } from 'react';
import { useGraphStore } from '../store/graphStore';
import { Play, Pause, SkipBack, SkipForward, Clock } from 'lucide-react';
import { format } from 'date-fns';

// ════════════════════════════════════════════════════════════
// Timeline Slider — scrub through knowledge history
// Shows temporal density & allows animated playback
// ════════════════════════════════════════════════════════════

export const TimelineSlider: React.FC = () => {
  const {
    currentTime, timeRange, isPlaying, playbackSpeed,
    setCurrentTime, togglePlay, setPlaybackSpeed, nodes,
  } = useGraphStore();

  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined);

  useEffect(() => {
    if (isPlaying) {
      intervalRef.current = setInterval(() => {
        setCurrentTime(new Date(
          Math.min(
            useGraphStore.getState().currentTime.getTime() + playbackSpeed * 86400000,
            timeRange.max.getTime()
          )
        ));
      }, 100);
    }
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [isPlaying, playbackSpeed, timeRange, setCurrentTime]);

  const density = React.useMemo(() => {
    const buckets = 80;
    const span = timeRange.max.getTime() - timeRange.min.getTime();
    const sz = span / buckets;
    const counts = new Array(buckets).fill(0);
    nodes.forEach((n) => {
      const t = new Date(n.validFrom).getTime();
      const idx = Math.floor((t - timeRange.min.getTime()) / sz);
      if (idx >= 0 && idx < buckets) counts[idx]++;
    });
    const mx = Math.max(...counts, 1);
    return counts.map((c: number) => c / mx);
  }, [nodes, timeRange]);

  const progress =
    (currentTime.getTime() - timeRange.min.getTime()) /
    (timeRange.max.getTime() - timeRange.min.getTime());

  const handleSlider = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const pct = parseFloat(e.target.value) / 100;
      setCurrentTime(new Date(
        timeRange.min.getTime() + pct * (timeRange.max.getTime() - timeRange.min.getTime())
      ));
    },
    [timeRange, setCurrentTime]
  );

  return (
    <div className="ng-timeline">
      <div className="ng-timeline-header">
        <div className="ng-timeline-label"><Clock size={14} /> Temporal Navigator</div>
        <span className="ng-timeline-date">{format(currentTime, 'MMM d, yyyy HH:mm')}</span>
      </div>

      <div className="ng-density-bar">
        {density.map((d: number, i: number) => (
          <div
            key={i}
            className="ng-density-col"
            style={{
              height: `${Math.max(3, d * 100)}%`,
              backgroundColor: i / density.length <= progress
                ? `rgba(99, 102, 241, ${0.3 + d * 0.7})`
                : `rgba(100, 116, 139, ${0.15 + d * 0.25})`,
            }}
          />
        ))}
      </div>

      <input
        type="range" min={0} max={100} step={0.1}
        value={progress * 100} onChange={handleSlider}
        className="ng-slider"
      />

      <div className="ng-timeline-controls">
        <div className="ng-playback">
          <button onClick={() => setCurrentTime(timeRange.min)} className="ng-btn-icon">
            <SkipBack size={14} />
          </button>
          <button onClick={togglePlay} className="ng-btn-play">
            {isPlaying ? <Pause size={14} /> : <Play size={14} />}
          </button>
          <button onClick={() => setCurrentTime(timeRange.max)} className="ng-btn-icon">
            <SkipForward size={14} />
          </button>
        </div>
        <div className="ng-speed-btns">
          {[0.5, 1, 2, 5, 10].map((speed) => (
            <button
              key={speed}
              onClick={() => setPlaybackSpeed(speed)}
              className={`ng-speed-btn ${playbackSpeed === speed ? 'active' : ''}`}
            >
              {speed}×
            </button>
          ))}
        </div>
        <div className="ng-time-range">
          {format(timeRange.min, 'MMM yyyy')} — {format(timeRange.max, 'MMM yyyy')}
        </div>
      </div>
    </div>
  );
};
