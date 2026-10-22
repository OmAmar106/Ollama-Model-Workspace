import { useCallback, useEffect, useRef, useState } from 'react';
import {
  cancelGeneration,
  cancelSession,
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

interface SessionStreamState {
  isGenerating: boolean;
  generationId: string | null;
  content: string;
  thinking: string;
}

function appendStreamChunk(
  state: SessionStreamState,
  content: string,
  thinking: string
): SessionStreamState {
  return {
    ...state,
    content: state.content + content,
    thinking: state.thinking + thinking,
  };
}

export function useChat(defaultModelName: string) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoadingMessages, setIsLoadingMessages] = useState<boolean>(false);

  // Generation state is tracked per session so background chats can finish
  // without leaking streaming output into the currently viewed chat.
  const [sessionStreams, setSessionStreams] = useState<
    Record<string, SessionStreamState>
  >({});
  const [isThinkEnabled, setIsThinkEnabled] = useState<boolean>(false);

  const currentStream = currentSessionId
    ? sessionStreams[currentSessionId]
    : undefined;
  const isGenerating = currentStream?.isGenerating ?? false;
  const activeGenerationId = currentStream?.generationId ?? null;
  const streamContent = currentStream?.content ?? '';
  const streamThinking = currentStream?.thinking ?? '';

  // Active branch sub-chat state (anchored above branched message)
  const [activeBranch, setActiveBranch] = useState<ActiveBranchState | null>(null);

  const currentSessionIdRef = useRef<string | null>(null);
  const activeBranchRef = useRef<ActiveBranchState | null>(null);

  useEffect(() => {
    currentSessionIdRef.current = currentSessionId;
  }, [currentSessionId]);

  useEffect(() => {
    activeBranchRef.current = activeBranch;
  }, [activeBranch]);

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
    let cancelled = false;

    const setupListener = async () => {
      const dispose = await onChatStream((event: ChatStreamEvent) => {
        const isBranchEvent =
          activeBranchRef.current?.branchSession.id === event.session_id;

        // Branch sessions keep their own streaming state.
        if (isBranchEvent) {
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
            const branchId = event.session_id;
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
          return;
        }

        if (!event.done) {
          // Update the owning session even if it is not currently visible, so
          // switching back shows the accumulated output.
          setSessionStreams((prev) => {
            const existing = prev[event.session_id];
            if (!existing) return prev;
            return {
              ...prev,
              [event.session_id]: appendStreamChunk(
                existing,
                event.content || '',
                event.thinking || ''
              ),
            };
          });
        } else {
          // Terminal event always clears only its own session's stream.
          setSessionStreams((prev) => {
            if (!prev[event.session_id]) return prev;
            const next = { ...prev };
            delete next[event.session_id];
            return next;
          });
          refreshSessions();
          if (event.session_id === currentSessionIdRef.current) {
            loadMessages(event.session_id);
          }
        }
      });

      if (cancelled) {
        dispose();
      } else {
        unlisten = dispose;
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = undefined;
      }
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
    setActiveBranch(null);
  }, []);

  // Send message in current session (or create new session on first message)
  const sendMessage = useCallback(
    async (prompt: string, modelNameOverride?: string) => {
      if (!prompt.trim() || isGenerating) return;

      const model = modelNameOverride || currentSession?.model_name || defaultModelName;
      let targetSessionId = currentSessionId;

      try {
        if (!targetSessionId) {
          const newSession = await createSession(model, isThinkEnabled, null, null);
          targetSessionId = newSession.id;
          currentSessionIdRef.current = newSession.id;
          setCurrentSessionId(newSession.id);
          setMessages([]);
          await refreshSessions();
        }

        const sessionId = targetSessionId;

        // Optimistic user message representation
        const tempUserMsg: Message = {
          id: `temp-user-${Date.now()}`,
          session_id: sessionId,
          role: 'user',
          content: prompt.trim(),
          thinking_content: null,
          status: 'completed',
          created_at: new Date().toISOString(),
          completed_at: null,
        };

        setMessages((prev) => [...prev, tempUserMsg]);
        setSessionStreams((prev) => ({
          ...prev,
          [sessionId]: {
            isGenerating: true,
            generationId: null,
            content: '',
            thinking: '',
          },
        }));

        await generateChat(
          model,
          prompt.trim(),
          sessionId,
          isThinkEnabled
        );

        // The terminal event normally clears this first; make sure nothing is
        // left behind if event delivery is delayed.
        setSessionStreams((prev) => {
          if (!prev[sessionId]) return prev;
          const next = { ...prev };
          delete next[sessionId];
          return next;
        });

        // Only reconcile if the user is still viewing this session.
        if (sessionId === currentSessionIdRef.current) {
          await loadMessages(sessionId);
        }
      } catch (err: unknown) {
        console.error('Generation error or cancellation:', err);
        const sessionId = targetSessionId;
        if (sessionId) {
          setSessionStreams((prev) => {
            if (!prev[sessionId]) return prev;
            const next = { ...prev };
            delete next[sessionId];
            return next;
          });
          if (sessionId === currentSessionIdRef.current) {
            await loadMessages(sessionId);
          }
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
