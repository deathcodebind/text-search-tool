<script lang="ts">
  import Router, { link } from "svelte-spa-router";
  import { onDestroy, onMount } from "svelte";
  import { routes } from "./routes";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import {
    appSettingsStore,
    keywordRuleSetsStore,
    activeKeywordRuleStore,
    keywordEditorTargetRuleStore,
    loginStore,
    type NamedKeywordRule,
  } from "./lib/stores";
  import Keyword from "./routes/Keyword.svelte";

  let dialog: any;
  let keywordEditorDialog: any;
  let isSettingsOpen = false;
  let isKeywordEditorOpen = false;
  let theme: "light" | "dark" | "system" = "system";
  let storageMode: "local" | "session" = "local";
  let autoSaveLogin = true;
  let namedKeywordRules: NamedKeywordRule[] = [];
  let loginState = { baseUrl: "", username: "", remember: false };
  let selectedRuleName = "";
  let systemPrefersDark = false;
  let currentTheme: "light" | "dark" = "light";
  let mediaQuery: MediaQueryList | null = null;
  const queryClient = new QueryClient();

  const unsubscribeSettings = appSettingsStore.subscribe((value) => {
    theme = value.theme || "system";
    storageMode = value.storageMode || "local";
    autoSaveLogin = value.autoSaveLogin;
  });

  const unsubscribeLoginStore = loginStore.subscribe((value) => {
    loginState = value;
  });

  const unsubscribeKeywordRules = keywordRuleSetsStore.subscribe((value) => {
    namedKeywordRules = value;
    if (!selectedRuleName && value.length > 0) {
      selectedRuleName = value[0].name;
    }
  });

  function openSettings() {
    dialog?.show();
  }

  function closeSettings() {
    dialog?.hide();
  }

  function toggleSettings() {
    if (dialog?.open) {
      closeSettings();
    } else {
      openSettings();
    }
  }

  function saveSettings() {
    appSettingsStore.set({ theme, storageMode, autoSaveLogin });
    closeSettings();
  }

  function goBack() {
    if (window.history.length > 1) {
      window.history.back();
    } else {
      window.location.hash = "#/login";
    }
  }

  function handleKeywordRuleSelect(event: CustomEvent<{ value?: string; item?: { value?: string } }>) {
    const item = event.detail?.item as any;
    const value =
      event.detail?.value ||
      item?.value ||
      item?.getAttribute?.("value") ||
      (event.target as any)?.value ||
      "";
    selectedRuleName = value;
    if (value === "__add_new__") {
      selectedRuleName = "";
      openKeywordEditor("add");
    }
  }

  function openKeywordEditor(action: "add" | "edit") {
    if (action === "edit") {
      if (!selectedRuleName) {
        return;
      }
      keywordEditorTargetRuleStore.set(selectedRuleName);
    } else {
      keywordEditorTargetRuleStore.set(null);
    }
    keywordEditorDialog?.show();
  }

  function applyRule() {
    const rule = namedKeywordRules.find((r) => r.name === selectedRuleName);
    if (!rule) {
      return;
    }
    activeKeywordRuleStore.set(rule);
    isSettingsOpen = false;
  }

  function clearRule() {
    activeKeywordRuleStore.set(null);
    isSettingsOpen = false;
  }

  function resetKeywordRules() {
    keywordRuleSetsStore.set([]);
    selectedRuleName = "";
  }

  function updateSystemTheme(event?: MediaQueryListEvent) {
    systemPrefersDark = event ? event.matches : mediaQuery?.matches || false;
  }

  onMount(() => {
    if (!window.location.hash || window.location.hash === "#" || window.location.hash === "#/") {
      window.location.hash = "#/login";
    }

    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    updateSystemTheme();

    if (mediaQuery?.addEventListener) {
      mediaQuery.addEventListener("change", updateSystemTheme);
    } else if (mediaQuery?.addListener) {
      mediaQuery.addListener(updateSystemTheme);
    }
  });

  onDestroy(() => {
    if (mediaQuery?.removeEventListener) {
      mediaQuery.removeEventListener("change", updateSystemTheme);
    } else if (mediaQuery?.removeListener) {
      mediaQuery.removeListener(updateSystemTheme);
    }
    unsubscribeSettings();
    unsubscribeLoginStore();
    unsubscribeKeywordRules();
  });

  $: currentTheme = theme === "system" ? (systemPrefersDark ? "dark" : "light") : theme;
  $: document.documentElement.dataset.theme = currentTheme;
</script>

