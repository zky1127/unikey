import { useState } from "react";
import { Key, Sliders, Layers, Play, Copy, Check, Plus, Trash2, Zap } from "lucide-react";
import "./App.css";

// -- Types ----------------------------------------------------------------

type ProviderKey = {
  id: string;
  name: string;
  provider: string;
  encrypted_key: string;
  base_url?: string;
  created_at: number;
};

type ModelConfig = {
  id: string;
  provider_key_id: string;
  name: string;
  model: string;
  temperature: number;
  top_p: number;
  max_tokens: number;
  frequency_penalty: number;
  presence_penalty: number;
  system_prompt?: string;
};

type RouteRule = {
  id: string;
  condition: { default?: true; keyword?: { keywords: string[] }; always?: true };
  model_config_id: string;
  priority: number;
};

type Scene = {
  id: string;
  name: string;
  description: string;
  rules: RouteRule[];
  created_at: number;
  updated_at: number;
};

type UnifiedKey = {
  id: string;
  key_value: string;
  scene_id: string;
  name: string;
  created_at: number;
};

// -- Mock Data (TODO: replace with Tauri invoke) ---------------------------

const MOCK_KEYS: ProviderKey[] = [];
const MOCK_CONFIGS: ModelConfig[] = [];
const MOCK_SCENES: Scene[] = [];
const MOCK_UNIKEYS: UnifiedKey[] = [];

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

// -- App -------------------------------------------------------------------

