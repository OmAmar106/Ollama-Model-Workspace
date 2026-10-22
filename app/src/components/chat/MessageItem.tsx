import React, { useState } from 'react';
import {
  User,
  Bot,
  Copy,
  Check,
  GitBranch,
  ChevronDown,
  ChevronRight,
  BrainCircuit,
  RotateCcw,
  AlertTriangle,
  StopCircle,
} from 'lucide-react';
import { MarkdownRenderer } from './MarkdownRenderer';
import type { Message } from '../../api/types';

interface MessageItemProps {
  message: Message;
  isStreaming?: boolean;
  streamingThinking?: string;
  streamingContent?: string;
  onBranchFromHere?: (messageId: string) => void;
  onRetry?: (content: string) => void;
}

export const MessageItem: React.FC<MessageItemProps> = ({
  message,
  isStreaming = false,
  streamingThinking = '',
  streamingContent = '',
  onBranchFromHere,
  onRetry,
}) => {
  const [copied, setCopied] = useState(false);
  const [isThinkingExpanded, setIsThinkingExpanded] = useState(true);

  const isUser = message.role === 'user';
  const effectiveContent = isStreaming ? streamingContent : message.content;
  const effectiveThinking = isStreaming
    ? streamingThinking
    : message.thinking_content;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(effectiveContent);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy message:', err);
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        gap: '0.85rem',
        padding: '1.1rem 1.25rem',
        borderBottom: '1px solid var(--border-subtle)',
        backgroundColor: isUser ? 'rgba(255, 255, 255, 0.015)' : 'transparent',
        transition: 'background var(--transition-fast)',
        position: 'relative',
      }}
      className="message-row"
    >
      {/* Avatar */}
      <div style={{ flexShrink: 0, marginTop: '2px' }}>
        {isUser ? (
          <div
            style={{
              width: '30px',
              height: '30px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-surface-elevated)',
              border: '1px solid var(--border-medium)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--text-secondary)',
            }}
          >
            <User size={16} />
          </div>
        ) : (
          <div
            style={{
              width: '30px',
              height: '30px',
              borderRadius: 'var(--radius-sm)',
              background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.25) 0%, rgba(59, 130, 246, 0.25) 100%)',
              border: '1px solid rgba(99, 102, 241, 0.4)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#818cf8',
              boxShadow: '0 2px 8px rgba(99, 102, 241, 0.2)',
            }}
          >
            <Bot size={16} />
          </div>
        )}
      </div>

      {/* Message Body */}
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* Header line with role name & timestamps */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: '0.35rem',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontWeight: 600, fontSize: '0.84rem', color: 'var(--text-primary)' }}>
              {isUser ? 'You' : 'Assistant'}
            </span>

            {/* Status Badges */}
            {message.status === 'cancelled' && (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.2rem',
                  fontSize: '0.7rem',
                  color: 'var(--status-warning)',
                  backgroundColor: 'rgba(245, 158, 11, 0.12)',
                  border: '1px solid rgba(245, 158, 11, 0.25)',
                  padding: '0.1rem 0.4rem',
                  borderRadius: 'var(--radius-sm)',
                  fontWeight: 500,
                }}
              >
                <StopCircle size={10} />
                Cancelled
              </span>
            )}

            {message.status === 'failed' && (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.2rem',
                  fontSize: '0.7rem',
                  color: 'var(--status-error)',
                  backgroundColor: 'rgba(239, 68, 68, 0.12)',
                  border: '1px solid rgba(239, 68, 68, 0.25)',
                  padding: '0.1rem 0.4rem',
                  borderRadius: 'var(--radius-sm)',
                  fontWeight: 500,
                }}
              >
                <AlertTriangle size={10} />
                Failed
              </span>
            )}

            {message.status === 'interrupted' && (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.2rem',
                  fontSize: '0.7rem',
                  color: 'var(--text-dim)',
                  backgroundColor: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-subtle)',
                  padding: '0.1rem 0.4rem',
                  borderRadius: 'var(--radius-sm)',
                  fontWeight: 500,
                }}
              >
                Interrupted
              </span>
            )}
          </div>

          {/* Action toolbar on hover */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '0.25rem',
            }}
          >
            {/* Signature "Continue from here" Branch Action */}
            {onBranchFromHere && !isStreaming && (
              <button
                type="button"
                onClick={() => onBranchFromHere(message.id)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.3rem',
                  padding: '0.25rem 0.5rem',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'rgba(6, 182, 212, 0.08)',
                  border: '1px solid rgba(6, 182, 212, 0.25)',
                  color: 'var(--branch-accent)',
                  fontSize: '0.74rem',
                  fontWeight: 500,
                  transition: 'all var(--transition-fast)',
                }}
                title="Branch conversation from this point"
              >
                <GitBranch size={12} />
                <span>Continue from here</span>
              </button>
            )}

            {/* Copy Button */}
            <button
              type="button"
              onClick={handleCopy}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '0.3rem',
                padding: '0.25rem 0.45rem',
                borderRadius: 'var(--radius-sm)',
                color: 'var(--text-muted)',
                backgroundColor: 'transparent',
                fontSize: '0.74rem',
                transition: 'color var(--transition-fast)',
              }}
              title="Copy message text"
            >
              {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
            </button>

            {/* Retry Button (for user message or failed assistant message) */}
            {onRetry && (isUser || message.status === 'failed') && !isStreaming && (
              <button
                type="button"
                onClick={() => onRetry(effectiveContent)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  padding: '0.25rem 0.45rem',
                  borderRadius: 'var(--radius-sm)',
                  color: 'var(--text-muted)',
                  fontSize: '0.74rem',
                }}
                title="Retry prompt"
              >
                <RotateCcw size={13} />
              </button>
            )}
          </div>
        </div>

        {/* Collapsible Reasoning / Thinking Section */}
        {effectiveThinking && (
          <div
            style={{
              marginBottom: '0.75rem',
              borderRadius: 'var(--radius-md)',
              backgroundColor: 'var(--reasoning-bg)',
              border: '1px solid var(--reasoning-border)',
              overflow: 'hidden',
            }}
          >
            <button
              type="button"
              onClick={() => setIsThinkingExpanded((prev) => !prev)}
              style={{
                width: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '0.45rem 0.75rem',
                color: 'var(--reasoning-header)',
                fontSize: '0.76rem',
                fontWeight: 600,
                letterSpacing: '0.02em',
                background: 'rgba(139, 92, 246, 0.05)',
                borderBottom: isThinkingExpanded ? '1px solid var(--reasoning-border)' : 'none',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                <BrainCircuit size={13} />
                <span>Thinking Process</span>
              </div>
              {isThinkingExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            </button>

            {isThinkingExpanded && (
              <div
                style={{
                  padding: '0.75rem',
                  color: 'var(--reasoning-text)',
                  fontSize: '0.82rem',
                  lineHeight: 1.55,
                  textAlign: 'left',
                  maxHeight: '280px',
                  overflowY: 'auto',
                }}
              >
                <MarkdownRenderer content={effectiveThinking} />
                {isStreaming && !effectiveContent && <span className="streaming-cursor" />}
              </div>
            )}
          </div>
        )}

        {/* Content Display */}
        {isUser ? (
          <div
            style={{
              fontSize: '0.92rem',
              lineHeight: 1.6,
              color: 'var(--text-primary)',
              whiteSpace: 'pre-wrap',              
              textAlign: 'right'
            }}
          >
            {effectiveContent}
          </div>
        ) : (
          <div>
            <MarkdownRenderer content={effectiveContent} />
            {isStreaming && <span className="streaming-cursor" />}
          </div>
        )}
      </div>
    </div>
  );
};
