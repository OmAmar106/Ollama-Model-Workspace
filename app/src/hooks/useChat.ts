import { useCallback, useEffect, useRef, useState } from 'react';
import {
  cancelGeneration,
  createBranchSession,
  createSession,
  deleteSession,
  generateChat,
  getSessionMessages,
  listSessions,
  onChatStream,
  updateSession,
} from '../api/tauri';
import type {
  ChatStreamEvent,
  Message,
  Session,
} from '../api/types';

export interface ActiveBranchState {
  parentSessionId: string;
  branchedFromMessageId: string;
  branchSession: Session;
  messages: Message[];
  isGenerating: boolean;
  streamContent: string;
  streamThinking: string;
}

export function useChat(defaultModelName: string) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoadingMessages, setIsLoadingMessages] = useState<boolean>(false);

  // Generation state for current session
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  const [activeGenerationId, setActiveGenerationId] = useState<string | null>(null);
  const [streamContent, setStreamContent] = useState<string>('');
  const [streamThinking, setStreamThinking] = useState<string>('');
  const [isThinkEnabled, setIsThinkEnabled] = useState<boolean>(false);

  // Active branch sub-chat state (anchored above branched message)
  const [activeBranch, setActiveBranch] = useState<ActiveBranchState | null>(null);

  const activeGenIdRef = useRef<string | null>(null);
  activeGenIdRef.current = activeGenerationId;

  const currentSessionIdRef = useRef<string | null>(null);
  currentSessionIdRef.current = currentSessionId;

  const activeBranchRef = useRef<ActiveBranchState | null>(null);
  activeBranchRef.current = activeBranch;

  // Load all sessions on mount
  const refreshSessions = useCallback(async () => {
    try {
      const list = await listSessions();
      setSessions(list);
      return list;
    } catch (err) {
      console.error('Failed to list sessions:', err);
      return [];
    }
  }, []);

  useEffect(() => {
    refreshSessions().then((list) => {
      if (list.length > 0 && !currentSessionIdRef.current) {
        setCurrentSessionId(list[0].id);
      }
    });
  }, [refreshSessions]);

  // Load messages whenever currentSessionId changes
  const loadMessages = useCallback(async (sessionId: string) => {
    setIsLoadingMessages(true);
    try {
      const msgs = await getSessionMessages(sessionId);
      setMessages(msgs);
    } catch (err) {
      console.error('Failed to load messages for session:', sessionId, err);
      setMessages([]);
    } finally {
      setIsLoadingMessages(false);
    }
  }, []);

  useEffect(() => {
    if (currentSessionId) {
      loadMessages(currentSessionId);
      // Close branch when switching main session
      setActiveBranch(null);
    } else {
      setMessages([]);
    }
  }, [currentSessionId, loadMessages]);

  // Register chat streaming event listener
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await onChatStream((event: ChatStreamEvent) => {
        // 1. Check if event belongs to current main session
        if (event.session_id === currentSessionIdRef.current) {
          if (!event.done) {
            if (event.content) {
              setStreamContent((prev) => prev + event.content);
            }
            if (event.thinking) {
              setStreamThinking((prev) => prev + event.thinking);
            }
          } else {
            // Terminal event for main session: reload messages from DB
            setIsGenerating(false);
            setActiveGenerationId(null);
            setStreamContent('');
            setStreamThinking('');
            if (currentSessionIdRef.current) {
              loadMessages(currentSessionIdRef.current);
              refreshSessions();
            }
          }
        }

        // 2. Check if event belongs to active branch session
        if (
          activeBranchRef.current &&
          event.session_id === activeBranchRef.current.branchSession.id
        ) {
          if (!event.done) {
            setActiveBranch((prev) => {
              if (!prev) return null;
              return {
                ...prev,
                streamContent: prev.streamContent + (event.content || ''),
                streamThinking: prev.streamThinking + (event.thinking || ''),
              };
            });
          } else {
            // Terminal event for branch
            const branchId = activeBranchRef.current.branchSession.id;
            getSessionMessages(branchId).then((branchMsgs) => {
              setActiveBranch((prev) => {
                if (!prev) return null;
                return {
                  ...prev,
                  messages: branchMsgs,
                  isGenerating: false,
                  streamContent: '',
                  streamThinking: '',
                };
              });
              refreshSessions();
            });
          }
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [loadMessages, refreshSessions]);

  // Current session object
  const currentSession = sessions.find((s) => s.id === currentSessionId) || null;

  // Synchronize think mode with session preference
  useEffect(() => {
    if (currentSession?.think !== undefined && currentSession?.think !== null) {
      setIsThinkEnabled(currentSession.think);
    }
  }, [currentSession]);

  // Start a new empty chat
  const startNewChat = useCallback(() => {
    setCurrentSessionId(null);
    setMessages([]);
    setStreamContent('');
    setStreamThinking('');
    setIsGenerating(false);
    setActiveGenerationId(null);
    setActiveBranch(null);
  }, []);

  // Send message in current session (or create new session on first message)
  const sendMessage = useCallback(
    async (prompt: string, modelNameOverride?: string) => {
      if (!prompt.trim() || isGenerating) return;

      const model = modelNameOverride || currentSession?.model_name || defaultModelName;
      let targetSessionId = currentSessionId;

      // Optimistic user message representation
      const tempUserMsg: Message = {
        id: `temp-user-${Date.now()}`,
        session_id: targetSessionId || '',
        role: 'user',
        content: prompt.trim(),
        thinking_content: null,
        status: 'completed',
        created_at: new Date().toISOString(),
        completed_at: null,
      };

      setMessages((prev) => [...prev, tempUserMsg]);
      setIsGenerating(true);
      setStreamContent('');
      setStreamThinking('');

      try {
        const chatStart = await generateChat(
          model,
          prompt.trim(),
          targetSessionId,
          isThinkEnabled
        );

        setActiveGenerationId(chatStart.generation.id);

        // If this was a new session, select it and refresh sessions
        if (!targetSessionId) {
          targetSessionId = chatStart.session.id;
          setCurrentSessionId(chatStart.session.id);
          await refreshSessions();
        }

        // Reconcile messages with backend
        await loadMessages(targetSessionId);
      } catch (err: unknown) {
        console.error('Generation error or cancellation:', err);
        setIsGenerating(false);
        setActiveGenerationId(null);
        setStreamContent('');
        setStreamThinking('');
        if (targetSessionId) {
          await loadMessages(targetSessionId);
        }
      }
    },
    [
      isGenerating,
      currentSession?.model_name,
      defaultModelName,
      currentSessionId,
      isThinkEnabled,
      refreshSessions,
      loadMessages,
    ]
  );

  // Stop active generation
  const stopGeneration = useCallback(async () => {
    if (activeGenerationId) {
      try {
        await cancelGeneration(activeGenerationId);
      } catch (err) {
        console.error('Cancel generation error:', err);
      }
    } else if (currentSessionId) {
      // Fallback: cancel active generation for session
      try {
        const { cancelSession } = await import('../api/tauri');
        await cancelSession(currentSessionId);
      } catch (err) {
        console.error('Cancel session error:', err);
      }
    }
  }, [activeGenerationId, currentSessionId]);

  // Rename session
  const renameSession = useCallback(
    async (sessionId: string, title: string) => {
      try {
        const updated = await updateSession(sessionId, { title: title.trim() });
        setSessions((prev) => prev.map((s) => (s.id === sessionId ? updated : s)));
      } catch (err) {
        console.error('Failed to rename session:', err);
      }
    },
    []
  );

  // Update session model
  const changeSessionModel = useCallback(
    async (sessionId: string, modelName: string) => {
      try {
        const updated = await updateSession(sessionId, { modelName });
        setSessions((prev) => prev.map((s) => (s.id === sessionId ? updated : s)));
      } catch (err) {
        console.error('Failed to change session model:', err);
      }
    },
    []
  );

  // Update session system prompt
  const updateSystemPrompt = useCallback(
    async (sessionId: string, systemPrompt: string | null) => {
      try {
        const updated = await updateSession(sessionId, { systemPrompt });
        setSessions((prev) => prev.map((s) => (s.id === sessionId ? updated : s)));
      } catch (err) {
        console.error('Failed to update system prompt:', err);
        throw err;
      }
    },
    []
  );

  // Delete session
  const removeSession = useCallback(
    async (sessionId: string) => {
      try {
        await deleteSession(sessionId);
        setSessions((prev) => prev.filter((s) => s.id !== sessionId));
        if (currentSessionId === sessionId) {
          startNewChat();
        }
      } catch (err) {
        console.error('Failed to delete session:', err);
      }
    },
    [currentSessionId, startNewChat]
  );

  // Continue from here: Branch creation
  const startBranchFromMessage = useCallback(
    async (messageId: string) => {
      if (!currentSessionId) return;

      try {
        const branchSession = await createBranchSession(currentSessionId, messageId);
        const branchMessages = await getSessionMessages(branchSession.id);

        setActiveBranch({
          parentSessionId: currentSessionId,
          branchedFromMessageId: messageId,
          branchSession,
          messages: branchMessages,
          isGenerating: false,
          streamContent: '',
          streamThinking: '',
        });

        await refreshSessions();
      } catch (err) {
        console.error('Failed to create branch:', err);
      }
    },
    [currentSessionId, refreshSessions]
  );

  // Send message inside active branch
  const sendBranchMessage = useCallback(
    async (prompt: string) => {
      if (!activeBranch || !prompt.trim() || activeBranch.isGenerating) return;

      const branchSessionId = activeBranch.branchSession.id;
      const model = activeBranch.branchSession.model_name || defaultModelName;

      const tempMsg: Message = {
        id: `temp-branch-user-${Date.now()}`,
        session_id: branchSessionId,
        role: 'user',
        content: prompt.trim(),
        thinking_content: null,
        status: 'completed',
        created_at: new Date().toISOString(),
        completed_at: null,
      };

      setActiveBranch((prev) => {
        if (!prev) return null;
        return {
          ...prev,
          messages: [...prev.messages, tempMsg],
          isGenerating: true,
          streamContent: '',
          streamThinking: '',
        };
      });

      try {
        await generateChat(
          model,
          prompt.trim(),
          branchSessionId,
          activeBranch.branchSession.think
        );

        const refreshed = await getSessionMessages(branchSessionId);
        setActiveBranch((prev) => {
          if (!prev) return null;
          return {
            ...prev,
            messages: refreshed,
            isGenerating: false,
            streamContent: '',
            streamThinking: '',
          };
        });
        await refreshSessions();
      } catch (err) {
        console.error('Branch generation error:', err);
        const refreshed = await getSessionMessages(branchSessionId);
        setActiveBranch((prev) => {
          if (!prev) return null;
          return {
            ...prev,
            messages: refreshed,
            isGenerating: false,
            streamContent: '',
            streamThinking: '',
          };
        });
      }
    },
    [activeBranch, defaultModelName, refreshSessions]
  );

  // Close branch panel
  const closeBranch = useCallback(() => {
    setActiveBranch(null);
  }, []);

  return {
    sessions,
    currentSessionId,
    currentSession,
    messages,
    isLoadingMessages,
    isGenerating,
    streamContent,
    streamThinking,
    isThinkEnabled,
    setIsThinkEnabled,
    activeBranch,
    startNewChat,
    selectSession: setCurrentSessionId,
    sendMessage,
    stopGeneration,
    renameSession,
    changeSessionModel,
    updateSystemPrompt,
    removeSession,
    startBranchFromMessage,
    sendBranchMessage,
    closeBranch,
    refreshSessions,
  };
}
