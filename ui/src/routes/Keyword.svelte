<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    keywordConfigStore,
    keywordRuleSetsStore,
    type KeywordConfig,
    type NamedKeywordRule,
  } from "../lib/stores";

  import { keywordEditorTargetRuleStore } from "../lib/stores";

  let rootMinimumShouldMatch = 1;
  let groups = [{ occur: "should", terms: [""], minimumShouldMatch: 1 }];
  let status = "";
  let ruleName = "";
  let selectedRuleName = "";
  let namedRules: NamedKeywordRule[] = [];
  let editorTargetRule: string | null = null;

  const unsubscribeEditorTarget = keywordEditorTargetRuleStore.subscribe((value) => {
    editorTargetRule = value;
    if (value) {
      selectedRuleName = value;
      loadSelectedRule();
    } else {
      selectedRuleName = "";
      ruleName = "";
    }
  });

  const unsubscribeConfig = keywordConfigStore.subscribe((value) => {
    rootMinimumShouldMatch = value.rootMinimumShouldMatch;
    groups = value.groups.map((group) => ({ ...group, terms: [...group.terms] }));
  });

  const unsubscribeNamedRules = keywordRuleSetsStore.subscribe((value) => {
    namedRules = value;
    if (!selectedRuleName && value.length > 0) {
      selectedRuleName = value[0].name;
    }
  });

  onDestroy(() => {
    unsubscribeConfig();
    unsubscribeNamedRules();
    unsubscribeEditorTarget();
  });

  function normalizeGroups() {
    return groups.map((group) => ({
      occur: group.occur,
      terms: group.terms[0]?.split(",").map((term) => term.trim()).filter(Boolean) || [],
      minimumShouldMatch: Number(group.minimumShouldMatch) || 1,
    }));
  }

  function updateCurrentConfig() {
    keywordConfigStore.set({ rootMinimumShouldMatch, groups: normalizeGroups() });
  }

  function saveConfig() {
    updateCurrentConfig();
    status = "关键词规则已保存到当前配置";
  }

  function saveNamedRule() {
    const name = ruleName.trim();
    if (!name) {
      status = "请输入规则名称";
      return;
    }

    const config: KeywordConfig = {
      rootMinimumShouldMatch,
      groups: normalizeGroups(),
    };

    const updatedRules = namedRules.filter((rule) => rule.name !== name);
    updatedRules.unshift({
      name,
      config,
      createdAt: new Date().toISOString(),
    });

    keywordRuleSetsStore.set(updatedRules);
    selectedRuleName = name;
    status = `已保存规则：${name}`;
  }

  function loadSelectedRule() {
    const rule = namedRules.find((item) => item.name === selectedRuleName);
    if (!rule) {
      status = "请选择一个已有规则";
      return;
    }

    rootMinimumShouldMatch = rule.config.rootMinimumShouldMatch;
    groups = rule.config.groups.map((group) => ({ ...group, terms: [...group.terms] }));
    ruleName = rule.name;
    updateCurrentConfig();
    status = `已加载规则：${rule.name}`;
  }

  function deleteSelectedRule() {
    if (!selectedRuleName) {
      status = "请选择要删除的规则";
      return;
    }

    keywordRuleSetsStore.set(namedRules.filter((item) => item.name !== selectedRuleName));
    status = `已删除规则：${selectedRuleName}`;
    selectedRuleName = namedRules.length > 1 ? namedRules.find((item) => item.name !== selectedRuleName)?.name || "" : "";
    ruleName = "";
  }

  function addGroup() {
    groups = [...groups, { occur: "should", terms: [""], minimumShouldMatch: 1 }];
  }

  function removeGroup() {
    if (groups.length <= 1) {
      status = "至少保留一组关键词";
      return;
    }
    groups = groups.slice(0, -1);
  }
</script>

<style>
  .group-list {
    display: grid;
    gap: 14px;
    max-width: 900px;
  }

  .group-row {
    display: grid;
    grid-template-columns: 120px 1fr 150px;
    gap: 12px;
  }

  .action-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 16px;
  }

</style>

<h2>{editorTargetRule ? '编辑关键词规则' : '创建关键词规则'}</h2>
<div>
  <label>
    规则名称
    <input type="text" bind:value={ruleName} placeholder="输入规则名称" />
  </label>

  <div class="group-list">
    {#each groups as group, index}
      <div class="group-row">
        <select bind:value={group.occur}>
          <option value="must">must</option>
          <option value="should">should</option>
          <option value="mustNot">mustNot</option>
        </select>
        <input bind:value={group.terms[0]} placeholder="关键词，逗号分隔" />
        <input type="number" bind:value={group.minimumShouldMatch} min="1" />
      </div>
    {/each}
  </div>

  <div class="action-row">
    <button type="button" on:click={addGroup}>添加一行</button>
    <button type="button" on:click={removeGroup} disabled={groups.length <= 1}>删除一行</button>
  </div>

  <div class="action-row">
    <button on:click={saveNamedRule}>保存规则</button>
    {#if editorTargetRule}
      <button type="button" on:click={deleteSelectedRule}>删除规则</button>
    {/if}
  </div>

  <p>{status}</p>
</div>
