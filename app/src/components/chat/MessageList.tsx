import React, { useEffect, useRef, useState, useCallback } from 'react';
import { ArrowDown } from 'lucide-react';
import { MessageItem } from './MessageItem';
import { EmptyState } from './EmptyState';
import { BranchSubChatPanel } from '../panels/BranchSubChatPanel';
import type { Message } from '../../api/types';
import type { ActiveBranchState } from '../../hooks/useChat';

interface MessageListProps {
  messages: Message[];
  isLoading: boolean;
  isStreaming: boolean;
  streamingContent: string;
  streamingThinking: string;
  selectedModel: string;
  onSelectSuggestion: (prompt: string) => void;
  onBranchFromHere: (messageId: string) => void;
  onRetry: (prompt: string) => void;
  activeBranch: ActiveBranchState | null;
  onSendBranchMessage: (prompt: string) => void;
  onCloseBranch: () => void;
}

export const MessageList: React.FC<MessageListProps> = ({
  messages,
  isLoading,
  isStreaming,
  streamingContent,
  streamingThinking,
  selectedModel,
  onSelectSuggestion,
  onBranchFromHere,
  onRetry,
  activeBranch,
  onSendBranchMessage,
  onCloseBranch,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const [showScrollBottomBtn, setShowScrollBottomBtn] = useState(false);

  // Check if user is scrolled to bottom
  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
    const atBottom = distanceFromBottom < 80;
    setIsAtBottom(atBottom);
    setShowScrollBottomBtn(distanceFromBottom > 160);
  }, []);

  // Auto-scroll on new content only if user was already at bottom
  useEffect(() => {
    if (isAtBottom && containerRef.current) {
      containerRef.current.scrollTo({
        top: containerRef.current.scrollHeight,
        behavior: isStreaming ? 'auto' : 'smooth',
      });
    }
  }, [messages, streamingContent, streamingThinking, isAtBottom, isStreaming, activeBranch]);

  const scrollToBottom = () => {
    if (containerRef.current) {
      containerRef.current.scrollTo({
        top: containerRef.current.scrollHeight,
        behavior: 'smooth',
      });
      setIsAtBottom(true);
      setShowScrollBottomBtn(false);
    }
  };

  if (isLoading && messages.length === 0) {
    return (
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-muted)',
          fontSize: '0.84rem',
        }}
      >
        Loading messages...
      </div>
    );
  }

  if (messages.length === 0 && !isStreaming) {
    return (
      <div style={{ flex: 1, overflowY: 'auto' }}>
        <EmptyState
          selectedModel={selectedModel}
          onSelectSuggestion={onSelectSuggestion}
        />
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      style={{
        flex: 1,
        overflowY: 'auto',
        display: 'flex',
        flexDirection: 'column',
        position: 'relative',
      }}
    >
      <div style={{ maxWidth: '820px', width: '100%', margin: '0 auto', paddingBottom: '2rem' }}>
        {messages
          .filter((message) => !(isStreaming && message.status === 'generating'))
          .map((message) => {
            const isAnchoredBranch = activeBranch?.branchedFromMessageId === message.id;

            return (
              <React.Fragment key={message.id}>
                <MessageItem
                  message={message}
                  onBranchFromHere={onBranchFromHere}
                  onRetry={onRetry}
                />

                {/* Anchored Sub-Chat Branch Panel above/below selected message point */}
                {isAnchoredBranch && activeBranch && (
                  <BranchSubChatPanel
                    branchState={activeBranch}
                    onSendMessage={onSendBranchMessage}
                    onClose={onCloseBranch}
                  />
                )}
              </React.Fragment>
            );
          })}

        {/* In-Flight Streaming Message */}
        {isStreaming && (
          <MessageItem
            message={{
              id: 'streaming-assistant-msg',
              session_id: '',
              role: 'assistant',
              content: '',
              thinking_content: null,
              status: 'generating',
              created_at: new Date().toISOString(),
              completed_at: null,
            }}
            isStreaming={true}
            streamingContent={streamingContent}
            streamingThinking={streamingThinking}
          />
        )}
      </div>

      {/* Floating Scroll to Bottom Button */}
      {showScrollBottomBtn && (
        <button
          type="button"
          onClick={scrollToBottom}
          style={{
            position: 'absolute',
            bottom: '1.25rem',
            right: '2rem',
            width: '36px',
            height: '36px',
            borderRadius: '50%',
            backgroundColor: 'var(--bg-surface-elevated)',
            border: '1px solid var(--border-medium)',
            color: 'var(--text-primary)',
            boxShadow: 'var(--shadow-lg)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'all var(--transition-fast)',
            zIndex: 10,
          }}
          title="Scroll to bottom"
        >
          <ArrowDown size={16} />
        </button>
      )}
    </div>
  );
};
