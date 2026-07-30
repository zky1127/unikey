// Tauri IPC API wrappers — all backend communication

import { invoke } from "@tauri-apps/api/core";

// -- Types (matches Rust models with camelCase) -----------------------------

export type ProviderKey = {
  id: string;
  name: string;
  provider: string;
  encryptedKey: string;
  baseUrl?: string;
  createdAt: number;
};

export type ModelConfig = {
  id: string;
  providerKeyId: string;
  name: string;
  model: string;
  temperature: number;
  topP: number;
  maxTokens: number;
  frequencyPenalty: number;
  presencePenalty: number;
  systemPrompt?: string;
  extraParams?: unknown;
  createdAt: number;
};

export type RouteRule = {
  id: string;
  condition: RouteCondition;
  modelConfigId: string;
  priority: number;
};

export type RouteCondition =
  | { default: true }
  | { keyword: { keywords: string[] } }
  | { capability: { capabilities: string[] } }
  | { modelName: { patterns: string[] } }
  | { always: true };

export type Scene = {
  id: string;
  name: string;
  description: string;
  rules: RouteRule[];
  createdAt: number;
  updatedAt: number;
};

export type UnifiedKey = {
  id: string;
  keyValue: string;
  sceneId: string;
  name: string;
  createdAt: number;
  lastUsedAt?: number;
  usageCount: number;
};

// -- Provider Keys API -------------------------------------------------------

export async function listProviderKeys(): Promise<ProviderKey[]> {
  return invoke("list_provider_keys");
}

export async function addProviderKey(
  name: string,
  provider: string,
  apiKey: string,
  baseUrl?: string,
): Promise<ProviderKey> {
  return invoke("add_provider_key", { name, provider, apiKey, baseUrl });
}

export async function deleteProviderKey(id: string): Promise<void> {
  return invoke("delete_provider_key", { id });
}

export async function decryptKey(encrypted: string): Promise<string> {
  return invoke("decrypt_key", { encrypted });
}

// -- Model Configs API -------------------------------------------------------

export async function listModelConfigs(): Promise<ModelConfig[]> {
  return invoke("list_model_configs");
}

export async function saveModelConfig(config: ModelConfig): Promise<void> {
  return invoke("save_model_config", { config });
}

export async function deleteModelConfig(id: string): Promise<void> {
  return invoke("delete_model_config", { id });
}

// -- Scenes API --------------------------------------------------------------

export async function listScenes(): Promise<Scene[]> {
  return invoke("list_scenes");
}

export async function saveScene(scene: Scene): Promise<void> {
  return invoke("save_scene", { scene });
}

export async function deleteScene(id: string): Promise<void> {
  return invoke("delete_scene", { id });
}

// -- Unified Keys API --------------------------------------------------------

export async function listUnifiedKeys(): Promise<UnifiedKey[]> {
  return invoke("list_unified_keys");
}

export async function generateUnifiedKey(
  sceneId: string,
  name: string,
): Promise<UnifiedKey> {
  return invoke("generate_unified_key", { sceneId, name });
}

export async function deleteUnifiedKey(id: string): Promise<void> {
  return invoke("delete_unified_key", { id });
}

// -- Proxy Control API -------------------------------------------------------

export async function startProxy(port: number): Promise<void> {
  return invoke("start_proxy", { port });
}

export async function stopProxy(): Promise<void> {
  return invoke("stop_proxy");
}

// -- Recommendations & Presets -----------------------------------------------

export type UserNeeds = {
  scenarios: string[];
  budget: "free" | "balanced" | "premium";
  quality: "sufficient" | "good" | "best";
  language: "chinese" | "english" | "mixed";
};

export type RecommendSlot = {
  role: string;
  provider: string;
  model: string;
  temperature: number;
  maxTokens: number;
  reason: string;
};

export type Recommendation = {
  name: string;
  description: string;
  estimatedMonthlyCost: string;
  slots: RecommendSlot[];
};

export type Preset = {
  id: string;
  name: string;
  description: string;
  icon: string;
  slots: RecommendSlot[];
  tags: string[];
};

export async function getRecommendations(needs: UserNeeds): Promise<Recommendation[]> {
  return invoke("get_recommendations", { needs });
}

export async function getPresets(): Promise<Preset[]> {
  return invoke("get_presets");
}
