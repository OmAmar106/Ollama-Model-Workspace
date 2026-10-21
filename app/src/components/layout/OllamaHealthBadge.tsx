import React, { useState } from 'react';
import { RefreshCw, AlertCircle, CheckCircle2 } from 'lucide-react';
import type { HealthStatus } from '../../api/types';

interface OllamaHealthBadgeProps {
  health: HealthStatus | null;
  isChecking: boolean;
  onRefresh: () => void;
}

export const OllamaHealthBadge: React.FC<OllamaHealthBadgeProps> = ({
  health,
  isChecking,
  onRefresh,
}) => {
  const [showTooltip, setShowTooltip] = useState(false);

  const isHealthy = health?.is_healthy ?? false;
  const statusColor = isChecking
    ? 'var(--status-warning)'
    : isHealthy
    ? 'var(--status-healthy)'
    : 'var(--status-error)';

  const statusText = isChecking
    ? 'Checking...'
    : isHealthy
    ? health?.version
      ? `Ollama v${health.version}`
      : 'Ollama Active'
    : 'Ollama Offline';

  return (
    <div
      style={{ position: 'relative', display: 'inline-flex', alignItems: 'center' }}
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
    >
      <div
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '0.45rem',
          padding: '0.35rem 0.65rem',
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-pill)',
          fontSize: '0.78rem',
          color: 'var(--text-secondary)',
          cursor: 'pointer',
          transition: 'all var(--transition-fast)',
          boxShadow: 'var(--shadow-sm)',
        }}
        onClick={onRefresh}
        title="Click to refresh Ollama health"
      >
        {/* Pulsing Status Dot */}
        <span
          style={{
            position: 'relative',
            display: 'inline-block',
            width: '8px',
            height: '8px',
            borderRadius: '50%',
            backgroundColor: statusColor,
            boxShadow: `0 0 8px ${statusColor}`,
          }}
        />

        <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>
          {statusText}
        </span>

        {/* Manual Refresh Button */}
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onRefresh();
          }}
          disabled={isChecking}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '2px',
            marginLeft: '2px',
            color: 'var(--text-muted)',
            transition: 'color var(--transition-fast)',
          }}
          aria-label="Refresh Ollama connection status"
        >
          <RefreshCw
            size={12}
            style={{
              animation: isChecking ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {/* Tooltip / Details Popover on Hover */}
      {showTooltip && (
        <div
          style={{
            position: 'absolute',
            top: 'calc(100% + 8px)',
            right: 0,
            width: '260px',
            backgroundColor: 'var(--bg-surface-elevated)',
            border: '1px solid var(--border-medium)',
            borderRadius: 'var(--radius-md)',
            padding: '0.75rem',
            boxShadow: 'var(--shadow-lg)',
            zIndex: 100,
            fontSize: '0.78rem',
            lineHeight: 1.45,
            pointerEvents: 'none',
            animation: 'fadeIn 0.15s ease-out',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '0.4rem',
              fontWeight: 600,
              color: isHealthy ? 'var(--status-healthy)' : 'var(--status-error)',
              marginBottom: '0.4rem',
            }}
          >
            {isHealthy ? <CheckCircle2 size={14} /> : <AlertCircle size={14} />}
            <span>{isHealthy ? 'Local Ollama Connected' : 'Connection Failed'}</span>
          </div>

          <div style={{ color: 'var(--text-secondary)', marginBottom: '0.35rem' }}>
            <span style={{ color: 'var(--text-muted)' }}>Endpoint: </span>
            <code
              style={{
                fontFamily: 'var(--font-mono)',
                backgroundColor: 'rgba(255, 255, 255, 0.05)',
                padding: '0.1rem 0.3rem',
                borderRadius: '3px',
              }}
            >
              {health?.base_url || 'http://127.0.0.1:11434'}
            </code>
          </div>

          {health?.error && (
            <div
              style={{
                color: 'var(--status-error)',
                backgroundColor: 'rgba(239, 68, 68, 0.1)',
                border: '1px solid rgba(239, 68, 68, 0.2)',
                borderRadius: 'var(--radius-sm)',
                padding: '0.4rem',
                marginTop: '0.35rem',
                fontSize: '0.72rem',
                wordBreak: 'break-word',
              }}
            >
              {health.error}
            </div>
          )}

          <div
            style={{
              marginTop: '0.5rem',
              fontSize: '0.7rem',
              color: 'var(--text-dim)',
              borderTop: '1px solid var(--border-subtle)',
              paddingTop: '0.4rem',
            }}
          >
            Auto-checks every 30s • Click to test now
          </div>
        </div>
      )}

      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @keyframes fadeIn {
          from { opacity: 0; transform: translateY(-4px); }
          to { opacity: 1; transform: translateY(0); }
        }
      `}</style>
    </div>
  );
};
