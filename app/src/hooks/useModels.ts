import { useCallback, useEffect, useState } from 'react';
import { listOllamaModels } from '../api/tauri';
import type { ModelInfo } from '../api/types';

const STORED_MODEL_KEY = 'ollama_workspace_preferred_model';

export function useModels() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>(() => {
    return localStorage.getItem(STORED_MODEL_KEY) || 'llama3.2';
  });
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const fetchModels = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await listOllamaModels();
      setModels(list);

      // If selectedModel is not in list and list is not empty, pick first installed model
      if (list.length > 0) {
        const exists = list.some(
          (m) => m.name === selectedModel || m.name.startsWith(`${selectedModel}:`)
        );
        if (!exists) {
          const first = list[0].name;
          setSelectedModel(first);
          localStorage.setItem(STORED_MODEL_KEY, first);
        }
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, [selectedModel]);

  const selectModel = useCallback((modelName: string) => {
    setSelectedModel(modelName);
    localStorage.setItem(STORED_MODEL_KEY, modelName);
  }, []);

  useEffect(() => {
    fetchModels();
  }, [fetchModels]);

  return {
    models,
    selectedModel,
    selectModel,
    isLoading,
    error,
    refreshModels: fetchModels,
  };
}
