import type { SearchEngine, SearchEngineCredentialStatus } from './types';

export interface SearchEngineSpec {
  readonly engine: SearchEngine;
  readonly label: string;
  readonly envVar: string;
  readonly docsUrl: string;
  readonly prefix?: string;
  readonly minLength: number;
}

export const SEARCH_ENGINES: readonly SearchEngineSpec[] = [
  { engine: 'tavily', label: 'Tavily', envVar: 'TAVILY_API_KEY', docsUrl: 'https://app.tavily.com/', prefix: 'tvly-', minLength: 20 },
  { engine: 'brave', label: 'Brave Search', envVar: 'BRAVE_API_KEY', docsUrl: 'https://brave.com/search/api/', minLength: 20 },
  { engine: 'exa', label: 'Exa', envVar: 'EXA_API_KEY', docsUrl: 'https://dashboard.exa.ai/api-keys', minLength: 12 },
  { engine: 'firecrawl', label: 'Firecrawl', envVar: 'FIRECRAWL_API_KEY', docsUrl: 'https://www.firecrawl.dev/app/api-keys', prefix: 'fc-', minLength: 20 },
  { engine: 'ollama', label: 'Ollama', envVar: 'OLLAMA_API_KEY', docsUrl: 'https://ollama.com/settings/keys', minLength: 12 },
];

export const FALLBACK_SEARCH_ENGINES: SearchEngineCredentialStatus[] = SEARCH_ENGINES.map(
  ({ engine, label, envVar, docsUrl }) => ({
    engine,
    label,
    envVar,
    docsUrl,
    hasStoredKey: false,
    maskedKey: null,
    hasEnvKey: false,
  }),
);