<style>
  .app-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--ink);
  }

  header {
    position: relative;
    padding: 18px 24px 12px;
    background: var(--header-bg);
    color: var(--header-ink, #f8fafc);
  }

  .header-content {
    width: min(1050px, 92vw);
    margin: 0 auto;
    display: grid;
    gap: 18px;
    place-items: start center;
    padding-top: 10px;
  }

  .header-toolbar {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .back-button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 16px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: inherit;
    cursor: pointer;
    font-weight: 600;
  }

  .back-button:hover {
    background: var(--surface-hover);
  }

  .header-brand {
    display: grid;
    gap: 6px;
    justify-self: center;
    text-align: center;
  }

  .header-brand h1 {
    margin: 0;
    font-size: 2rem;
  }

  .header-brand p {
    margin: 0;
    color: var(--muted);
  }

  .header-settings {
    display: flex;
    justify-content: flex-end;
  }

  .settings-toggle-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.18);
    color: inherit;
    cursor: pointer;
  }

  .settings-toggle-button svg {
    width: 24px;
    height: 24px;
    stroke-width: 2;
  }

  .settings-toggle-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.18);
    color: inherit;
    cursor: pointer;
  }

  .settings-toggle-button svg {
    width: 24px;
    height: 24px;
    stroke-width: 2;
  }

  .settings-blocks {
    display: grid;
    gap: 14px;
  }

  details {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 14px 16px;
    background: var(--card);
  }

  summary {
    cursor: pointer;
    list-style: none;
    font-weight: 700;
    font-size: 1rem;
    margin-bottom: 12px;
    outline: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary::before {
    content: "▸";
    display: inline-block;
    width: 1.2em;
    transform: rotate(0deg);
    transition: transform 0.2s ease;
  }

  details[open] summary::before {
    transform: rotate(90deg);
  }

  .settings-block-content {
    display: grid;
    gap: 14px;
    padding-left: 1.2em;
  }

  .keyword-actions {
    display: flex;
    gap: 12px;
    flex-wrap: nowrap;
  }

  .keyword-rule-toolbar {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }

  .keyword-rule-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .page-body {
    position: relative;
    flex: 1;
    padding: 20px 20px 20px;
    max-width: 1200px;
    margin: 0 auto;
  }

</style>

<div class="app-shell">
  <header>
    <div class="header-content">
        <div class="header-brand">
          <h1>Text Search Tool</h1>
          <p>迁移中：Svelte + Tauri + SPA 路由</p>
        </div>
      </div>
    </header>

  <main class="page-body">
    <QueryClientProvider client={queryClient}>
      <Router {routes} />
    </QueryClientProvider>
  </main>
</div>

<sl-dialog
  class="settings-dialog"
  label="应用设置"
  bind:this={dialog}
  on:sl-after-show={() => (isSettingsOpen = true)}
  on:sl-after-hide={() => (isSettingsOpen = false)}
>
  <div class="settings-blocks">
    <details open>
      <summary>外观设置</summary>
      <div class="settings-block-content">
        <sl-select label="主题" value={theme} on:sl-change={(event) => (theme = event.target.value)}>
          <sl-menu-item value="light" selected={theme === "light"}>浅色</sl-menu-item>
          <sl-menu-item value="dark" selected={theme === "dark"}>深色</sl-menu-item>
          <sl-menu-item value="system" selected={theme === "system"}>跟随系统</sl-menu-item>
        </sl-select>
      </div>
    </details>

    <details>
      <summary>存储与登录</summary>
      <div class="settings-block-content">
        <sl-select label="存储模式" value={storageMode} on:sl-change={(event) => (storageMode = event.target.value)}>
          <sl-menu-item value="local">本地存储</sl-menu-item>
          <sl-menu-item value="session">会话存储</sl-menu-item>
        </sl-select>

        <sl-checkbox checked={autoSaveLogin} on:sl-change={(event) => (autoSaveLogin = event.target.checked)}>自动保存登录</sl-checkbox>
      </div>
    </details>

    <details>
      <summary>关键词规则</summary>
      <div class="settings-block-content">
        <div class="keyword-rule-toolbar">
          <sl-select label="选择规则" value={selectedRuleName} on:sl-select={handleKeywordRuleSelect} on:sl-change={handleKeywordRuleSelect}>
            <sl-menu-item value="" disabled>请选择规则</sl-menu-item>
            {#each namedKeywordRules as rule}
              <sl-menu-item value={rule.name}>{rule.name}</sl-menu-item>
            {/each}
            <sl-menu-item
              value="__add_new__"
              role="button"
              tabindex="0"
              on:click={() => {
                selectedRuleName = "";
                openKeywordEditor("add");
              }}
            >
              添加规则
            </sl-menu-item>
          </sl-select>

          <div class="keyword-rule-actions">
            <sl-button type="button" variant="default" outline role="button" tabindex="0" disabled={!selectedRuleName} on:click={() => openKeywordEditor("edit")}>修改规则</sl-button>
          </div>
        </div>

        <div class="keyword-actions">
          <sl-button type="button" variant="primary" outline disabled={!selectedRuleName} role="button" tabindex="0" on:click={applyRule}>应用规则</sl-button>
          <sl-button type="button" variant="default" outline role="button" tabindex="0" on:click={clearRule}>清除已应用规则</sl-button>
          <sl-button type="button" variant="danger" outline role="button" tabindex="0" on:click={resetKeywordRules}>清空本地关键词规则</sl-button>
        </div>
      </div>
    </details>
  </div>

  <div slot="footer" class="settings-footer">
    <sl-button type="button" variant="default" outline role="button" tabindex="0" on:click={closeSettings}>取消</sl-button>
    <sl-button type="button" variant="primary" role="button" tabindex="0" on:click={saveSettings}>保存</sl-button>
  </div>
</sl-dialog>

<sl-dialog
  class="keyword-editor-dialog"
  label="关键词规则编辑"
  bind:this={keywordEditorDialog}
  on:sl-after-show={() => (isKeywordEditorOpen = true)}
  on:sl-after-hide={() => {
    isKeywordEditorOpen = false;
    keywordEditorTargetRuleStore.set(null);
  }}
>
  <Keyword />
</sl-dialog>
