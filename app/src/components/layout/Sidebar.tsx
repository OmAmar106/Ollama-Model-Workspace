import React, { useMemo, useState } from 'react';
import {
  Plus,
  Search,
  MessageSquare,
  GitBranch,
  Trash2,
  Pencil,
  MoreVertical,
} from 'lucide-react';
import type { Session } from '../../api/types';

interface SidebarProps {
  sessions: Session[];
  currentSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onNewChat: () => void;
  onRenameSession: (sessionId: string, newTitle: string) => void;
  onDeleteSession: (sessionId: string) => void;
  isCollapsed: boolean;
}

interface GroupedSessions {
  today: Session[];
  yesterday: Session[];
  lastWeek: Session[];
  older: Session[];
}

export const Sidebar: React.FC<SidebarProps> = ({
  sessions,
  currentSessionId,
  onSelectSession,
  onNewChat,
  onRenameSession,
  onDeleteSession,
  isCollapsed,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [sessionToDelete, setSessionToDelete] = useState<Session | null>(null);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState('');
  const [actionMenuOpenId, setActionMenuOpenId] = useState<string | null>(null);

  // Group sessions by activity date
  const groupedSessions = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    const filtered = sessions.filter((s) => {
      if (!query) return true;
      const titleMatch = (s.title || 'Untitled Chat').toLowerCase().includes(query);
      const modelMatch = s.model_name.toLowerCase().includes(query);
      return titleMatch || modelMatch;
    });

    const now = new Date();
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const yesterdayStart = todayStart - 86400000;
    const lastWeekStart = todayStart - 7 * 86400000;

    const groups: GroupedSessions = {
      today: [],
      yesterday: [],
      lastWeek: [],
      older: [],
    };

    for (const session of filtered) {
      const date = new Date(session.updated_at || session.created_at).getTime();
      if (date >= todayStart) {
        groups.today.push(session);
      } else if (date >= yesterdayStart) {
        groups.yesterday.push(session);
      } else if (date >= lastWeekStart) {
        groups.lastWeek.push(session);
      } else {
        groups.older.push(session);
      }
    }

    return groups;
  }, [sessions, searchQuery]);

  const handleStartRename = (session: Session) => {
    setEditingSessionId(session.id);
    setEditTitle(session.title || 'Untitled Chat');
    setActionMenuOpenId(null);
  };

  const handleFinishRename = (sessionId: string) => {
    if (editTitle.trim()) {
      onRenameSession(sessionId, editTitle.trim());
    }
    setEditingSessionId(null);
  };

  return (
    <aside className={`sidebar ${isCollapsed ? 'collapsed' : ''}`}>
      {/* App Header */}
      <div
        style={{
          padding: '1rem',
          borderBottom: '1px solid var(--border-subtle)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
          <div
            style={{
              width: '28px',
              height: '28px',
              borderRadius: 'var(--radius-sm)',
              background: 'linear-gradient(135deg, #6366f1 0%, #3b82f6 100%)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 2px 8px rgba(99, 102, 241, 0.4)',
            }}
          >
          <img
            src="/icons.svg"
            alt="App logo"
            width={40}
            height={40}
            style={{ objectFit: 'contain' }}
          />          </div>
          <div>
            <div style={{ fontWeight: 700, fontSize: '0.92rem', letterSpacing: '-0.01em' }}>
              Ollama Workspace
            </div>
            <div style={{ fontSize: '0.68rem', color: 'var(--text-dim)', fontWeight: 500 }}>
              Local AI Engine
            </div>
          </div>
        </div>
      </div>

      {/* New Chat Button */}
      <div style={{ padding: '0.75rem 0.85rem 0.4rem' }}>
        <button
          type="button"
          onClick={onNewChat}
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '0.6rem 0.85rem',
            backgroundColor: 'var(--accent-primary)',
            color: '#ffffff',
            borderRadius: 'var(--radius-md)',
            fontWeight: 600,
            fontSize: '0.86rem',
            boxShadow: 'var(--shadow-sm)',
            transition: 'background var(--transition-fast)',
          }}
          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--accent-primary-hover)')}
          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'var(--accent-primary)')}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Plus size={16} />
            <span>New Chat</span>
          </div>
          <span
            style={{
              fontSize: '0.7rem',
              backgroundColor: 'rgba(255, 255, 255, 0.2)',
              padding: '0.1rem 0.35rem',
              borderRadius: '4px',
            }}
          >
            ⌘N
          </span>
        </button>
      </div>

      {/* Search Input */}
      <div style={{ padding: '0.4rem 0.85rem 0.6rem' }}>
        <div
          style={{
            position: 'relative',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <Search
            size={13}
            color="var(--text-muted)"
            style={{ position: 'absolute', left: '0.65rem' }}
          />
          <input
            type="text"
            placeholder="Search conversations..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: '100%',
              padding: '0.4rem 0.65rem 0.4rem 1.85rem',
              backgroundColor: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--text-primary)',
              fontSize: '0.78rem',
            }}
          />
        </div>
      </div>

      {/* Session History List */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '0.2rem 0.65rem 1rem',
        }}
      >
        <SessionGroup
          title="Today"
          sessions={groupedSessions.today}
          currentSessionId={currentSessionId}
          onSelect={onSelectSession}
          editingSessionId={editingSessionId}
          editTitle={editTitle}
          onEditTitleChange={setEditTitle}
          onFinishRename={handleFinishRename}
          onStartRename={handleStartRename}
          onConfirmDelete={setSessionToDelete}
          actionMenuOpenId={actionMenuOpenId}
          setActionMenuOpenId={setActionMenuOpenId}
        />

        <SessionGroup
          title="Yesterday"
          sessions={groupedSessions.yesterday}
          currentSessionId={currentSessionId}
          onSelect={onSelectSession}
          editingSessionId={editingSessionId}
          editTitle={editTitle}
          onEditTitleChange={setEditTitle}
          onFinishRename={handleFinishRename}
          onStartRename={handleStartRename}
          onConfirmDelete={setSessionToDelete}
          actionMenuOpenId={actionMenuOpenId}
          setActionMenuOpenId={setActionMenuOpenId}
        />

        <SessionGroup
          title="Previous 7 Days"
          sessions={groupedSessions.lastWeek}
          currentSessionId={currentSessionId}
          onSelect={onSelectSession}
          editingSessionId={editingSessionId}
          editTitle={editTitle}
          onEditTitleChange={setEditTitle}
          onFinishRename={handleFinishRename}
          onStartRename={handleStartRename}
          onConfirmDelete={setSessionToDelete}
          actionMenuOpenId={actionMenuOpenId}
          setActionMenuOpenId={setActionMenuOpenId}
        />

        <SessionGroup
          title="Older"
          sessions={groupedSessions.older}
          currentSessionId={currentSessionId}
          onSelect={onSelectSession}
          editingSessionId={editingSessionId}
          editTitle={editTitle}
          onEditTitleChange={setEditTitle}
          onFinishRename={handleFinishRename}
          onStartRename={handleStartRename}
          onConfirmDelete={setSessionToDelete}
          actionMenuOpenId={actionMenuOpenId}
          setActionMenuOpenId={setActionMenuOpenId}
        />

        {sessions.length === 0 && (
          <div
            style={{
              padding: '2rem 1rem',
              textAlign: 'center',
              color: 'var(--text-muted)',
              fontSize: '0.8rem',
            }}
          >
            No chat history yet. Click <strong>New Chat</strong> to begin.
          </div>
        )}
      </div>

      {/* Delete Confirmation Modal */}
      {sessionToDelete && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.65)',
            backdropFilter: 'blur(3px)',
            zIndex: 1000,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '1rem',
          }}
          onClick={() => setSessionToDelete(null)}
        >
          <div
            style={{
              width: '380px',
              backgroundColor: 'var(--bg-surface-elevated)',
              border: '1px solid var(--border-medium)',
              borderRadius: 'var(--radius-lg)',
              padding: '1.25rem',
              boxShadow: 'var(--shadow-xl)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ fontWeight: 600, fontSize: '1rem', marginBottom: '0.5rem' }}>
              Delete conversation?
            </div>
            <div
              style={{
                fontSize: '0.82rem',
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
                marginBottom: '1.25rem',
              }}
            >
              This will permanently delete <strong>{sessionToDelete.title || 'Untitled Chat'}</strong>{' '}
              and all of its messages. This action cannot be undone.
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
              <button
                type="button"
                onClick={() => setSessionToDelete(null)}
                style={{
                  padding: '0.45rem 0.85rem',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--bg-surface)',
                  border: '1px solid var(--border-subtle)',
                  fontSize: '0.82rem',
                  color: 'var(--text-secondary)',
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  onDeleteSession(sessionToDelete.id);
                  setSessionToDelete(null);
                }}
                style={{
                  padding: '0.45rem 0.85rem',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--status-error)',
                  fontSize: '0.82rem',
                  color: '#ffffff',
                  fontWeight: 600,
                }}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
};

