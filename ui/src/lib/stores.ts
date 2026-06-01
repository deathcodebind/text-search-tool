import { writable } from "svelte/store";

export interface KeywordGroup {
  occur: "must" | "should" | "mustNot";
  terms: string[];
  minimumShouldMatch: number;
}

export interface KeywordConfig {
  rootMinimumShouldMatch: number;
  groups: KeywordGroup[];
}

export interface NamedKeywordRule {
  name: string;
  config: KeywordConfig;
  createdAt: string;
}

function createPersistedStore<T>(key: string, initial: T) {
  const stored = typeof window !== "undefined" ? localStorage.getItem(key) : null;
  const data = stored ? (JSON.parse(stored) as T) : initial;
  const store = writable<T>(data);

  if (typeof window !== "undefined") {
    store.subscribe((value) => {
      localStorage.setItem(key, JSON.stringify(value));
    });
  }

  return store;
}

export const loginStore = createPersistedStore("text-search-tool.login.v1", {
  baseUrl: "https://login.jxemall.com",
  username: "",
  remember: true,
});

export const keywordConfigStore = createPersistedStore<KeywordConfig>("text-search-tool.keyword.v1", {
  rootMinimumShouldMatch: 1,
  groups: [
    {
      occur: "should",
      terms: [""],
      minimumShouldMatch: 1,
    },
  ],
});

export const keywordRuleSetsStore = createPersistedStore<NamedKeywordRule[]>("text-search-tool.named-keyword-rules.v1", []);

export interface AppSettings {
  theme: "light" | "dark" | "system";
  storageMode: "local" | "session";
  autoSaveLogin: boolean;
}

export const appSettingsStore = createPersistedStore<AppSettings>("text-search-tool.app-settings.v1", {
  theme: "system",
  storageMode: "local",
  autoSaveLogin: true,
});

export const activeKeywordRuleStore = writable<NamedKeywordRule | null>(null);
export const keywordEditorTargetRuleStore = writable<string | null>(null);
