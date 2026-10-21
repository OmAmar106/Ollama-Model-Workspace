import React, { useEffect, useState } from 'react';
import { X, Sparkles, Check, Trash2, ShieldAlert } from 'lucide-react';

interface SystemInstructionsDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  savedInstructions: string | null;
  onSave: (instructions: string | null) => Promise<void>;
  sessionTitle: string;
}

export const SystemInstructionsDrawer: React.FC<SystemInstructionsDrawerProps> = ({
  isOpen,
  onClose,
  savedInstructions,
  onSave,
  sessionTitle,
}) => {
  const [instructions, setInstructions] = useState<string>('');
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [saveSuccess, setSaveSuccess] = useState<boolean>(false);
  const [showClearConfirm, setShowClearConfirm] = useState<boolean>(false);

  useEffect(() => {
    setInstructions(savedInstructions || '');
    setSaveSuccess(false);
  }, [savedInstructions, isOpen]);

  if (!isOpen) return null;

  const hasUnsavedChanges = (instructions.trim() || '') !== (savedInstructions?.trim() || '');

  const handleSave = async () => {
    setIsSaving(true);
    setSaveSuccess(false);
    try {
      const trimmed = instructions.trim();
      await onSave(trimmed ? trimmed : null);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2500);
    } catch (err) {
      console.error('Failed to save system instructions:', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleClear = async () => {
    setInstructions('');
    setShowClearConfirm(false);
    setIsSaving(true);
    try {
      await onSave(null);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2500);
    } catch (err) {
      console.error('Failed to clear instructions:', err);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.5)',
        backdropFilter: 'blur(3px)',
        zIndex: 50,
        display: 'flex',
        justifyContent: 'flex-end',
        animation: 'fadeIn 0.2s ease-out',
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '420px',
          maxWidth: '100%',
          height: '100%',
          backgroundColor: 'var(--bg-surface-elevated)',
          borderLeft: '1px solid var(--border-medium)',
          boxShadow: 'var(--shadow-xl)',
          display: 'flex',
          flexDirection: 'column',
          animation: 'slideInRight 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Drawer Header */}
        <div
          style={{
            padding: '1.25rem',
            borderBottom: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Sparkles size={18} color="var(--accent-primary)" />
            <div>
              <div style={{ fontWeight: 600, fontSize: '0.96rem' }}>System Instructions</div>
              <div style={{ fontSize: '0.74rem', color: 'var(--text-muted)' }}>
                Scoped to: {sessionTitle || 'Current Session'}
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '6px',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--text-secondary)',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Drawer Body */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '1.25rem' }}>
          <p
            style={{
              fontSize: '0.82rem',
              color: 'var(--text-secondary)',
              lineHeight: 1.5,
              marginBottom: '1rem',
            }}
          >
            Provide persistent directives, output constraints, persona details, or guidelines. These
            instructions will be automatically prepended before conversation history on every turn.
          </p>

          <div style={{ position: 'relative', marginBottom: '1rem' }}>
            <textarea
              rows={12}
              placeholder="e.g. You are a senior systems engineer. Be concise, direct, and provide typed, production-ready Rust code snippets..."
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              style={{
                width: '100%',
                padding: '0.75rem',
                backgroundColor: 'var(--bg-surface)',
                border: `1px solid ${hasUnsavedChanges ? 'var(--accent-primary)' : 'var(--border-medium)'}`,
                borderRadius: 'var(--radius-md)',
                color: 'var(--text-primary)',
                fontSize: '0.86rem',
                lineHeight: 1.55,
                resize: 'vertical',
                fontFamily: 'inherit',
              }}
            />

            {hasUnsavedChanges && (
              <span
                style={{
                  position: 'absolute',
                  top: '8px',
                  right: '10px',
                  fontSize: '0.68rem',
                  fontWeight: 600,
                  color: 'var(--status-warning)',
                  backgroundColor: 'rgba(245, 158, 11, 0.15)',
                  padding: '2px 6px',
                  borderRadius: 'var(--radius-sm)',
                }}
              >
                Unsaved changes
              </span>
            )}
          </div>

          {/* Quick presets */}
          <div style={{ marginBottom: '1.5rem' }}>
            <div style={{ fontSize: '0.74rem', fontWeight: 600, color: 'var(--text-dim)', marginBottom: '0.4rem' }}>
              Suggested Styles:
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.4rem' }}>
              {[
                { label: 'Technical & Concise', text: 'You are an expert software engineer. Prioritize concise, idiomatic code examples without filler text.' },
                { label: 'Step-by-Step Tutor', text: 'You are an educational tutor. Explain complex concepts intuitively with practical analogies and step-by-step breakdowns.' },
                { label: 'Code Reviewer', text: 'You are a meticulous code reviewer. Identify bugs, performance bottlenecks, and security considerations.' },
              ].map((preset) => (
                <button
                  key={preset.label}
                  type="button"
                  onClick={() => setInstructions(preset.text)}
                  style={{
                    fontSize: '0.74rem',
                    padding: '0.25rem 0.55rem',
                    borderRadius: 'var(--radius-sm)',
                    backgroundColor: 'var(--bg-surface)',
                    border: '1px solid var(--border-subtle)',
                    color: 'var(--text-secondary)',
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.borderColor = 'var(--accent-primary)')}
                  onMouseLeave={(e) => (e.currentTarget.style.borderColor = 'var(--border-subtle)')}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Drawer Footer */}
        <div
          style={{
            padding: '1rem 1.25rem',
            borderTop: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            backgroundColor: 'var(--bg-surface)',
          }}
        >
          {showClearConfirm ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
              <button
                type="button"
                onClick={handleClear}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.3rem',
                  padding: '0.4rem 0.75rem',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--status-error)',
                  color: '#ffffff',
                  fontSize: '0.78rem',
                  fontWeight: 600,
                }}
              >
                <ShieldAlert size={12} />
                Confirm Clear
              </button>
              <button
                type="button"
                onClick={() => setShowClearConfirm(false)}
                style={{
                  padding: '0.4rem 0.6rem',
                  fontSize: '0.78rem',
                  color: 'var(--text-muted)',
                }}
              >
                Cancel
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setShowClearConfirm(true)}
              disabled={!instructions.trim() && !savedInstructions}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '0.3rem',
                fontSize: '0.78rem',
                color: 'var(--text-muted)',
                opacity: !instructions.trim() && !savedInstructions ? 0.3 : 1,
              }}
            >
              <Trash2 size={13} />
              <span>Clear</span>
            </button>
          )}

          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            {saveSuccess && (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.25rem',
                  fontSize: '0.76rem',
                  color: 'var(--status-healthy)',
                  fontWeight: 600,
                }}
              >
                <Check size={14} />
                Saved
              </span>
            )}

            <button
              type="button"
              onClick={handleSave}
              disabled={isSaving || !hasUnsavedChanges}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '0.4rem',
                padding: '0.45rem 1rem',
                borderRadius: 'var(--radius-sm)',
                backgroundColor: 'var(--accent-primary)',
                color: '#ffffff',
                fontWeight: 600,
                fontSize: '0.84rem',
                boxShadow: 'var(--shadow-sm)',
                opacity: isSaving || !hasUnsavedChanges ? 0.5 : 1,
              }}
            >
              {isSaving ? 'Saving...' : 'Save Instructions'}
            </button>
          </div>
        </div>
      </div>

      <style>{`
        @keyframes slideInRight {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
      `}</style>
    </div>
  );
};
