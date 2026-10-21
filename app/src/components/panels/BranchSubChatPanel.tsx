import React, { useState, useRef, useEffect } from 'react';
import {
  GitBranch,
  X,
  Maximize2,
  Minimize2,
  Send,
  CornerDownRight,
  Bot,
  User,
} from 'lucide-react';
import { MarkdownRenderer } from '../chat/MarkdownRenderer';
import type { ActiveBranchState } from '../../hooks/useChat';

interface BranchSubChatPanelProps {
  branchState: ActiveBranchState;
  onSendMessage: (prompt: string) => void;
  onClose: () => void;
}

export const BranchSubChatPanel: React.FC<BranchSubChatPanelProps> = ({
  branchState,
  onSendMessage,
  onClose,
}) => {
  const [isExpanded, setIsExpanded] = useState<boolean>(false);
  const [input, setInput] = useState<string>('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll inside branch messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [branchState.messages, branchState.streamContent]);

  const handleSend = () => {
    if (!input.trim() || branchState.isGenerating) return;
    onSendMessage(input.trim());
    setInput('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleSend();
    }
  };

  // Messages in this branch that were created AFTER the branch point
  const newBranchMessages = branchState.messages.filter(
    (m, idx) => idx >= branchState.messages.findIndex((x) => x.id === branchState.branchedFromMessageId) + 1
  );

  return (
    <div
      style={{
        margin: '0.85rem 1rem',
        borderRadius: 'var(--radius-lg)',
        border: '1px solid var(--branch-border)',
        backgroundColor: 'rgba(9, 14, 22, 0.95)',
        backdropFilter: 'blur(16px)',
        boxShadow: '0 12px 36px rgba(0, 0, 0, 0.5), 0 0 20px rgba(6, 182, 212, 0.15)',
        overflow: 'hidden',
        transition: 'all var(--transition-normal)',
        maxHeight: isExpanded ? '600px' : '380px',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 15,
        position: 'relative',
      }}
    >
      {/* Branch Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '0.6rem 0.85rem',
          backgroundColor: 'rgba(6, 182, 212, 0.08)',
          borderBottom: '1px solid var(--branch-border)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '22px',
              height: '22px',
              borderRadius: '4px',
              backgroundColor: 'rgba(6, 182, 212, 0.2)',
              color: 'var(--branch-accent)',
            }}
          >
            <GitBranch size={13} />
          </div>
          <div>
            <div style={{ fontSize: '0.82rem', fontWeight: 600, color: 'var(--branch-accent)' }}>
              Branch Exploration Path
            </div>
            <div style={{ fontSize: '0.68rem', color: 'var(--text-dim)' }}>
              Anchored alternative path • Original context preserved
            </div>
          </div>
        </div>

        {/* Header Controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
          <button
            type="button"
            onClick={() => setIsExpanded((prev) => !prev)}
            style={{
              padding: '4px',
              color: 'var(--text-secondary)',
              borderRadius: 'var(--radius-sm)',
            }}
            title={isExpanded ? 'Collapse panel' : 'Expand panel'}
          >
            {isExpanded ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
          </button>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '4px',
              color: 'var(--text-secondary)',
              borderRadius: 'var(--radius-sm)',
            }}
            title="Close branch panel"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* Branch Messages Content */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '0.85rem',
          display: 'flex',
          flexDirection: 'column',
          gap: '0.75rem',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '0.4rem',
            padding: '0.4rem 0.65rem',
            backgroundColor: 'rgba(6, 182, 212, 0.05)',
            border: '1px dashed rgba(6, 182, 212, 0.25)',
            borderRadius: 'var(--radius-sm)',
            fontSize: '0.74rem',
            color: 'var(--branch-accent)',
          }}
        >
          <CornerDownRight size={13} />
          <span>Branch created from selected message. Continue the alternative conversation below:</span>
        </div>

        {newBranchMessages.length === 0 && !branchState.isGenerating && (
          <div
            style={{
              padding: '1.25rem',
              textAlign: 'center',
              color: 'var(--text-muted)',
              fontSize: '0.8rem',
            }}
          >
            Type a prompt below to explore an alternative response or direction from this point.
          </div>
        )}

        {newBranchMessages.map((msg) => (
          <div
            key={msg.id}
            style={{
              display: 'flex',
              gap: '0.65rem',
              padding: '0.65rem',
              borderRadius: 'var(--radius-md)',
              backgroundColor: msg.role === 'user' ? 'rgba(255, 255, 255, 0.04)' : 'rgba(6, 182, 212, 0.03)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            <div style={{ flexShrink: 0, marginTop: '2px' }}>
              {msg.role === 'user' ? (
                <User size={15} color="var(--text-secondary)" />
              ) : (
                <Bot size={15} color="var(--branch-accent)" />
              )}
            </div>
            <div style={{ flex: 1, minWidth: 0, fontSize: '0.86rem' }}>
              <div style={{ fontWeight: 600, fontSize: '0.76rem', color: 'var(--text-dim)', marginBottom: '2px' }}>
                {msg.role === 'user' ? 'You (Branch)' : 'Assistant (Branch)'}
              </div>
              <MarkdownRenderer content={msg.content} />
            </div>
          </div>
        ))}

        {/* Streaming In-Flight Message */}
        {branchState.isGenerating && (
          <div
            style={{
              display: 'flex',
              gap: '0.65rem',
              padding: '0.65rem',
              borderRadius: 'var(--radius-md)',
              backgroundColor: 'rgba(6, 182, 212, 0.04)',
              border: '1px solid rgba(6, 182, 212, 0.2)',
            }}
          >
            <Bot size={15} color="var(--branch-accent)" style={{ flexShrink: 0, marginTop: '2px' }} />
            <div style={{ flex: 1, minWidth: 0, fontSize: '0.86rem' }}>
              <div style={{ fontWeight: 600, fontSize: '0.76rem', color: 'var(--branch-accent)', marginBottom: '2px' }}>
                Assistant Generating in Branch...
              </div>
              {branchState.streamThinking && (
                <div
                  style={{
                    fontSize: '0.78rem',
                    color: 'var(--reasoning-text)',
                    fontStyle: 'italic',
                    marginBottom: '0.4rem',
                  }}
                >
                  {branchState.streamThinking}
                </div>
              )}
              <MarkdownRenderer content={branchState.streamContent} />
              <span className="streaming-cursor" />
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Branch Mini-Composer */}
      <div
        style={{
          padding: '0.6rem 0.85rem',
          borderTop: '1px solid var(--branch-border)',
          backgroundColor: 'rgba(6, 182, 212, 0.04)',
          display: 'flex',
          gap: '0.5rem',
        }}
      >
        <input
          type="text"
          placeholder="Continue this branch..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={branchState.isGenerating}
          style={{
            flex: 1,
            padding: '0.45rem 0.75rem',
            backgroundColor: 'var(--bg-surface)',
            border: '1px solid var(--border-medium)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-primary)',
            fontSize: '0.82rem',
          }}
        />
        <button
          type="button"
          onClick={handleSend}
          disabled={!input.trim() || branchState.isGenerating}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.35rem',
            padding: '0.45rem 0.85rem',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: 'var(--branch-accent)',
            color: '#082f49',
            fontWeight: 600,
            fontSize: '0.8rem',
            opacity: !input.trim() || branchState.isGenerating ? 0.4 : 1,
          }}
        >
          <Send size={13} />
          <span>Send</span>
        </button>
      </div>
    </div>
  );
};
