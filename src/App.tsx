import { useState, useEffect, useCallback } from "react";
import { Key, Sliders, Layers, Play, Copy, Check, Plus, Trash2, Zap, Loader, Wand2, Download, Sparkles } from "lucide-react";
import "./App.css";
import * as api from "./lib/api";
import type { ProviderKey, ModelConfig, RouteRule, RouteCondition, Scene, UnifiedKey, Preset, UserNeeds } from "./lib/api";

const PROVIDERS = [
  { id: "openai", name: "OpenAI", models: ["gpt-4o", "gpt-4.1", "o3", "o4-mini"] },
  { id: "anthropic", name: "Anthropic Claude", models: ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4.5"] },
  { id: "deepseek", name: "DeepSeek", models: ["deepseek-chat", "deepseek-reasoner"] },
  { id: "qwen", name: "通义千问", models: ["qwen-max", "qwen-plus", "qwen-turbo"] },
  { id: "zhipu", name: "智谱 GLM", models: ["glm-4-plus", "glm-4-flash"] },
  { id: "moonshot", name: "Kimi (月之暗面)", models: ["moonshot-v1-8k", "moonshot-v1-32k"] },
  { id: "baichuan", name: "百川", models: ["baichuan4", "baichuan3-turbo"] },
  { id: "gemini", name: "Google Gemini", models: ["gemini-2.5-pro", "gemini-2.5-flash"] },
  { id: "custom", name: "自定义 OpenAI 兼容", models: [] },
];

export default function App() {
  const [tab, setTab] = useState<"keys" | "tuning" | "scenes" | "unikeys" | "wizard">("wizard");

  return (
    <div className="app">
      <Sidebar tab={tab} setTab={setTab} />
      <main className="main">
        {tab === "keys" && <KeysPage />}
        {tab === "tuning" && <TuningPage />}
        {tab === "scenes" && <ScenesPage />}
        {tab === "unikeys" && <UniKeysPage />}
        {tab === "wizard" && <WizardPage />}
      </main>
    </div>
  );
}

// -- Sidebar ---------------------------------------------------------------

function Sidebar({ tab, setTab }: { tab: string; setTab: (t: any) => void }) {
  const items = [
    { id: "keys", icon: Key, label: "密钥管理" },
    { id: "tuning", icon: Sliders, label: "模型微调" },
    { id: "scenes", icon: Layers, label: "场景组合" },
    { id: "unikeys", icon: Play, label: "统一 Key" },
    { id: "wizard", icon: Wand2, label: "智能推荐" },
  ];

  return (
    <nav className="sidebar">
      <div className="sidebar-logo">
        <Zap size={28} />
        <span>UniKey</span>
      </div>
      <div className="sidebar-nav">
        {items.map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            className={`sidebar-btn ${tab === id ? "active" : ""}`}
            onClick={() => setTab(id)}
          >
            <Icon size={20} />
            <span>{label}</span>
          </button>
        ))}
      </div>
    </nav>
  );
}

// -- Page: 密钥管理 ---------------------------------------------------------

