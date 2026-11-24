import type { ProviderConfig, ProviderCredentialStatus, ProviderKind, Settings, ThinkingLevel } from './types';

export const DEFAULT_PROVIDER_ID = 'claude-cli';

export const DEFAULT_MODELS = { readModel: 'sonnet', chatModel: 'sonnet', drawModel: 'opus' };

export const DEFAULT_PROVIDERS: ProviderConfig[] = [
  {
    id: 'claude-cli',
    name: 'Claude CLI',
    kind: 'claude-cli',
    baseUrl: null,
    command: 'claude',
    args: [],
    envVar: null,
    models: ['haiku', 'sonnet', 'opus', 'fable'],
    builtin: true,
  },
  {
    id: 'agy-acp',
    name: 'Antigravity CLI',
    kind: 'agy-acp',
    baseUrl: null,
    command: 'agy',
    args: ['acp'],
    envVar: null,
    models: [
      'gemini-4-neon',
      'gemini-4-carbon',
      'gemini-4-argon',
      'gemini-4-hydrogen',
      'gemini-3.8-flash',
    ],
    builtin: true,
  },
  {
    id: 'codex-cli',
    name: 'Codex CLI',
    kind: 'codex-cli',
    baseUrl: null,
    command: 'codex',
    args: [],
    envVar: null,
    models: ['gpt-5.4', 'gpt-5.3-codex', 'o3', 'o4-mini'],
    builtin: true,
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    kind: 'openai',
    baseUrl: 'https://openrouter.ai/api/v1',
    command: null,
    args: [],
    envVar: 'OPENROUTER_API_KEY',
    models: [
      'anthropic/claude-sonnet-4',
      'anthropic/claude-opus-4',
      'openai/gpt-4.1',
      'google/gemini-2.5-pro',
      'deepseek/deepseek-chat-v3-0324',
    ],
    builtin: true,
  },
  {
    id: 'openai',
    name: 'OpenAI API',
    kind: 'openai',
    baseUrl: 'https://api.openai.com/v1',
    command: null,
    args: [],
    envVar: 'OPENAI_API_KEY',
    models: ['gpt-4.1', 'gpt-4.1-mini', 'gpt-5', 'o4-mini'],
    builtin: true,
  },
  {
    id: 'anthropic',
    name: 'Anthropic API',
    kind: 'anthropic',
    baseUrl: 'https://api.anthropic.com',
    command: null,
    args: [],
    envVar: 'ANTHROPIC_API_KEY',
    models: ['claude-sonnet-4-20250514', 'claude-opus-4-20250514', 'claude-haiku-4-5-20251001'],
    builtin: true,
  },
  {
    id: 'gemini',
    name: 'Google Gemini API',
    kind: 'gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    command: null,
    args: [],
    envVar: 'GEMINI_API_KEY',
    models: ['gemini-2.5-pro', 'gemini-2.5-flash'],
    builtin: true,
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    kind: 'openai',
    baseUrl: 'https://api.deepseek.com/v1',
    command: null,
    args: [],
    envVar: 'DEEPSEEK_API_KEY',
    models: ['deepseek-chat', 'deepseek-reasoner'],
    builtin: true,
  },
  {
    id: 'zlm',
    name: 'Z.AI / GLM',
    kind: 'openai',
    baseUrl: 'https://api.z.ai/api/paas/v4',
    command: null,
    args: [],
    envVar: 'ZAI_API_KEY',
    models: ['glm-4.5', 'glm-4.5-air', 'glm-4-plus'],
    builtin: true,
  },
  {
    id: 'mimo',
    name: 'Xiaomi MiMo',
    kind: 'openai',
    baseUrl: 'https://api.xiaomimimo.com/v1',
    command: null,
    args: [],
    envVar: 'MIMO_API_KEY',
    models: ['mimo-v2-flash', 'mimo-v2-pro'],
    builtin: true,
  },
  {
    id: 'opencode-go',
    name: 'OpenCode Go',
    kind: 'openai',
    baseUrl: 'https://opencode.ai/zen/go/v1',
    command: null,
    args: [],
    envVar: 'OPENCODE_API_KEY',
    models: ['glm-5', 'kimi-k2.5', 'minimax-m2.5', 'mimo-v2-pro'],
    builtin: true,
  },
];

export function isHttpKind(kind: ProviderKind): boolean {
  return kind === 'openai' || kind === 'anthropic' || kind === 'gemini';
}

