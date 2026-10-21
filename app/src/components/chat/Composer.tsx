import React, { useRef, useEffect, useState } from 'react';
import {
  ArrowUp,
  Square,
  BrainCircuit,
  ChevronDown,
  Check,
} from 'lucide-react';
import type { ModelInfo } from '../../api/types';

interface ComposerProps {
  onSend: (prompt: string) => void;
  onStop: () => void;
  isGenerating: boolean;
  models: ModelInfo[];
  selectedModel: string;
  onSelectModel: (modelName: string) => void;
  isThinkEnabled: boolean;
  onToggleThink: () => void;
  disabled?: boolean;
}

export const Composer: React.FC<ComposerProps> = ({
  onSend,
  onStop,
  isGenerating,
  models,
  selectedModel,
  onSelectModel,
  isThinkEnabled,
  onToggleThink,
  disabled = false,
}) => {
  const [prompt, setPrompt] = useState('');
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea height as content changes
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      const scrollHeight = textareaRef.current.scrollHeight;
      textareaRef.current.style.height = `${Math.min(scrollHeight, 220)}px`;
    }
  }, [prompt]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleSend = () => {
    if (!prompt.trim() || isGenerating || disabled) return;
    onSend(prompt);
    setPrompt('');
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  };

  return (
    <div
      style={{
        padding: '0.75rem 1.25rem 1.25rem',
        backgroundColor: 'var(--bg-app)',
        borderTop: '1px solid var(--border-subtle)',
      }}
    >
      <div
        style={{
          maxWidth: '820px',
          margin: '0 auto',
          position: 'relative',
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-medium)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-md)',
          transition: 'border-color var(--transition-fast), box-shadow var(--transition-fast)',
        }}
        onFocus={(e) => (e.currentTarget.style.borderColor = 'var(--accent-primary)')}
        onBlur={(e) => (e.currentTarget.style.borderColor = 'var(--border-medium)')}
      >
        {/* Text Area */}
        <textarea
          ref={textareaRef}
          rows={1}
          placeholder="Ask anything... (Shift+Enter for new line)"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={disabled}
          style={{
            width: '100%',
            padding: '0.85rem 1rem 0.4rem',
            backgroundColor: 'transparent',
            border: 'none',
            color: 'var(--text-primary)',
            fontSize: '0.94rem',
            lineHeight: 1.55,
            resize: 'none',
            maxHeight: '220px',
            fontFamily: 'inherit',
          }}
        />

        {/* Action Controls Bar below input */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '0.45rem 0.75rem 0.65rem',
          }}
        >
          {/* Left: Model selector & Think toggle */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            {/* Quick Model Selector Pill */}
            <div style={{ position: 'relative' }}>
              <button
                type="button"
                onClick={() => setIsModelMenuOpen((prev) => !prev)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.35rem',
                  padding: '0.25rem 0.55rem',
                  borderRadius: 'var(--radius-pill)',
                  backgroundColor: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-subtle)',
                  fontSize: '0.74rem',
                  fontWeight: 500,
                  color: 'var(--text-secondary)',
                }}
              >
                <span>{selectedModel}</span>
                <ChevronDown size={11} />
              </button>

              {isModelMenuOpen && (
                <>
                  <div
                    style={{ position: 'fixed', inset: 0, zIndex: 90 }}
                    onClick={() => setIsModelMenuOpen(false)}
                  />
                  <div
                    style={{
                      position: 'absolute',
                      bottom: 'calc(100% + 6px)',
                      left: 0,
                      width: '220px',
                      maxHeight: '220px',
                      overflowY: 'auto',
                      backgroundColor: 'var(--bg-surface-elevated)',
                      border: '1px solid var(--border-medium)',
                      borderRadius: 'var(--radius-md)',
                      boxShadow: 'var(--shadow-xl)',
                      padding: '0.35rem',
                      zIndex: 100,
                    }}
                  >
                    {models.map((m) => (
                      <button
                        key={m.name}
                        type="button"
                        onClick={() => {
                          onSelectModel(m.name);
                          setIsModelMenuOpen(false);
                        }}
                        style={{
                          width: '100%',
                          textAlign: 'left',
                          padding: '0.35rem 0.5rem',
                          borderRadius: 'var(--radius-sm)',
                          fontSize: '0.78rem',
                          color: m.name === selectedModel ? 'var(--accent-primary)' : 'var(--text-primary)',
                          backgroundColor: m.name === selectedModel ? 'var(--accent-subtle)' : 'transparent',
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                        }}
                      >
                        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{m.name}</span>
                        {m.name === selectedModel && <Check size={12} color="var(--accent-primary)" />}
                      </button>
                    ))}
                  </div>
                </>
              )}
            </div>

            {/* Reasoning / Think Toggle */}
            <button
              type="button"
              onClick={onToggleThink}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '0.3rem',
                padding: '0.25rem 0.55rem',
                borderRadius: 'var(--radius-pill)',
                backgroundColor: isThinkEnabled ? 'rgba(139, 92, 246, 0.18)' : 'rgba(255, 255, 255, 0.04)',
                border: `1px solid ${isThinkEnabled ? 'rgba(139, 92, 246, 0.45)' : 'var(--border-subtle)'}`,
                color: isThinkEnabled ? '#c4b5fd' : 'var(--text-muted)',
                fontSize: '0.74rem',
                fontWeight: 500,
                transition: 'all var(--transition-fast)',
              }}
              title="Toggle reasoning / thinking mode"
            >
              <BrainCircuit size={13} color={isThinkEnabled ? '#c4b5fd' : 'var(--text-muted)'} />
              <span>Think</span>
            </button>
          </div>

          {/* Right: Send or Stop Button */}
          <div>
            {isGenerating ? (
              <button
                type="button"
                onClick={onStop}
                style={{
                  width: '32px',
                  height: '32px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'rgba(239, 68, 68, 0.18)',
                  border: '1px solid rgba(239, 68, 68, 0.35)',
                  color: 'var(--status-error)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: 'var(--shadow-sm)',
                }}
                title="Stop generation (Esc)"
              >
                <Square size={13} fill="currentColor" />
              </button>
            ) : (
              <button
                type="button"
                onClick={handleSend}
                disabled={!prompt.trim() || disabled}
                style={{
                  width: '32px',
                  height: '32px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: prompt.trim() ? 'var(--accent-primary)' : 'rgba(255, 255, 255, 0.08)',
                  color: prompt.trim() ? '#ffffff' : 'var(--text-dim)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  transition: 'all var(--transition-fast)',
                  boxShadow: prompt.trim() ? 'var(--shadow-sm)' : 'none',
                }}
                title="Send message (Enter)"
              >
                <ArrowUp size={16} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