function KeysPage() {
  const [keys, setKeys] = useState<ProviderKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [error, setError] = useState("");

  const loadKeys = useCallback(async () => {
    try {
      setLoading(true);
      const data = await api.listProviderKeys();
      setKeys(data);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadKeys(); }, [loadKeys]);

  const handleDelete = async (id: string) => {
    try {
      await api.deleteProviderKey(id);
      setKeys(keys.filter((k) => k.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="page">
      <header className="page-header">
        <h2>🔑 密钥管理</h2>
        <p>添加你的 AI 服务商 API Key，所有密钥本地加密存储</p>
        <button className="btn-primary" onClick={() => setShowAdd(true)}>
          <Plus size={16} /> 添加 Key
        </button>
      </header>

      {error && <div className="error-banner">{error}</div>}

      {showAdd && <AddKeyForm onAdded={() => { setShowAdd(false); loadKeys(); }} onClose={() => setShowAdd(false)} />}

      {loading ? (
        <div className="loading"><Loader size={24} className="spin" /> 加载中...</div>
      ) : (
        <div className="card-grid">
          {keys.map((k) => (
            <div className="card" key={k.id}>
              <div className="card-header">
                <span className="badge">{k.provider}</span>
                <button className="btn-icon" onClick={() => handleDelete(k.id)}>
                  <Trash2 size={14} />
                </button>
              </div>
              <h4>{k.name}</h4>
              <code className="key-preview">••••{k.encryptedKey.slice(-8)}</code>
            </div>
          ))}
          {keys.length === 0 && !loading && <EmptyState text="还没有添加任何 Key，点击上方按钮添加" />}
        </div>
      )}
    </div>
  );
}

function AddKeyForm({ onAdded, onClose }: { onAdded: () => void; onClose: () => void }) {
  const [name, setName] = useState("");
  const [provider, setProvider] = useState("openai");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleAdd = async () => {
    if (!name || !apiKey) return;
    try {
      setSaving(true);
      setError("");
      await api.addProviderKey(name, provider, apiKey, baseUrl || undefined);
      onAdded();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="form-card">
      <h3>添加 API Key</h3>
      {error && <div className="error-text">{error}</div>}
      <input placeholder="名称，如：我的 DeepSeek" value={name} onChange={(e) => setName(e.target.value)} />
      <select value={provider} onChange={(e) => setProvider(e.target.value)}>
        {PROVIDERS.map((p) => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
      </select>
      <input placeholder="API Key" type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
      <input placeholder="自定义 Base URL（可选）" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      <div className="form-actions">
        <button className="btn-primary" onClick={handleAdd} disabled={saving}>
          {saving ? "保存中..." : "添加"}
        </button>
        <button className="btn-ghost" onClick={onClose}>取消</button>
      </div>
    </div>
  );
}

// -- Page: 模型微调 ---------------------------------------------------------

function TuningPage() {
  const [configs, setConfigs] = useState<ModelConfig[]>([]);
  const [keys, setKeys] = useState<ProviderKey[]>([]);
  const [selected, setSelected] = useState<ModelConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [c, k] = await Promise.all([api.listModelConfigs(), api.listProviderKeys()]);
      setConfigs(c);
      setKeys(k);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const handleSave = async () => {
    if (!selected) return;
    try {
      await api.saveModelConfig(selected);
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.deleteModelConfig(id);
      setSelected(null);
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const newConfig = async () => {
    const c: ModelConfig = {
      id: crypto.randomUUID(),
      providerKeyId: keys[0]?.id || "",
      name: "新配置",
      model: "deepseek-chat",
      temperature: 1.0,
      topP: 1.0,
      maxTokens: 4096,
      frequencyPenalty: 0,
      presencePenalty: 0,
      createdAt: Math.floor(Date.now() / 1000),
    };
    try {
      await api.saveModelConfig(c);
      await loadData();
      setSelected(c);
    } catch (e) {
      setError(String(e));
    }
  };

  const update = (patch: Partial<ModelConfig>) => {
    if (!selected) return;
    setSelected({ ...selected, ...patch });
  };

  return (
    <div className="page">
      <header className="page-header">
        <h2>🎛️ 模型微调</h2>
        <p>对每个模型的参数进行精细调整，同一个 Key 可创建多个配置</p>
        <div className="header-actions">
          <button className="btn-primary" onClick={newConfig}><Plus size={16} /> 新建配置</button>
          {selected && <button className="btn-primary" onClick={handleSave} style={{ marginLeft: 8 }}>保存</button>}
        </div>
      </header>

      {error && <div className="error-banner">{error}</div>}

      {loading ? (
        <div className="loading"><Loader size={24} className="spin" /> 加载中...</div>
      ) : (
        <div className="tuning-layout">
          <div className="config-list">
            {configs.map((c) => (
              <button
                key={c.id}
                className={`config-item ${selected?.id === c.id ? "active" : ""}`}
                onClick={() => setSelected(c)}
              >
                <strong>{c.name}</strong>
                <span className="muted">{c.model}</span>
              </button>
            ))}
            {configs.length === 0 && <EmptyState text="还没有配置" />}
          </div>

          {selected && (
            <div className="tuning-panel">
              <label>配置名称</label>
              <input value={selected.name} onChange={(e) => update({ name: e.target.value })} />

              <label>关联 Key</label>
              <select
                value={selected.providerKeyId}
                onChange={(e) => update({ providerKeyId: e.target.value })}
              >
                <option value="">-- 选择 Key --</option>
                {keys.map((k) => (
                  <option key={k.id} value={k.id}>{k.name} ({k.provider})</option>
                ))}
              </select>

              <label>模型</label>
              <select value={selected.model} onChange={(e) => update({ model: e.target.value })}>
                {PROVIDERS.flatMap((p) =>
                  p.models.map((m) => (
                    <option key={`${p.id}:${m}`} value={m}>{p.name} — {m}</option>
                  ))
                )}
              </select>

              <ParamSlider label="Temperature" value={selected.temperature} min={0} max={2} step={0.05}
                onChange={(v) => update({ temperature: v })} />
              <ParamSlider label="Top P" value={selected.topP} min={0} max={1} step={0.05}
                onChange={(v) => update({ topP: v })} />
              <ParamSlider label="Max Tokens" value={selected.maxTokens} min={256} max={131072} step={256}
                onChange={(v) => update({ maxTokens: v })} />
              <ParamSlider label="Frequency Penalty" value={selected.frequencyPenalty} min={-2} max={2} step={0.1}
                onChange={(v) => update({ frequencyPenalty: v })} />
              <ParamSlider label="Presence Penalty" value={selected.presencePenalty} min={-2} max={2} step={0.1}
                onChange={(v) => update({ presencePenalty: v })} />

              <label>System Prompt</label>
              <textarea
                rows={4}
                value={selected.systemPrompt || ""}
                onChange={(e) => update({ systemPrompt: e.target.value })}
                placeholder="可选：设定模型行为..."
              />

              <button className="btn-danger" onClick={() => handleDelete(selected.id)}>
                <Trash2 size={14} /> 删除此配置
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ParamSlider({ label, value, min, max, step, onChange }: {
  label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void;
}) {
  return (
    <div className="param-slider">
      <div className="param-label">
        <span>{label}</span>
        <code>{value}</code>
      </div>
      <input type="range" min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))} />
    </div>
  );
}

// -- Page: 场景组合 ---------------------------------------------------------

function ScenesPage() {
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [configs, setConfigs] = useState<ModelConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [editing, setEditing] = useState<Scene | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [s, c] = await Promise.all([api.listScenes(), api.listModelConfigs()]);
      setScenes(s);
      setConfigs(c);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const createScene = async () => {
    const scene: Scene = {
      id: crypto.randomUUID(),
      name: "新场景",
      description: "",
      rules: [{ id: crypto.randomUUID(), condition: { default: true }, modelConfigId: configs[0]?.id || "", priority: 99 }],
      createdAt: Math.floor(Date.now() / 1000),
      updatedAt: Math.floor(Date.now() / 1000),
    };
    try {
      await api.saveScene(scene);
      await loadData();
      setEditing(scene);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.deleteScene(id);
      if (editing?.id === id) setEditing(null);
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const saveEditing = async () => {
    if (!editing) return;
    try {
      await api.saveScene({ ...editing, updatedAt: Math.floor(Date.now() / 1000) });
      await loadData();
      setEditing(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const conditionLabel = (r: RouteRule) => {
    if ("default" in r.condition && r.condition.default) return "默认路由";
    if ("always" in r.condition && r.condition.always) return "始终使用";
    if ("keyword" in r.condition) return `关键词: ${r.condition.keyword.keywords.join(", ")}`;
    if ("modelName" in r.condition) return `模型: ${r.condition.modelName.patterns.join(", ")}`;
    if ("capability" in r.condition) return `能力: ${r.condition.capability.capabilities.join(", ")}`;
    return "未设置";
  };

  return (
    <div className="page">
      <header className="page-header">
        <h2>🔗 场景组合</h2>
        <p>把多个微调好的模型配置组合成一个场景，设置路由规则</p>
        <button className="btn-primary" onClick={createScene}><Plus size={16} /> 创建场景</button>
      </header>

      {error && <div className="error-banner">{error}</div>}

      {loading ? (
        <div className="loading"><Loader size={24} className="spin" /> 加载中...</div>
      ) : (
        <div className="scenes-layout">
          <div className="card-grid scenes-grid">
            {scenes.map((s) => (
              <div className={`card scene-card ${editing?.id === s.id ? "selected" : ""}`} key={s.id} onClick={() => setEditing(s)}>
                <h4>{s.name}</h4>
                <p className="muted">{s.description || "暂无描述"}</p>
                <div className="rules-preview">
                  {s.rules.map((r) => (
                    <span key={r.id} className="badge badge-rule">{conditionLabel(r)}</span>
                  ))}
                </div>
                <div className="card-footer">
                  <span className="muted">{s.rules.length} 条规则</span>
                  <button className="btn-icon" onClick={(e) => { e.stopPropagation(); handleDelete(s.id); }}>
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
            {scenes.length === 0 && <EmptyState text="还没有创建场景" />}
          </div>

          {editing && (
            <div className="scene-editor">
              <h3>编辑场景</h3>
              <label>场景名称</label>
              <input
                value={editing.name}
                onChange={(e) => setEditing({ ...editing, name: e.target.value })}
              />
              <label>描述</label>
              <input
                value={editing.description}
                onChange={(e) => setEditing({ ...editing, description: e.target.value })}
                placeholder="如：编程全能、内容创作..."
              />

              <label>路由规则</label>
              {editing.rules.map((rule, i) => (
                <div key={rule.id} className="rule-editor">
                  <select
                    value={JSON.stringify(rule.condition)}
                    onChange={(e) => {
                      const cond: RouteCondition = JSON.parse(e.target.value);
                      const rules = [...editing.rules];
                      rules[i] = { ...rules[i], condition: cond };
                      setEditing({ ...editing, rules });
                    }}
                  >
                    <option value={JSON.stringify({ default: true })}>默认路由（兜底）</option>
                    <option value={JSON.stringify({ always: true })}>始终使用</option>
                    <option value={JSON.stringify({ keyword: { keywords: ["代码", "编程", "code"] } })}>关键词匹配</option>
                    <option value={JSON.stringify({ modelName: { patterns: ["gpt", "claude"] } })}>模型名匹配</option>
                    <option value={JSON.stringify({ capability: { capabilities: ["codeGeneration"] } })}>能力：代码生成</option>
                    <option value={JSON.stringify({ capability: { capabilities: ["imageUnderstanding"] } })}>能力：图片理解</option>
                    <option value={JSON.stringify({ capability: { capabilities: ["translation"] } })}>能力：翻译</option>
                  </select>
                  <select
                    value={rule.modelConfigId}
                    onChange={(e) => {
                      const rules = [...editing.rules];
                      rules[i] = { ...rules[i], modelConfigId: e.target.value };
                      setEditing({ ...editing, rules });
                    }}
                  >
                    <option value="">-- 选择配置 --</option>
                    {configs.map((c) => (
                      <option key={c.id} value={c.id}>{c.name} ({c.model})</option>
                    ))}
                  </select>
                  <button className="btn-icon" onClick={() => {
                    setEditing({ ...editing, rules: editing.rules.filter((_, j) => j !== i) });
                  }}>
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}

              <button className="btn-ghost" onClick={() => {
                setEditing({
                  ...editing,
                  rules: [...editing.rules, {
                    id: crypto.randomUUID(),
                    condition: { default: true },
                    modelConfigId: "",
                    priority: editing.rules.length,
                  }],
                });
              }}>
                <Plus size={14} /> 添加规则
              </button>

              <div className="form-actions" style={{ marginTop: 12 }}>
                <button className="btn-primary" onClick={saveEditing}>保存场景</button>
                <button className="btn-ghost" onClick={() => setEditing(null)}>取消</button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// -- Page: 统一 Key ---------------------------------------------------------

function UniKeysPage() {
  const [unikeys, setUnikeys] = useState<UnifiedKey[]>([]);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState<string | null>(null);
  const [proxyRunning, setProxyRunning] = useState(false);
  const [error, setError] = useState("");

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [u, s] = await Promise.all([api.listUnifiedKeys(), api.listScenes()]);
      setUnikeys(u);
      setScenes(s);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const handleGenerate = async () => {
    if (scenes.length === 0) return;
    try {
      const scene = scenes[scenes.length - 1];
      const key = await api.generateUnifiedKey(scene.id, scene.name);
      setUnikeys([key, ...unikeys]);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.deleteUnifiedKey(id);
      setUnikeys(unikeys.filter((k) => k.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  const toggleProxy = async () => {
    try {
      if (proxyRunning) {
        await api.stopProxy();
        setProxyRunning(false);
      } else {
        await api.startProxy(7890);
        setProxyRunning(true);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const copyKey = async (kv: string) => {
    await navigator.clipboard.writeText(kv);
    setCopied(kv);
    setTimeout(() => setCopied(null), 2000);
  };

  return (
    <div className="page">
      <header className="page-header">
        <h2>🚀 统一 Key</h2>
        <p>每个场景生成一个统一 Key，复制到任何软件即可使用</p>
        <div className="header-actions">
          <button className="btn-primary" onClick={handleGenerate} disabled={scenes.length === 0}>
            <Plus size={16} /> 生成统一 Key
          </button>
        </div>
      </header>

      {error && <div className="error-banner">{error}</div>}

      <div className="proxy-bar">
        <div className={`proxy-status ${proxyRunning ? "running" : "stopped"}`}>
          <span className="dot" />
          {proxyRunning ? "代理运行中 — localhost:7890" : "代理未启动"}
        </div>
        <button className="btn-primary" onClick={toggleProxy}>
          <Play size={14} /> {proxyRunning ? "停止代理" : "启动代理"}
        </button>
      </div>

      {loading ? (
        <div className="loading"><Loader size={24} className="spin" /> 加载中...</div>
      ) : (
        <div className="card-grid">
          {unikeys.map((uk) => (
            <div className="card unikey-card" key={uk.id}>
              <div className="card-header">
                <h4>{uk.name}</h4>
                <button className="btn-icon" onClick={() => handleDelete(uk.id)}>
                  <Trash2 size={14} />
                </button>
              </div>
              <div className="key-display">
                <code>{uk.keyValue}</code>
                <button className="btn-icon" onClick={() => copyKey(uk.keyValue)}>
                  {copied === uk.keyValue ? <Check size={16} /> : <Copy size={16} />}
                </button>
              </div>
              <p className="muted usage-info">
                Endpoint: <code>http://localhost:7890/v1</code>
                {uk.usageCount > 0 && <> · 已使用 {uk.usageCount} 次</>}
              </p>
            </div>
          ))}
          {unikeys.length === 0 && (
            <div className="empty-guide">
              <p>← 先去<b>场景组合</b>创建场景，然后点击上方按钮生成统一 Key</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// -- Page: 智能推荐 + 预设 ---------------------------------------------------------

function WizardPage() {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [applying, setApplying] = useState<string | null>(null);
  const [success, setSuccess] = useState("");

  // Recommendation form state
  const [scenarios, setScenarios] = useState<string[]>(["编程"]);
  const [budget, setBudget] = useState<string>("balanced");
  const [quality, setQuality] = useState<string>("good");
  const [recommendations, setRecommendations] = useState<api.Recommendation[]>([]);

  const loadPresets = useCallback(async () => {
    try {
      setLoading(true);
      const p = await api.getPresets();
      setPresets(p);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadPresets(); }, [loadPresets]);

  const handleRecommend = async () => {
    try {
      const needs: UserNeeds = {
        scenarios,
        budget: budget as UserNeeds["budget"],
        quality: quality as UserNeeds["quality"],
        language: "chinese",
      };
      const results = await api.getRecommendations(needs);
      setRecommendations(results);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  };

  const applyPreset = async (preset: Preset) => {
    try {
      setApplying(preset.id);
      setError("");
      setSuccess("");

      // 1. Save model configs from the preset slots
      const configIds: string[] = [];

      for (const slot of preset.slots) {
        const config: ModelConfig = {
          id: crypto.randomUUID(),
          providerKeyId: "", // user needs to assign keys later
          name: `${preset.name} - ${slot.role}`,
          model: slot.model,
          temperature: slot.temperature,
          topP: 1.0,
          maxTokens: slot.maxTokens,
          frequencyPenalty: 0,
          presencePenalty: 0,
          systemPrompt: "",
          createdAt: Math.floor(Date.now() / 1000),
        };
        await api.saveModelConfig(config);
        configIds.push(config.id);
      }

      // 2. Create a scene with routing rules
      const rules: RouteRule[] = configIds.map((cid, i) => ({
        id: crypto.randomUUID(),
        condition: i === 0 ? { default: true } as RouteCondition : { keyword: { keywords: [preset.tags[i - 1] || preset.tags[0]] } } as RouteCondition,
        modelConfigId: cid,
        priority: i + 1,
      }));

      const scene: Scene = {
        id: crypto.randomUUID(),
        name: preset.name,
        description: preset.description,
        rules,
        createdAt: Math.floor(Date.now() / 1000),
        updatedAt: Math.floor(Date.now() / 1000),
      };
      await api.saveScene(scene);
      setSuccess(`"${preset.name}" 预设已应用！去「场景组合」和「模型微调」绑定你的 Key 即可使用。`);
    } catch (e) {
      setError(String(e));
    } finally {
      setApplying(null);
    }
  };

  const toggleScenario = (s: string) => {
    setScenarios(prev => prev.includes(s) ? prev.filter(x => x !== s) : [...prev, s]);
  };

  return (
    <div className="page">
      <header className="page-header">
        <h2><Sparkles size={24} style={{ marginRight: 8 }} /> 智能推荐 & 预设方案</h2>
        <p>不知道用什么模型？告诉我们你的需求，自动推荐最佳组合。或者从预设方案一键套用。</p>
      </header>

      {error && <div className="error-banner">{error}</div>}
      {success && <div className="success-banner">{success}</div>}

      {/* Smart Recommendation */}
      <section className="wizard-section">
        <h3>🧠 智能推荐</h3>
        <div className="wizard-form">
          <div className="wizard-field">
            <label>使用场景（可多选）</label>
            <div className="chip-group">
              {["编程","写作","翻译","数据分析","多模态","日常对话"].map(s => (
                <button key={s} className={`chip ${scenarios.includes(s) ? "active" : ""}`}
                  onClick={() => toggleScenario(s)}>{s}</button>
              ))}
            </div>
          </div>
          <div className="wizard-row">
            <div className="wizard-field">
              <label>预算</label>
              <select value={budget} onChange={e => setBudget(e.target.value)}>
                <option value="free">尽量免费</option>
                <option value="balanced">性价比优先</option>
                <option value="premium">质量优先</option>
              </select>
            </div>
            <div className="wizard-field">
              <label>质量要求</label>
              <select value={quality} onChange={e => setQuality(e.target.value)}>
                <option value="sufficient">够用就行</option>
                <option value="good">中等偏上</option>
                <option value="best">追求最好</option>
              </select>
            </div>
          </div>
          <button className="btn-primary" onClick={handleRecommend}>
            <Wand2 size={14} /> 生成推荐方案
          </button>
        </div>

        {recommendations.length > 0 && (
          <div className="recommend-results">
            {recommendations.map((r, i) => (
              <div key={i} className="recommend-card">
                <h4>{r.name}</h4>
                <p className="muted">{r.description}</p>
                <span className="badge">💰 {r.estimatedMonthlyCost}</span>
                <div className="slot-list">
                  {r.slots.map((s, j) => (
                    <div key={j} className="slot-item">
                      <strong>{s.role}</strong> → {s.provider} / {s.model}
                      <br/><span className="muted">T={s.temperature} | max_tokens={s.maxTokens}</span>
                      <br/><span className="muted">💡 {s.reason}</span>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Presets */}
      <section className="wizard-section">
        <h3>📦 预设方案库</h3>
        <p className="muted">一键套用，无需手动配置</p>

        {loading ? (
          <div className="loading"><Loader size={24} className="spin" /> 加载中...</div>
        ) : (
          <div className="preset-grid">
            {presets.map(p => (
              <div key={p.id} className="preset-card">
                <div className="preset-header">
                  <span className="preset-icon">{p.icon}</span>
                  <h4>{p.name}</h4>
                </div>
                <p className="muted">{p.description}</p>
                <div className="tags">
                  {p.tags.map(t => <span key={t} className="badge badge-rule">{t}</span>)}
                </div>
                <div className="slot-mini">
                  {p.slots.map((s, i) => (
                    <div key={i} className="slot-line">
                      <span>{s.role}</span> → <code>{s.provider}/{s.model}</code>
                    </div>
                  ))}
                </div>
                <button className="btn-primary btn-sm"
                  onClick={() => applyPreset(p)}
                  disabled={applying === p.id}>
                  <Download size={14} /> {applying === p.id ? "应用..." : "一键应用"}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

// -- Shared Components ------------------------------------------------------

function EmptyState({ text }: { text: string }) {
  return <div className="empty-state">{text}</div>;
}
