import type { ProviderConfig, ProviderKind } from './types';

export const DEFAULT_PROVIDER_ID = 'claude-cli';

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
    name: 'Antigravity CLI (ACP)',
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
    name: 'OpenAI OAuth (Codex CLI)',
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
    name: 'Z.AI / GLM (ZLM)',
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

export function isCliKind(kind: ProviderKind): boolean {
  return !isHttpKind(kind);
}

export function mergeProviders(saved: ProviderConfig[] | undefined): ProviderConfig[] {
  const base = saved && saved.length > 0 ? saved.map((p) => ({ ...p, args: [...(p.args ?? [])], models: [...(p.models ?? [])] })) : [];
  for (const preset of DEFAULT_PROVIDERS) {
    const existing = base.find((p) => p.id === preset.id);
    if (existing) {
      if (existing.builtin) {
        existing.models = [...preset.models];
      }
    } else {
      base.push({ ...preset, args: [...preset.args], models: [...preset.models] });
    }
  }
  return base;
}
