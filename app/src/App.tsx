import { useState, useEffect, useCallback } from 'react';
import { Sidebar } from './components/layout/Sidebar';
import { Header } from './components/layout/Header';
import { MessageList } from './components/chat/MessageList';
import { Composer } from './components/chat/Composer';
import { SystemInstructionsDrawer } from './components/panels/SystemInstructionsDrawer';
import { useChat } from './hooks/useChat';
import { useModels } from './hooks/useModels';
import { useOllamaHealth } from './hooks/useOllamaHealth';
import './styles/index.css';

export function App() {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isInstructionsOpen, setIsInstructionsOpen] = useState(false);

  const {
    models,
    selectedModel,
    selectModel,
    refreshModels,
  } = useModels();

  const {
    health,
    isChecking: isCheckingHealth,
    refreshHealth,
  } = useOllamaHealth();

  const {
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
    selectSession,
    sendMessage,
    stopGeneration,
    renameSession,
    changeSessionModel,
    updateSystemPrompt,
    removeSession,
    startBranchFromMessage,
    sendBranchMessage,
    closeBranch,
  } = useChat(selectedModel);

  // Synchronize current session's model with selectedModel
  useEffect(() => {
    if (currentSession?.model_name && currentSession.model_name !== selectedModel) {
      selectModel(currentSession.model_name);
    }
  }, [currentSession, selectedModel, selectModel]);

  const handleSelectModel = useCallback(
    (modelName: string) => {
      selectModel(modelName);
      if (currentSessionId) {
        changeSessionModel(currentSessionId, modelName);
      }
    },
    [selectModel, currentSessionId, changeSessionModel]
  );

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      // ⌘N or Ctrl+N -> New Chat
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'n') {
        e.preventDefault();
        startNewChat();
      }

      // ⌘B or Ctrl+B -> Toggle Sidebar
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSidebarCollapsed((prev) => !prev);
      }

      // Esc -> Stop generation or close drawer
      if (e.key === 'Escape') {
        if (isInstructionsOpen) {
          setIsInstructionsOpen(false);
        } else if (isGenerating) {
          stopGeneration();
        } else if (activeBranch) {
          closeBranch();
        }
      }
    };

    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, [startNewChat, isInstructionsOpen, isGenerating, stopGeneration, activeBranch, closeBranch]);

  return (
    <div className="app-container">
      {/* Collapsible Left Sidebar */}
      <Sidebar
        sessions={sessions}
        currentSessionId={currentSessionId}
        onSelectSession={selectSession}
        onNewChat={startNewChat}
        onRenameSession={renameSession}
        onDeleteSession={removeSession}
        isCollapsed={isSidebarCollapsed}
      />

      {/* Main Conversation Workspace */}
      <div className="main-content">
        <Header
          currentSession={currentSession}
          models={models}
          selectedModel={selectedModel}
          onSelectModel={handleSelectModel}
          onRenameSession={renameSession}
          onToggleSidebar={() => setIsSidebarCollapsed((prev) => !prev)}
          isSidebarCollapsed={isSidebarCollapsed}
          onOpenSystemInstructions={() => setIsInstructionsOpen(true)}
          hasSystemInstructions={Boolean(currentSession?.system_prompt?.trim())}
          health={health}
          isCheckingHealth={isCheckingHealth}
          onRefreshHealth={() => {
            refreshHealth();
            refreshModels();
          }}
        />

        <MessageList
          messages={messages}
          isLoading={isLoadingMessages}
          isStreaming={isGenerating}
          streamingContent={streamContent}
          streamingThinking={streamThinking}
          selectedModel={selectedModel}
          onSelectSuggestion={(prompt) => sendMessage(prompt, selectedModel)}
          onBranchFromHere={startBranchFromMessage}
          onRetry={(prompt) => sendMessage(prompt, selectedModel)}
          activeBranch={activeBranch}
          onSendBranchMessage={sendBranchMessage}
          onCloseBranch={closeBranch}
        />

        <Composer
          onSend={(prompt) => sendMessage(prompt, selectedModel)}
          onStop={stopGeneration}
          isGenerating={isGenerating}
          models={models}
          selectedModel={selectedModel}
          onSelectModel={handleSelectModel}
          isThinkEnabled={isThinkEnabled}
          onToggleThink={() => setIsThinkEnabled((prev) => !prev)}
        />
      </div>

      {/* Per-Chat System Instructions Drawer */}
      <SystemInstructionsDrawer
        isOpen={isInstructionsOpen}
        onClose={() => setIsInstructionsOpen(false)}
        savedInstructions={currentSession?.system_prompt || null}
        onSave={async (newPrompt) => {
          if (currentSessionId) {
            await updateSystemPrompt(currentSessionId, newPrompt);
          }
        }}
        sessionTitle={currentSession?.title || 'Current Session'}
      />
    </div>
  );
}

export default App;
