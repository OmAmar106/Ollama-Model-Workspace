import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ChatStart,
  ChatStreamEvent,
  Generation,
  HealthStatus,
  Message,
  ModelInfo,
  Session,
} from './types';

export async function createSession(
  modelName: string,
  think?: boolean | null,
  title?: string | null,
  systemPrompt?: string | null
): Promise<Session> {
  return await invoke<Session>('create_session', {
    modelName,
    think,
    title,
    systemPrompt,
  });
}

export async function getSession(sessionId: string): Promise<Session | null> {
  return await invoke<Session | null>('get_session', { sessionId });
}

export async function listSessions(): Promise<Session[]> {
  return await invoke<Session[]>('list_sessions');
}

export async function updateSession(
  sessionId: string,
  params: {
    title?: string | null;
    modelName?: string | null;
    think?: boolean | null;
    systemPrompt?: string | null;
  }
): Promise<Session> {
  return await invoke<Session>('update_session', {
    sessionId,
    title: params.title,
    modelName: params.modelName,
    think: params.think,
    systemPrompt: params.systemPrompt,
  });
}

export async function deleteSession(sessionId: string): Promise<boolean> {
  return await invoke<boolean>('delete_session', { sessionId });
}

export async function createBranchSession(
  parentSessionId: string,
  messageId: string,
  title?: string | null
): Promise<Session> {
  return await invoke<Session>('create_branch_session', {
    parentSessionId,
    messageId,
    title,
  });
}

export async function getSessionMessages(sessionId: string): Promise<Message[]> {
  return await invoke<Message[]>('get_session_messages', { sessionId });
}

export async function getGeneration(
  generationId: string
): Promise<Generation | null> {
  return await invoke<Generation | null>('get_generation', { generationId });
}

export async function checkOllamaHealth(): Promise<HealthStatus> {
  return await invoke<HealthStatus>('check_ollama_health');
}

export async function listOllamaModels(): Promise<ModelInfo[]> {
  return await invoke<ModelInfo[]>('list_ollama_models');
}

export async function generateChat(
  modelName: string,
  prompt: string,
  sessionId?: string | null,
  think?: boolean | null
): Promise<ChatStart> {
  return await invoke<ChatStart>('generate_chat', {
    modelName,
    prompt,
    sessionId,
    think,
  });
}

export async function cancelGeneration(
  generationId: string
): Promise<boolean> {
  return await invoke<boolean>('cancel_generation', { generationId });
}

export async function cancelSession(sessionId: string): Promise<boolean> {
  return await invoke<boolean>('cancel_session', { sessionId });
}

export async function onChatStream(
  callback: (event: ChatStreamEvent) => void
): Promise<UnlistenFn> {
  return await listen<ChatStreamEvent>('chat-stream', (e) => {
    callback(e.payload);
  });
}
