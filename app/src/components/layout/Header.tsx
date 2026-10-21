import React, { useState } from 'react';
import {
  PanelLeftClose,
  PanelLeft,
  Settings2,
  GitBranch,
  Pencil,
  Check,
  X,
  ChevronDown,
} from 'lucide-react';
import { OllamaHealthBadge } from './OllamaHealthBadge';
import type { HealthStatus, ModelInfo, Session } from '../../api/types';

interface HeaderProps {
  currentSession: Session | null;
  models: ModelInfo[];
  selectedModel: string;
  onSelectModel: (modelName: string) => void;
  onRenameSession: (sessionId: string, newTitle: string) => void;
  onToggleSidebar: () => void;
  isSidebarCollapsed: boolean;
  onOpenSystemInstructions: () => void;
  hasSystemInstructions: boolean;
  health: HealthStatus | null;
  isCheckingHealth: boolean;
  onRefreshHealth: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentSession,
  models,
  selectedModel,
  onSelectModel,
  onRenameSession,
  onToggleSidebar,
  isSidebarCollapsed,
  onOpenSystemInstructions,
  hasSystemInstructions,
  health,
  isCheckingHealth,
  onRefreshHealth,
}) => {
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [titleInput, setTitleInput] = useState('');
  const [isModelDropdownOpen, setIsModelDropdownOpen] = useState(false);

  const isBranch = currentSession?.options_json?.includes('"is_branch":true');

  const handleStartRename = () => {
    if (!currentSession) return;
    setTitleInput(currentSession.title || 'Untitled Chat');
    setIsEditingTitle(true);
  };

  const handleSaveRename = () => {
    if (currentSession && titleInput.trim()) {
      onRenameSession(currentSession.id, titleInput.trim());
    }
    setIsEditingTitle(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSaveRename();
    } else if (e.key === 'Escape') {
      setIsEditingTitle(false);
    }
  };

  return (
    <header
      style={{
        height: '56px',
        borderBottom: '1px solid var(--border-subtle)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 1rem',
        backgroundColor: 'var(--bg-app)',
        zIndex: 10,
        gap: '1rem',
      }}
    >
      {/* Left side: Sidebar Toggle & Session Info */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', minWidth: 0 }}>
        <button
          type="button"
          onClick={onToggleSidebar}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '32px',
            height: '32px',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-secondary)',
            backgroundColor: 'var(--bg-surface)',
            border: '1px solid var(--border-subtle)',
            transition: 'all var(--transition-fast)',
          }}
          title={isSidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-label={isSidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {isSidebarCollapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
        </button>

        {/* Title / Renaming */}
        {currentSession ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', minWidth: 0 }}>
            {isBranch && (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.25rem',
                  padding: '0.15rem 0.45rem',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--branch-bg)',
                  border: '1px solid var(--branch-border)',
                  color: 'var(--branch-accent)',
                  fontSize: '0.72rem',
                  fontWeight: 600,
                  whiteSpace: 'nowrap',
                }}
              >
                <GitBranch size={11} />
                Branch
              </span>
            )}

            {isEditingTitle ? (
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                <input
                  type="text"
                  value={titleInput}
                  onChange={(e) => setTitleInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  autoFocus
                  style={{
                    backgroundColor: 'var(--bg-surface-elevated)',
                    border: '1px solid var(--accent-primary)',
                    borderRadius: 'var(--radius-sm)',
                    padding: '0.2rem 0.5rem',
                    color: 'var(--text-primary)',
                    fontSize: '0.88rem',
                    fontWeight: 500,
                  }}
                />
                <button
                  type="button"
                  onClick={handleSaveRename}
                  style={{ color: 'var(--status-healthy)', padding: '2px' }}
                >
                  <Check size={14} />
                </button>
                <button
                  type="button"
                  onClick={() => setIsEditingTitle(false)}
                  style={{ color: 'var(--text-muted)', padding: '2px' }}
                >
                  <X size={14} />
                </button>
              </div>
            ) : (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.35rem',
                  cursor: 'pointer',
                  minWidth: 0,
                }}
                onClick={handleStartRename}
                title="Click to rename session"
              >
                <span
                  style={{
                    fontWeight: 600,
                    fontSize: '0.92rem',
                    color: 'var(--text-primary)',
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    maxWidth: '300px',
                  }}
                >
                  {currentSession.title || 'Untitled Chat'}
                </span>
                <Pencil size={12} color="var(--text-dim)" />
              </div>
            )}
          </div>
        ) : (
          <span style={{ fontWeight: 600, fontSize: '0.92rem', color: 'var(--text-muted)' }}>
            New Chat
          </span>
        )}
      </div>

      {/* Right side: Model Selector, System Instructions, Ollama Health */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
        {/* Model Selector Dropdown */}
        <div style={{ position: 'relative' }}>
          <button
            type="button"
            onClick={() => setIsModelDropdownOpen((prev) => !prev)}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '0.4rem',
              padding: '0.32rem 0.65rem',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              fontSize: '0.8rem',
              fontWeight: 500,
              color: 'var(--text-primary)',
              transition: 'border-color var(--transition-fast)',
            }}
          >
            <span>{selectedModel || 'Select Model'}</span>
            <ChevronDown size={13} color="var(--text-muted)" />
          </button>

          {isModelDropdownOpen && (
            <>
              <div
                style={{ position: 'fixed', inset: 0, zIndex: 40 }}
                onClick={() => setIsModelDropdownOpen(false)}
              />
              <div
                style={{
                  position: 'absolute',
                  top: 'calc(100% + 4px)',
                  right: 0,
                  width: '220px',
                  maxHeight: '260px',
                  overflowY: 'auto',
                  backgroundColor: 'var(--bg-surface-elevated)',
                  border: '1px solid var(--border-medium)',
                  borderRadius: 'var(--radius-md)',
                  boxShadow: 'var(--shadow-lg)',
                  zIndex: 50,
                  padding: '0.35rem',
                }}
              >
                <div
                  style={{
                    fontSize: '0.7rem',
                    color: 'var(--text-dim)',
                    padding: '0.3rem 0.5rem',
                    fontWeight: 600,
                    textTransform: 'uppercase',
                  }}
                >
                  Installed Models
                </div>
                {models.length === 0 ? (
                  <div style={{ padding: '0.5rem', fontSize: '0.78rem', color: 'var(--text-muted)' }}>
                    No models found. Pull one via <code>ollama run llama3.2</code>
                  </div>
                ) : (
                  models.map((m) => (
                    <button
                      key={m.name}
                      type="button"
                      onClick={() => {
                        onSelectModel(m.name);
                        setIsModelDropdownOpen(false);
                      }}
                      style={{
                        width: '100%',
                        textAlign: 'left',
                        padding: '0.4rem 0.5rem',
                        borderRadius: 'var(--radius-sm)',
                        fontSize: '0.8rem',
                        color: m.name === selectedModel ? 'var(--accent-primary)' : 'var(--text-primary)',
                        backgroundColor:
                          m.name === selectedModel ? 'var(--accent-subtle)' : 'transparent',
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center',
                      }}
                    >
                      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{m.name}</span>
                      {m.name === selectedModel && <Check size={12} color="var(--accent-primary)" />}
                    </button>
                  ))
                )}
              </div>
            </>
          )}
        </div>

        {/* System Instructions Button */}
        <button
          type="button"
          onClick={onOpenSystemInstructions}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.4rem',
            padding: '0.32rem 0.65rem',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: hasSystemInstructions ? 'var(--accent-subtle)' : 'var(--bg-surface)',
            border: `1px solid ${hasSystemInstructions ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
            color: hasSystemInstructions ? 'var(--accent-primary)' : 'var(--text-secondary)',
            fontSize: '0.8rem',
            fontWeight: 500,
            transition: 'all var(--transition-fast)',
          }}
          title="Edit per-chat system prompt instructions"
        >
          <Settings2 size={13} />
          <span>Instructions</span>
          {hasSystemInstructions && (
            <span
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: 'var(--accent-primary)',
              }}
            />
          )}
        </button>

        {/* Ollama Health Status Badge */}
        <OllamaHealthBadge
          health={health}
          isChecking={isCheckingHealth}
          onRefresh={onRefreshHealth}
        />
      </div>
    </header>
  );
};
