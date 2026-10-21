export interface Session {
  id: string;
  title: string | null;
  model_name: string;
  think: boolean | null;
  system_prompt: string | null;
  options_json: string | null;
  created_at: string;
  updated_at: string;
}

export type MessageRole = 'user' | 'assistant' | 'system';
export type MessageStatus =
  | 'completed'
  | 'generating'
  | 'cancelled'
  | 'failed'
  | 'interrupted';

export interface Message {
  id: string;
  session_id: string;
  role: MessageRole;
  content: string;
  thinking_content: string | null;
  status: MessageStatus;
  created_at: string;
  completed_at: string | null;
}

export type GenerationStatus =
  | 'running'
  | 'completed'
  | 'cancelled'
  | 'failed'
  | 'interrupted';

export interface Generation {
  id: string;
  message_id: string;
  model_name: string;
  status: GenerationStatus;
  started_at: string;
  finished_at: string | null;
  time_to_first_token_ms: number | null;
  duration_ms: number | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  tokens_per_second: number | null;
  error_code: string | null;
  error_message: string | null;
}

export interface ChatStart {
  session: Session;
  user_message: Message;
  assistant_message: Message;
  generation: Generation;
}

export interface ChatStreamEvent {
  session_id: string;
  user_message_id: string;
  assistant_message_id: string;
  generation_id: string;
  content: string;
  thinking: string | null;
  done: boolean;
  status: string;
  error: string | null;
}

export interface HealthStatus {
  is_healthy: boolean;
  version: string | null;
  base_url: string;
  error: string | null;
}

export interface ModelDetails {
  parent_model?: string | null;
  format?: string | null;
  family?: string | null;
  families?: string[] | null;
  parameter_size?: string | null;
  quantization_level?: string | null;
}

export interface ModelInfo {
  name: string;
  model?: string | null;
  modified_at?: string | null;
  size?: number | null;
  digest?: string | null;
  details?: ModelDetails | null;
}

export interface BranchMetadata {
  parent_session_id: string;
  branched_from_message_id: string;
  is_branch: boolean;
}