export default function App() {
  const [tab, setTab] = useState<"keys" | "tuning" | "scenes" | "unikeys">("keys");

  return (
    <div className="app">
      <Sidebar tab={tab} setTab={setTab} />
      <main className="main">
        {tab === "keys" && <KeysPage />}
        {tab === "tuning" && <TuningPage />}
        {tab === "scenes" && <ScenesPage />}
        {tab === "unikeys" && <UniKeysPage />}
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
  const [keys, setKeys] = useState<ProviderKey[]>(MOCK_KEYS);
  const [showAdd, setShowAdd] = useState(false);

  return (
    <div className="page">
      <header className="page-header">
        <h2>🔑 密钥管理</h2>
        <p>添加你的 AI 服务商 API Key，所有密钥本地加密存储</p>
        <button className="btn-primary" onClick={() => setShowAdd(true)}>
          <Plus size={16} /> 添加 Key
        </button>
      </header>

      {showAdd && <AddKeyForm keys={keys} setKeys={setKeys} onClose={() => setShowAdd(false)} />}

      <div className="card-grid">
        {keys.map((k) => (
          <div className="card" key={k.id}>
            <div className="card-header">
              <span className="badge">{k.provider}</span>
              <button className="btn-icon" onClick={() => setKeys(keys.filter((x) => x.id !== k.id))}>
                <Trash2 size={14} />
              </button>
            </div>
            <h4>{k.name}</h4>
            <code className="key-preview">••••{k.encrypted_key.slice(-8)}</code>
          </div>
        ))}
        {keys.length === 0 && <EmptyState text="还没有添加任何 Key" />}
      </div>
    </div>
  );
}

function AddKeyForm({ keys, setKeys, onClose }: { keys: ProviderKey[]; setKeys: (k: ProviderKey[]) => void; onClose: () => void }) {
  const [name, setName] = useState("");
  const [provider, setProvider] = useState("openai");
  const [apiKey, setApiKey] = useState("");

  const handleAdd = () => {
    if (!name || !apiKey) return;
    const newKey: ProviderKey = {
      id: crypto.randomUUID(),
      name,
      provider,
      encrypted_key: apiKey, // 实际由 Rust 加密
      created_at: Date.now(),
    };
    setKeys([...keys, newKey]);
    onClose();
  };

  return (
    <div className="form-card">
      <h3>添加 API Key</h3>
      <input placeholder="名称，如：我的 DeepSeek" value={name} onChange={(e) => setName(e.target.value)} />
      <select value={provider} onChange={(e) => setProvider(e.target.value)}>
        {PROVIDERS.map((p) => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
      </select>
      <input placeholder="API Key" type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
      <div className="form-actions">
        <button className="btn-primary" onClick={handleAdd}>添加</button>
        <button className="btn-ghost" onClick={onClose}>取消</button>
      </div>
    </div>
  );
}

// -- Page: 模型微调 ---------------------------------------------------------

function TuningPage() {
  const [configs, setConfigs] = useState<ModelConfig[]>(MOCK_CONFIGS);
  const [selected, setSelected] = useState<ModelConfig | null>(null);

  return (
    <div className="page">
      <header className="page-header">
        <h2>🎛️ 模型微调</h2>
        <p>对每个模型的参数进行精细调整，同一个 Key 可创建多个配置</p>
        <button
          className="btn-primary"
          onClick={() => {
            const c: ModelConfig = {
              id: crypto.randomUUID(),
              provider_key_id: "",
              name: "新配置",
              model: "deepseek-chat",
              temperature: 1.0,
              top_p: 1.0,
              max_tokens: 4096,
              frequency_penalty: 0,
              presence_penalty: 0,
            };
            setConfigs([...configs, c]);
            setSelected(c);
          }}
        >
          <Plus size={16} /> 新建配置
        </button>
      </header>

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
          {configs.length === 0 && <EmptyState text="还没有模型配置" />}
        </div>

        {selected && (
          <div className="tuning-panel">
            <input
              value={selected.name}
              onChange={(e) => {
                const updated = { ...selected, name: e.target.value };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }}
              placeholder="配置名称"
            />
            <select
              value={selected.model}
              onChange={(e) => {
                const updated = { ...selected, model: e.target.value };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }}
            >
              {PROVIDERS.flatMap((p) =>
                p.models.map((m) => (
                  <option key={`${p.id}:${m}`} value={m}>{p.name} — {m}</option>
                ))
              )}
            </select>

            <ParamSlider label="Temperature" value={selected.temperature} min={0} max={2} step={0.05}
              onChange={(v) => {
                const updated = { ...selected, temperature: v };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }} />
            <ParamSlider label="Top P" value={selected.top_p} min={0} max={1} step={0.05}
              onChange={(v) => {
                const updated = { ...selected, top_p: v };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }} />
            <ParamSlider label="Max Tokens" value={selected.max_tokens} min={256} max={131072} step={256}
              onChange={(v) => {
                const updated = { ...selected, max_tokens: v };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }} />

            <label>System Prompt</label>
            <textarea
              rows={4}
              value={selected.system_prompt || ""}
              onChange={(e) => {
                const updated = { ...selected, system_prompt: e.target.value };
                setSelected(updated);
                setConfigs(configs.map((c) => (c.id === selected.id ? updated : c)));
              }}
              placeholder="可选：设定模型行为..."
            />

            <button
              className="btn-danger"
              onClick={() => {
                setConfigs(configs.filter((c) => c.id !== selected.id));
                setSelected(null);
              }}
            >
              <Trash2 size={14} /> 删除此配置
            </button>
          </div>
        )}
      </div>
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
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
      />
    </div>
  );
}

// -- Page: 场景组合 ---------------------------------------------------------

function ScenesPage() {
  const [scenes, setScenes] = useState<Scene[]>(MOCK_SCENES);

  return (
    <div className="page">
      <header className="page-header">
        <h2>🔗 场景组合</h2>
        <p>把多个微调好的模型配置组合成一个场景，设置路由规则</p>
        <button
          className="btn-primary"
          onClick={() => {
            const scene: Scene = {
              id: crypto.randomUUID(),
              name: "新场景",
              description: "",
              rules: [
                { id: crypto.randomUUID(), condition: { default: true }, model_config_id: "", priority: 99 },
              ],
              created_at: Date.now(),
              updated_at: Date.now(),
            };
            setScenes([...scenes, scene]);
          }}
        >
          <Plus size={16} /> 创建场景
        </button>
      </header>

      <div className="card-grid">
        {scenes.map((s) => (
          <div className="card scene-card" key={s.id}>
            <h4>{s.name}</h4>
            <p className="muted">{s.description || "暂无描述"}</p>
            <div className="rules-preview">
              {s.rules.map((r) => (
                <span key={r.id} className="badge badge-rule">
                  {r.condition.default ? "默认" : r.condition.always ? "始终" : "关键词"}
                </span>
              ))}
            </div>
            <div className="card-footer">
              <span className="muted">{s.rules.length} 条路由规则</span>
              <button className="btn-icon" onClick={() => setScenes(scenes.filter((x) => x.id !== s.id))}>
                <Trash2 size={14} />
              </button>
            </div>
          </div>
        ))}
        {scenes.length === 0 && <EmptyState text="还没有创建场景" />}
      </div>
    </div>
  );
}

// -- Page: 统一 Key ---------------------------------------------------------

function UniKeysPage() {
  const [unikeys, setUnikeys] = useState<UnifiedKey[]>(MOCK_UNIKEYS);
  const [copied, setCopied] = useState<string | null>(null);
  const [proxyRunning, setProxyRunning] = useState(false);

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
      </header>

      <div className="proxy-bar">
        <div className={`proxy-status ${proxyRunning ? "running" : "stopped"}`}>
          <span className="dot" />
          {proxyRunning ? "代理运行中 — localhost:7890" : "代理未启动"}
        </div>
        <button className="btn-primary" onClick={() => setProxyRunning(!proxyRunning)}>
          <Play size={14} /> {proxyRunning ? "停止代理" : "启动代理"}
        </button>
      </div>

      <div className="card-grid">
        {unikeys.map((uk) => (
          <div className="card unikey-card" key={uk.id}>
            <h4>{uk.name}</h4>
            <div className="key-display">
              <code>{uk.key_value}</code>
              <button className="btn-icon" onClick={() => copyKey(uk.key_value)}>
                {copied === uk.key_value ? <Check size={16} /> : <Copy size={16} />}
              </button>
            </div>
            <p className="muted usage-info">
              Endpoint: <code>http://localhost:7890/v1</code>
            </p>
          </div>
        ))}
        {unikeys.length === 0 && (
          <div className="empty-guide">
            <p>← 先去<b>场景组合</b>创建场景，再回来生成统一 Key</p>
            <br/>
            <button className="btn-primary" onClick={() => {
              const uk: UnifiedKey = {
                id: crypto.randomUUID(),
                key_value: `sk-unikey-${crypto.randomUUID().replace(/-/g, "").slice(0, 32)}`,
                scene_id: "demo-scene",
                name: "示例 Key",
                created_at: Date.now(),
              };
              setUnikeys([uk]);
            }}>
              <Plus size={14} /> 生成示例 Key
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// -- Shared Components ------------------------------------------------------

function EmptyState({ text }: { text: string }) {
  return <div className="empty-state">{text}</div>;
}