export function mergeProviders(saved: ProviderConfig[] | undefined): ProviderConfig[] {
  const base = saved && saved.length > 0 ? saved.map((p) => ({ ...p, args: [...(p.args ?? [])], models: [...(p.models ?? [])] })) : [];
  for (const preset of DEFAULT_PROVIDERS) {
    const existing = base.find((p) => p.id === preset.id);
    if (existing) {
      if (existing.builtin) {
        existing.name = preset.name;
        existing.models = [...preset.models];
      }
    } else {
      base.push({ ...preset, args: [...preset.args], models: [...preset.models] });
    }
  }
  return base;
}

export const THINKING_LEVELS: { id: ThinkingLevel; name: string }[] = [
  { id: 'default', name: 'Auto' },
  { id: 'low', name: 'Low' },
  { id: 'medium', name: 'Medium' },
  { id: 'high', name: 'High' },
  { id: 'xhigh', name: 'Highest' },
];

export const JOBS = [
  { name: 'Reading', desc: 'Reads along and asks in the margin.', providerKey: 'readProvider', modelKey: 'readModel', thinkingKey: 'readThinking' },
  { name: 'Chat', desc: 'Answers in the side chat, and when you say its name.', providerKey: 'chatProvider', modelKey: 'chatModel', thinkingKey: 'chatThinking' },
  { name: 'Drawing', desc: 'Sketches beside your notes.', providerKey: 'drawProvider', modelKey: 'drawModel', thinkingKey: 'drawThinking' },
] as const;

export type Job = (typeof JOBS)[number];

/** How Duragon gets into a provider. `unknown` means the saved keys can't be read. */
export type Access =
  | { kind: 'local' }
  | { kind: 'saved'; masked: string }
  | { kind: 'system'; envVar: string }
  | { kind: 'missing' }
  | { kind: 'unknown' };

export interface ProviderView {
  provider: ProviderConfig;
  access: Access;
  jobs: Job[];
  /** Set up or in use. The others wait under Add. */
  listed: boolean;
}

export const isReady = ({ access }: ProviderView) => access.kind !== 'missing' && access.kind !== 'unknown';

/** The provider that does a job. The core falls back the same way when a saved one is gone. */
export const jobProvider = (views: ProviderView[], settings: Settings, job: Job): ProviderView =>
  views.find((v) => v.provider.id === settings[job.providerKey]) ?? views.find((v) => v.provider.id === DEFAULT_PROVIDER_ID)!;

function accessOf(provider: ProviderConfig, status: ProviderCredentialStatus | undefined, lookupFailed: boolean): Access {
  if (!isHttpKind(provider.kind)) return { kind: 'local' };
  if (status?.hasStoredKey) return { kind: 'saved', masked: status.maskedKey ?? 'Key saved' };
  if (status?.hasEnvKey && status.envVar) return { kind: 'system', envVar: status.envVar };
  return { kind: (status ? status.error : lookupFailed) ? 'unknown' : 'missing' };
}

export function providerViews(settings: Settings, statuses: ProviderCredentialStatus[], lookupFailed: boolean): ProviderView[] {
  const views = settings.providers.map((provider): ProviderView => ({
    provider,
    access: accessOf(provider, statuses.find((s) => s.providerId === provider.id), lookupFailed),
    jobs: [],
    listed: false,
  }));
  for (const job of JOBS) jobProvider(views, settings, job).jobs.push(job);
  for (const v of views) v.listed = isReady(v) || !v.provider.builtin || v.jobs.length > 0;
  return views;
}

export function accessText(access: Access): string {
  switch (access.kind) {
    case 'local': return 'On this computer';
    case 'saved': return access.masked;
    case 'system': return 'Key from your system';
    case 'missing': return 'Needs a key';
    case 'unknown': return 'Key can’t be read';
  }
}

export function accessProblem({ access, provider }: ProviderView): string | null {
  if (access.kind === 'missing') return `${provider.name} needs a key`;
  if (access.kind === 'unknown') return `${provider.name}’s key can’t be read`;
  return null;
}

/** Where a key is sent, short enough for a sentence. */
export function hostOf(address: string): string {
  try {
    return new URL(address).host || address;
  } catch {
    return address;
  }
}

/** Takes a provider out, and gives the jobs it did to the default one, on a model that one has. */
export function withoutProvider(settings: Settings, id: string): Partial<Settings> {
  const patch: Partial<Settings> = { providers: settings.providers.filter((p) => p.id !== id) };
  for (const job of JOBS) {
    if (settings[job.providerKey] === id) Object.assign(patch, { [job.providerKey]: DEFAULT_PROVIDER_ID, [job.modelKey]: DEFAULT_MODELS[job.modelKey] });
  }
  return patch;
}