interface SessionGroupProps {
  title: string;
  sessions: Session[];
  currentSessionId: string | null;
  onSelect: (id: string) => void;
  editingSessionId: string | null;
  editTitle: string;
  onEditTitleChange: (v: string) => void;
  onFinishRename: (id: string) => void;
  onStartRename: (s: Session) => void;
  onConfirmDelete: (s: Session) => void;
  actionMenuOpenId: string | null;
  setActionMenuOpenId: (id: string | null) => void;
}

const SessionGroup: React.FC<SessionGroupProps> = ({
  title,
  sessions,
  currentSessionId,
  onSelect,
  editingSessionId,
  editTitle,
  onEditTitleChange,
  onFinishRename,
  onStartRename,
  onConfirmDelete,
  actionMenuOpenId,
  setActionMenuOpenId,
}) => {
  if (sessions.length === 0) return null;

  return (
    <div style={{ marginBottom: '1rem' }}>
      <div
        style={{
          fontSize: '0.68rem',
          fontWeight: 700,
          color: 'var(--text-dim)',
          textTransform: 'uppercase',
          letterSpacing: '0.06em',
          padding: '0.35rem 0.5rem 0.2rem',
        }}
      >
        {title}
      </div>
      <div>
        {sessions.map((session) => {
          const isSelected = session.id === currentSessionId;
          const isEditing = editingSessionId === session.id;
          const isBranch = session.options_json?.includes('"is_branch":true');
          const isMenuOpen = actionMenuOpenId === session.id;

          return (
            <div
              key={session.id}
              style={{
                position: 'relative',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '0.42rem 0.55rem',
                borderRadius: 'var(--radius-sm)',
                backgroundColor: isSelected ? 'var(--bg-surface-active)' : 'transparent',
                border: `1px solid ${isSelected ? 'var(--border-accent)' : 'transparent'}`,
                marginBottom: '2px',
                cursor: 'pointer',
                transition: 'background var(--transition-fast)',
              }}
              onClick={() => onSelect(session.id)}
              onMouseEnter={(e) => {
                if (!isSelected) e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
              }}
              onMouseLeave={(e) => {
                if (!isSelected) e.currentTarget.style.backgroundColor = 'transparent';
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.45rem',
                  minWidth: 0,
                  flex: 1,
                }}
              >
                {isBranch ? (
                  <GitBranch size={13} color="var(--branch-accent)" style={{ flexShrink: 0 }} />
                ) : (
                  <MessageSquare
                    size={13}
                    color={isSelected ? 'var(--accent-primary)' : 'var(--text-muted)'}
                    style={{ flexShrink: 0 }}
                  />
                )}

                {isEditing ? (
                  <input
                    type="text"
                    value={editTitle}
                    onChange={(e) => onEditTitleChange(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') onFinishRename(session.id);
                      if (e.key === 'Escape') onFinishRename(session.id);
                    }}
                    onBlur={() => onFinishRename(session.id)}
                    autoFocus
                    onClick={(e) => e.stopPropagation()}
                    style={{
                      width: '100%',
                      backgroundColor: 'var(--bg-surface-elevated)',
                      border: '1px solid var(--accent-primary)',
                      color: 'var(--text-primary)',
                      fontSize: '0.8rem',
                      padding: '0.15rem 0.35rem',
                      borderRadius: '3px',
                    }}
                  />
                ) : (
                  <span
                    style={{
                      fontSize: '0.82rem',
                      fontWeight: isSelected ? 600 : 400,
                      color: isSelected ? 'var(--text-primary)' : 'var(--text-secondary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {session.title || 'Untitled Chat'}
                  </span>
                )}
              </div>

              {/* Actions Context Menu */}
              {!isEditing && (
                <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setActionMenuOpenId(isMenuOpen ? null : session.id);
                    }}
                    style={{
                      padding: '2px',
                      borderRadius: '4px',
                      color: 'var(--text-dim)',
                    }}
                    title="Session actions"
                    aria-label="Session actions"
                  >
                    <MoreVertical size={13} />
                  </button>

                  {isMenuOpen && (
                    <>
                      <div
                        style={{ position: 'fixed', inset: 0, zIndex: 60 }}
                        onClick={(e) => {
                          e.stopPropagation();
                          setActionMenuOpenId(null);
                        }}
                      />
                      <div
                        style={{
                          position: 'absolute',
                          top: '100%',
                          right: 0,
                          backgroundColor: 'var(--bg-surface-elevated)',
                          border: '1px solid var(--border-medium)',
                          borderRadius: 'var(--radius-sm)',
                          boxShadow: 'var(--shadow-md)',
                          padding: '0.25rem',
                          zIndex: 70,
                          minWidth: '110px',
                        }}
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          onClick={() => onStartRename(session)}
                          style={{
                            width: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            padding: '0.35rem 0.55rem',
                            fontSize: '0.76rem',
                            color: 'var(--text-primary)',
                            borderRadius: '3px',
                          }}
                          onMouseEnter={(e) =>
                            (e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)')
                          }
                          onMouseLeave={(e) =>
                            (e.currentTarget.style.backgroundColor = 'transparent')
                          }
                        >
                          <Pencil size={12} />
                          <span>Rename</span>
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            setActionMenuOpenId(null);
                            onConfirmDelete(session);
                          }}
                          style={{
                            width: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            padding: '0.35rem 0.55rem',
                            fontSize: '0.76rem',
                            color: 'var(--status-error)',
                            borderRadius: '3px',
                          }}
                          onMouseEnter={(e) =>
                            (e.currentTarget.style.backgroundColor = 'rgba(239, 68, 68, 0.12)')
                          }
                          onMouseLeave={(e) =>
                            (e.currentTarget.style.backgroundColor = 'transparent')
                          }
                        >
                          <Trash2 size={12} />
                          <span>Delete</span>
                        </button>
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
