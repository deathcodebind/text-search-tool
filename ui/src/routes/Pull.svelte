<script lang="ts">
  import { derived, get, writable } from "svelte/store";
  import { onDestroy, onMount } from "svelte";
  import { pullStart, pullCancel, pullRecords, pullProgress, fetchDetailPageHtml } from "../lib/api";
  import { loginStore, keywordRuleSetsStore, activeKeywordRuleStore, keywordEditorTargetRuleStore } from "../lib/stores";
  import { JXEMALL_DISTRICT_OPTIONS } from "../../jxemall-district-options.js";
  import Keyword from "./Keyword.svelte";

  const pullStatus = writable("尚未启动");
  const currentJobId = writable("");
  const regionKeyword = writable("");
  const selectedDistricts = writable<string[]>([]);
  const records = writable<any[]>([]);
  const page = writable(1);
  const pageSize = 10;
  const TASK_HISTORY_KEY = "text-search-tool.pull-task-history.v1";
  const PULL_PROGRESS_POLL_INTERVAL_MS = 4000;

  type PullTaskHistoryEntry = {
    jobId: string;
    startedAt: string;
    updatedAt: string;
    expiresAt?: string;
    requestSignature?: string;
    ruleSignature?: string;
    filterSummary: string;
    progressText: string;
    snapshotCount: number;
    snapshotSourceIds: string[];
  };

  let allRecords: any[] = [];
  let totalRecords = 0;
  let currentPageNo = 1;
  let currentPageSize = pageSize;
  let isKeywordPanelOpen = false;
  let isLoadingRecords = false;
  let isKeywordEditorOpen = false;
  let keywordEditorDialog: any;
  let taskHistoryDialog: any;
  let startPullConfirmDialog: any;
  let stateWarningDialog: any;
  let embeddedBrowserDialog: any;
  let isEmbeddedBrowserOpen = false;
  let embeddedBrowserLoading = false;
  let embeddedBrowserHtml = "";
  let embeddedBrowserError = "";
  let embeddedBrowserSourceUrl = "";
  let embeddedBrowserTitle = "";
  let taskHistory: PullTaskHistoryEntry[] = [];
  let namedRules: any[] = [];
  let selectedRuleName = "";
  let keywordStatus = "";
  let activeRuleLabel = "未应用";
  let activeRuleConfig: any = null;
  let loginState = { baseUrl: "", username: "", remember: false };
  let suppressNextAutoLoad = false;
  let pullProgressPollTimer: ReturnType<typeof setInterval> | null = null;
  let isPollingProgress = false;
  let startPullConfirmTitle = "";
  let startPullConfirmMessage = "";
  let startPullConfirmActionLabel = "确认";
  let startPullConfirmResolved = false;
  let startPullConfirmResolver: ((confirmed: boolean) => void) | null = null;
  let stateWarningMessage = "";

  const isProvinceCode = (code: string) => code.slice(2) === "0000";
  const isCityCode = (code: string) => code.slice(4) === "00" && code.slice(2, 4) !== "00";
  const provinceCode = (code: string) => `${code.slice(0, 2)}0000`;
  const cityCode = (code: string) => `${code.slice(0, 4)}00`;

  const provinces = JXEMALL_DISTRICT_OPTIONS.filter((item) => isProvinceCode(item.code));
  const cityOptions = JXEMALL_DISTRICT_OPTIONS.filter((item) => isCityCode(item.code));
  const countyOptions = JXEMALL_DISTRICT_OPTIONS.filter((item) => !isProvinceCode(item.code) && !isCityCode(item.code));

  let selectedProvinceCode = "";
  let selectedCityCode = "";
  let isCountyDropdownOpen = false;
  let isFilterSectionOpen = true;
  let quickCategory = "all";
  let quickState = "bidding";
  let quickInstance = "all";
  let quickBudget = "all";

  let selectedCategoryType = "";
  let selectedStateList: number[] = [4];
  let selectedInstanceCodes: string[] = ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"];
  let minBudgetInput = "";
  let maxBudgetInput = "";
  let selectedSortField = "ANNOUNCEMENT_PUBLISH_TIME";
  let selectedSortMethod = "DESC";

  const quickCategoryOptions = [
    { key: "all", label: "全部", value: "" },
    { key: "goods", label: "货物类", value: "GOODS" },
    { key: "service", label: "服务类", value: "SERVICE" },
    { key: "project", label: "工程类", value: "PROJECT" },
  ];

  const quickStateOptions = [
    { key: "all", label: "全部", values: [3, 4, 5, 6, 7, 10, 12, 50] },
    { key: "pending", label: "竞价未开始", values: [3] },
    { key: "bidding", label: "竞价中", values: [4] },
    { key: "expired", label: "已过期", values: [5, 6, 7, 10, 12, 50] },
  ];

  const quickInstanceOptions = [
    { key: "all", label: "全部", values: ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"] },
    { key: "JXWSCS", label: "江西网上超市", values: ["JXWSCS"] },
    { key: "JXDDCG", label: "江西定点采购馆", values: ["JXDDCG"] },
    { key: "JXXYGH", label: "江西协议供货馆", values: ["JXXYGH"] },
    { key: "JXFWGC", label: "江西服务工程馆", values: ["JXFWGC"] },
  ];

  const quickBudgetOptions = [
    { key: "all", label: "全部", min: null, max: null },
    { key: "lt50k", label: "5万以下", min: null, max: 50000 },
    { key: "50to60", label: "5-6万", min: 50000, max: 60000 },
    { key: "60to70", label: "6-7万", min: 60000, max: 70000 },
    { key: "gt100k", label: "10万以上", min: 100000, max: null },
  ];

  const sortFieldOptions = [
    { value: "ANNOUNCEMENT_PUBLISH_TIME", label: "发布时间" },
    { value: "QUOTE_DEADLINE", label: "竞价截止时间" },
    { value: "BUDGET_AMOUNT", label: "控制总价" },
  ];

  const sortMethodOptions = [
    { value: "DESC", label: "降序" },
    { value: "ASC", label: "升序" },
  ];

  $: currentCities = cityOptions.filter((item) => provinceCode(item.code) === selectedProvinceCode);
  $: if (!currentCities.find((item) => item.code === selectedCityCode)) {
    selectedCityCode = currentCities[0]?.code || "";
  }
  $: currentCounties = countyOptions.filter((item) => cityCode(item.code) === selectedCityCode);
  $: filteredCounties = currentCounties.filter((item) => {
    const term = $regionKeyword.trim().toLowerCase();
    if (!term) {
      return true;
    }
    return item.code.includes(term) || item.name.toLowerCase().includes(term);
  });
  $: selectedCountiesInCity = currentCounties.filter((item) => $selectedDistricts.includes(item.code)).length;
  $: totalPages = Math.max(1, Math.ceil(totalRecords / currentPageSize));
  $: canGoPrev = $page > 1;
  $: canGoNext = $page < totalPages;
  $: if (!selectedCityCode) {
    isCountyDropdownOpen = false;
  }

  const selectedDistrictLabels = derived(selectedDistricts, ($selected) => {
    return $selected.map((code) => {
      const item = JXEMALL_DISTRICT_OPTIONS.find((entry) => entry.code === code);
      return {
        code,
        label: item ? `${item.name} (${item.code})` : code,
      };
    });
  });

  const unsubscribeLogin = loginStore.subscribe((value) => {
    loginState = value;
  });

  const unsubscribeNamedRules = keywordRuleSetsStore.subscribe((value) => {
    namedRules = value;
    if (!selectedRuleName && value.length > 0) {
      selectedRuleName = value[0].name;
    }
  });

  const unsubscribeActiveRule = activeKeywordRuleStore.subscribe((rule) => {
    if (rule) {
      activeRuleLabel = rule.name;
      activeRuleConfig = rule.config;
      if (allRecords.length > 0) {
        applyKeywordFilter(rule.config);
      }
    } else {
      activeRuleLabel = "未应用";
      activeRuleConfig = null;
      records.set(allRecords);
    }
  });

  onMount(() => {
    selectQuickState("bidding", false);
    restoreTaskHistory();
    if (!loginState.username) {
      window.location.hash = "#/login";
    }
  });

  onDestroy(() => {
    stopAutoPolling();
    unsubscribeLogin();
    unsubscribeNamedRules();
    unsubscribeActiveRule();
  });

  function isTerminalPullStatus(status: string) {
    const normalized = status.trim().toLowerCase();
    return normalized === "succeeded" || normalized === "failed" || normalized === "canceled";
  }

  function stopAutoPolling() {
    if (pullProgressPollTimer) {
      clearInterval(pullProgressPollTimer);
      pullProgressPollTimer = null;
    }
  }

  function startAutoPolling() {
    if (pullProgressPollTimer || !get(currentJobId)) {
      return;
    }

    pullProgressPollTimer = setInterval(() => {
      void queryProgress(true);
    }, PULL_PROGRESS_POLL_INTERVAL_MS);
  }

  function openStartPullConfirmDialog(title: string, message: string, actionLabel: string) {
    startPullConfirmTitle = title;
    startPullConfirmMessage = message;
    startPullConfirmActionLabel = actionLabel;
    startPullConfirmResolved = false;

    return new Promise<boolean>((resolve) => {
      startPullConfirmResolver = resolve;
      startPullConfirmDialog?.show();
    });
  }

  function resolveStartPullConfirm(confirmed: boolean) {
    if (startPullConfirmResolved) {
      return;
    }
    startPullConfirmResolved = true;
    const resolver = startPullConfirmResolver;
    startPullConfirmResolver = null;
    resolver?.(confirmed);
  }

  function openStateWarningDialog(message: string) {
    stateWarningMessage = message;
    stateWarningDialog?.show();
  }

  function toggleDistrict(code: string) {
    selectedDistricts.update((current) => {
      if (current.includes(code)) {
        return current.filter((item) => item !== code);
      }
      return [...current, code];
    });
  }

  function addVisibleDistricts() {
    selectedDistricts.update((current) => {
      const next = new Set(current);
      const source = get(regionKeyword).trim() ? filteredCounties : currentCounties;
      for (const district of source) {
        next.add(district.code);
      }
      return Array.from(next);
    });
  }

  function toggleCountyDropdown() {
    if (!selectedCityCode) {
      return;
    }
    isCountyDropdownOpen = !isCountyDropdownOpen;
  }

  function clearDistricts() {
    selectedDistricts.set([]);
  }

  function parseOptionalBudget(input: string): number | undefined {
    const trimmed = input.trim();
    if (!trimmed) {
      return undefined;
    }
    const value = Number(trimmed);
    if (!Number.isFinite(value) || value < 0) {
      return undefined;
    }
    return Math.floor(value);
  }

  function restoreTaskHistory() {
    try {
      const raw = localStorage.getItem(TASK_HISTORY_KEY);
      if (!raw) {
        taskHistory = [];
        return;
      }
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        taskHistory = [];
        return;
      }
      taskHistory = parsed.filter((item) => item && typeof item.jobId === "string").slice(0, 20);
      if (taskHistory.length > 0 && !get(currentJobId)) {
        currentJobId.set(taskHistory[0].jobId);
      }
    } catch {
      taskHistory = [];
    }
  }

  function persistTaskHistory() {
    try {
      localStorage.setItem(TASK_HISTORY_KEY, JSON.stringify(taskHistory.slice(0, 20)));
    } catch {
      // ignore persistence failure
    }
  }

  function upsertTaskHistory(next: PullTaskHistoryEntry) {
    const filtered = taskHistory.filter((item) => item.jobId !== next.jobId);
    taskHistory = [next, ...filtered].slice(0, 20);
    persistTaskHistory();
  }

  function resolveQuickLabel<T extends { key: string; label: string }>(source: T[], key: string) {
    return source.find((item) => item.key === key)?.label || "全部";
  }

  function buildFilterSummary() {
    const districtCount = get(selectedDistricts).length;
    const districtText = districtCount > 0 ? `${districtCount} 个地区` : "全部地区";
    return [
      `采购内容:${resolveQuickLabel(quickCategoryOptions, quickCategory)}`,
      `状态:${resolveQuickLabel(quickStateOptions, quickState)}`,
      `实例:${resolveQuickLabel(quickInstanceOptions, quickInstance)}`,
      `预算:${resolveQuickLabel(quickBudgetOptions, quickBudget)}`,
      `地区:${districtText}`,
    ].join(" | ");
  }

  function formatDateTime(iso: string) {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) {
      return iso;
    }
    return date.toLocaleString();
  }

  function buildCurrentPullRequestSignature() {
    return JSON.stringify({
      districtCodes: [...get(selectedDistricts)].sort(),
      categoryType: selectedCategoryType || null,
      stateList: [...selectedStateList].sort((a, b) => a - b),
      instanceCodes: [...selectedInstanceCodes].sort(),
      minBudgetInput: minBudgetInput.trim(),
      maxBudgetInput: maxBudgetInput.trim(),
      sortField: selectedSortField,
      sortMethod: selectedSortMethod,
    });
  }

  function buildCurrentRuleSignature() {
    if (!activeRuleConfig) {
      return "none";
    }
    return JSON.stringify(activeRuleConfig);
  }

  function isTaskStillRunning(entry?: PullTaskHistoryEntry) {
    if (!entry) {
      return false;
    }
    const text = (entry.progressText || "").toLowerCase();
    return !text.includes("succeeded")
      && !text.includes("failed")
      && !text.includes("canceled")
      && !text.includes("已取消")
      && !text.includes("无新拉取项目");
  }

  function rememberTaskStart(jobId: string) {
    const nowIso = new Date().toISOString();
    upsertTaskHistory({
      jobId,
      startedAt: nowIso,
      updatedAt: nowIso,
      requestSignature: buildCurrentPullRequestSignature(),
      ruleSignature: buildCurrentRuleSignature(),
      filterSummary: buildFilterSummary(),
      progressText: "已启动",
      snapshotCount: 0,
      snapshotSourceIds: [],
    });
  }

  function updateTaskProgress(jobId: string, progressText: string) {
    const existing = taskHistory.find((item) => item.jobId === jobId);
    if (!existing) {
      return;
    }
    upsertTaskHistory({
      ...existing,
      updatedAt: new Date().toISOString(),
      progressText,
    });
  }

  function updateTaskSnapshot(jobId: string) {
    const existing = taskHistory.find((item) => item.jobId === jobId);
    if (!existing) {
      return;
    }
    const sourceIds = allRecords.map((item) => item.sourceId).filter(Boolean);
    const expiresAt = allRecords[0]?.expiresAt;
    upsertTaskHistory({
      ...existing,
      updatedAt: new Date().toISOString(),
      expiresAt: typeof expiresAt === "number" ? new Date(expiresAt * 1000).toISOString() : existing.expiresAt,
      snapshotCount: sourceIds.length,
      snapshotSourceIds: sourceIds.slice(0, 20),
    });
  }

  function collectHistorySourceIds(excludeJobId: string) {
    const ids = new Set<string>();
    for (const item of taskHistory) {
      if (item.jobId === excludeJobId) {
        continue;
      }
      for (const id of item.snapshotSourceIds || []) {
        if (id) {
          ids.add(id);
        }
      }
    }
    return ids;
  }

  function hasNewItemsComparedToHistory(jobId: string) {
    if (allRecords.length === 0) {
      return false;
    }
    const historyIds = collectHistorySourceIds(jobId);
    if (historyIds.size === 0) {
      return true;
    }
    return allRecords.some((item) => {
      const sourceId = item?.sourceId;
      return sourceId && !historyIds.has(sourceId);
    });
  }

  async function useHistoryJob(jobId: string) {
    currentJobId.set(jobId);
    pullStatus.set(`已切换任务：${jobId}`);
    page.set(1);
    await loadRecords();
  }

  function openTaskHistoryDialog() {
    taskHistoryDialog?.show();
  }

  function selectQuickCategory(key: string) {
    quickCategory = key;
    const option = quickCategoryOptions.find((item) => item.key === key);
    selectedCategoryType = option?.value || "";
  }

  function selectQuickState(key: string, shouldWarn = true) {
    quickState = key;
    const option = quickStateOptions.find((item) => item.key === key);
    selectedStateList = option ? [...option.values] : [4];

    if (shouldWarn && key !== "bidding") {
      openStateWarningDialog("不推荐拉取非竞价中的数据，建议优先选择“竞价中”。");
    }
  }

  function selectQuickInstance(key: string) {
    quickInstance = key;
    const option = quickInstanceOptions.find((item) => item.key === key);
    selectedInstanceCodes = option ? [...option.values] : ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"];
  }

  function selectQuickBudget(key: string) {
    quickBudget = key;
    const option = quickBudgetOptions.find((item) => item.key === key);
    if (!option) {
      minBudgetInput = "";
      maxBudgetInput = "";
      return;
    }
    minBudgetInput = option.min === null ? "" : String(option.min);
    maxBudgetInput = option.max === null ? "" : String(option.max);
  }

  function buildRecordText(record: any) {
    return [record.title, record.regionCode, record.sourceId, record.detailStatus]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
  }

  function recordMatchesRule(record: any, config: any) {
    const text = buildRecordText(record);
    let matchedGroups = 0;

    for (const group of config.groups || []) {
      const terms = (group.terms || []).filter(Boolean).map((term: string) => term.toLowerCase());
      const foundCount = terms.filter((term: string) => text.includes(term)).length;
      let matched = false;

      if (group.occur === "must") {
        matched = terms.every((term: string) => text.includes(term));
      } else if (group.occur === "should") {
        matched = foundCount >= (group.minimumShouldMatch || 1);
      } else if (group.occur === "mustNot") {
        matched = foundCount === 0;
      }

      if (matched) {
        matchedGroups += 1;
      }
    }

    return matchedGroups >= (config.rootMinimumShouldMatch || 1);
  }

  function applyKeywordFilter(config: any) {
    records.set(allRecords.filter((record) => recordMatchesRule(record, config)));
  }

  async function guardBeforeStartPull() {
    const activeJobId = get(currentJobId);
    if (!activeJobId) {
      return true;
    }

    const runningEntry = taskHistory.find((item) => item.jobId === activeJobId);
    let liveStatus = "";

    try {
      const progress = await pullProgress(activeJobId);
      liveStatus = (progress?.status || "").toLowerCase();
      updateTaskProgress(activeJobId, `状态：${progress.status}；${progress.processed}/${progress.total}`);
      if (isTerminalPullStatus(progress.status)) {
        return true;
      }
    } catch {
      if (!isTaskStillRunning(runningEntry)) {
        return true;
      }
    }

    const sameRequest = runningEntry?.requestSignature === buildCurrentPullRequestSignature();
    const sameRule = (runningEntry?.ruleSignature || "none") === buildCurrentRuleSignature();

    if (sameRequest && sameRule) {
      const confirmed = await openStartPullConfirmDialog(
        "重复拉取确认",
        "和上次任务的筛选项及关键词规则均无变化。拉取源数据可能已经更新，是否仍然要启动新任务重新拉取？",
        "仍然启动新任务"
      );
      if (!confirmed) {
        pullStatus.set("当前任务需求无变化，已取消重复启动");
        return false;
      }
    } else {
      const confirmed = await openStartPullConfirmDialog(
        "任务切换确认",
        "当前任务尚未完成。是否停止当前任务，并按新的筛选条件或关键词规则开始新的拉取任务？",
        "停止当前任务并启动新任务"
      );
      if (!confirmed) {
        pullStatus.set("当前任务继续执行，未启动新任务");
        return false;
      }
    }

    if (activeJobId) {
      await pullCancel(activeJobId);
      updateTaskProgress(activeJobId, "已取消");
      stopAutoPolling();
    }

    return true;
  }

  async function startPull() {
    try {
      const shouldStart = await guardBeforeStartPull();
      if (!shouldStart) {
        return;
      }

      pullStatus.set("启动中...");
      const minBudget = parseOptionalBudget(minBudgetInput);
      const maxBudget = parseOptionalBudget(maxBudgetInput);
      if (minBudget !== undefined && maxBudget !== undefined && minBudget > maxBudget) {
        pullStatus.set("启动失败：最低预算不能大于最高预算");
        return;
      }
      const districts = get(selectedDistricts);
      const payload: any = {
        districtCodes: districts,
        categoryType: selectedCategoryType || null,
        stateList: selectedStateList,
        instanceCodes: selectedInstanceCodes,
        sortField: selectedSortField,
        sortMethod: selectedSortMethod,
        keywordHint: null,
      };

      if (minBudget !== undefined) {
        payload.minBudget = minBudget;
      }
      if (maxBudget !== undefined) {
        payload.maxBudget = maxBudget;
      }

      const result = await pullStart(payload);
      const nextJobId = result?.jobId || "";
      suppressNextAutoLoad = true;
      currentJobId.set(nextJobId);
      page.set(1);
      pullStatus.set(`任务已启动：${nextJobId || "未知"}，后台拉取中，请稍后点击查询进度`);
      if (nextJobId) {
        rememberTaskStart(nextJobId);
        updateTaskProgress(nextJobId, "后台拉取中");
        startAutoPolling();
      }
      records.set([]);
      allRecords = [];
      totalRecords = 0;
      currentPageNo = 1;
      currentPageSize = pageSize;
    } catch (error) {
      const message = String(error);
      pullStatus.set(`启动失败：${message}`);
      if (message.includes("缺少会话 cookie")) {
        window.location.hash = "#/login";
      }
    }
  }

  async function loadRecords() {
    const jobId = get(currentJobId);
    const currentPage = get(page);
    if (!jobId) {
      records.set([]);
      allRecords = [];
      totalRecords = 0;
      currentPageNo = currentPage;
      currentPageSize = pageSize;
      pullStatus.set(taskHistory.length > 0 ? "请选择一个任务查看其拉取结果" : "请先启动拉取任务");
      return;
    }

    isLoadingRecords = true;
    try {
      const resp = await pullRecords(jobId, currentPage, pageSize);
      allRecords = resp?.records || [];
      totalRecords = resp?.total || 0;
      currentPageNo = resp?.page || currentPage;
      currentPageSize = resp?.pageSize || pageSize;
      records.set(allRecords);
      pullStatus.set(`任务 ${jobId} 已加载第 ${resp?.page || currentPage} 页，共 ${resp?.total || 0} 条`);

      if (activeRuleConfig) {
        applyKeywordFilter(activeRuleConfig);
      }
      updateTaskSnapshot(jobId);
    } catch (error) {
      pullStatus.set(`拉取数据失败：${error}`);
      records.set([]);
      allRecords = [];
      totalRecords = 0;
      currentPageNo = currentPage;
      currentPageSize = pageSize;
    } finally {
      isLoadingRecords = false;
    }
  }

  $: if ($page && $currentJobId) {
    if (suppressNextAutoLoad) {
      suppressNextAutoLoad = false;
    } else {
      loadRecords();
    }
  }

  async function queryProgress(triggerLoad = true) {
    if (isPollingProgress) {
      return;
    }

    isPollingProgress = true;
    try {
      const jobId = get(currentJobId);
      if (!jobId) {
        stopAutoPolling();
        pullStatus.set("请先启动拉取任务");
        return;
      }
      const result = await pullProgress(jobId);
      const progressText = `状态：${result.status}；${result.processed}/${result.total}`;
      pullStatus.set(progressText);
      updateTaskProgress(jobId, progressText);

      if (triggerLoad || result.processed > 0 || isTerminalPullStatus(result.status)) {
        await loadRecords();
      }

      if (isTerminalPullStatus(result.status)) {
        stopAutoPolling();
      } else {
        startAutoPolling();
      }

      pullStatus.set(progressText);
    } catch (error) {
      stopAutoPolling();
      pullStatus.set(`查询失败：${error}`);
    } finally {
      isPollingProgress = false;
    }
  }

  $: if ($currentJobId) {
    const history = taskHistory.find((item) => item.jobId === $currentJobId);
    const progressText = history?.progressText || "";
    if (progressText.includes("状态：succeeded") || progressText.includes("状态：failed")) {
      stopAutoPolling();
    } else {
      startAutoPolling();
    }
  } else {
    stopAutoPolling();
  }

  function applySelectedRule() {
    const rule = namedRules.find((item) => item.name === selectedRuleName);
    if (!rule) {
      keywordStatus = "请选择一个关键词规则";
      return;
    }
    activeRuleLabel = rule.name;
    applyKeywordFilter(rule.config);
    keywordStatus = `已应用规则：${rule.name}`;
    isKeywordPanelOpen = false;
  }

  function openKeywordEditor(action: "add" | "edit") {
    if (action === "edit") {
      if (!selectedRuleName) {
        keywordStatus = "请选择要修改的规则";
        return;
      }
      keywordEditorTargetRuleStore.set(selectedRuleName);
    } else {
      keywordEditorTargetRuleStore.set(null);
    }
    isKeywordEditorOpen = true;
    keywordEditorDialog?.show();
  }

  function clearAppliedRule() {
    records.set(allRecords);
    activeRuleLabel = "未应用";
    activeRuleConfig = null;
    keywordStatus = "已清除关键词规则";
  }

  async function openEmbeddedBrowser(record: any) {
    embeddedBrowserError = "";
    embeddedBrowserHtml = "";
    embeddedBrowserSourceUrl = record.sourceUrl || record.source_url || "";
    embeddedBrowserTitle = record.title || record.sourceId || "详情页";
    embeddedBrowserLoading = true;
    isEmbeddedBrowserOpen = true;

    try {
      const result = await fetchDetailPageHtml(record.sourceId);
      embeddedBrowserSourceUrl = result.sourcePageUrl || embeddedBrowserSourceUrl;
      embeddedBrowserHtml = `<base href="${embeddedBrowserSourceUrl}">${result.html}`;
    } catch (error) {
      embeddedBrowserError = `无法加载详情页：${error}`;
    } finally {
      embeddedBrowserLoading = false;
    }
  }

  function goBack() {
    if (window.history.length > 1) {
      window.history.back();
    } else {
      window.location.hash = "#/login";
    }
  }

  function goDetail(sourceId: string) {
    if (!sourceId) {
      return;
    }
    window.location.hash = `#/detail/${sourceId}`;
  }
</script>

<style>
  .section-grid {
    display: grid;
    gap: 16px;
    max-width: 1000px;
  }

  .action-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  .record-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 12px;
  }

  .record-header > div:first-child {
    display: grid;
    gap: 6px;
  }

  .page-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }

  .page-title-left {
    display: inline-flex;
    align-items: center;
    gap: 12px;
  }

  .page-title-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .page-title-row h2 {
    margin: 0;
  }

  .page-title-row .back-button {
    width: fit-content;
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    color: inherit;
    cursor: pointer;
  }

  .page-title-row .back-button:hover {
    background: var(--surface-hover);
  }

  .embedded-browser-toolbar {
    margin-bottom: 12px;
    font-size: 0.95rem;
    color: var(--accent);
  }

  .embedded-browser-frame {
    width: 100%;
    height: 70vh;
    border: 1px solid var(--line);
    border-radius: 12px;
  }

  .header-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .pagination-summary {
    display: inline-grid;
    gap: 2px;
    padding: 0 6px;
    color: #475569;
    font-size: 0.92rem;
  }

  .pagination-summary strong {
    color: #334155;
    font-weight: 600;
  }

  .keyword-panel {
    padding: 16px;
    border: 1px solid #cbd5e1;
    background: #ffffff;
    border-radius: 12px;
    margin-bottom: 16px;
  }

  .district-panel {
    display: grid;
    gap: 12px;
    padding: 16px;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    background: #ffffff;
  }

  .quick-filter-box {
    border: 1px solid var(--line, #e2e8f0);
    border-radius: 8px;
    padding: 6px 14px;
    display: grid;
    gap: 0;
    background: var(--surface-hover, #f8fafc);
  }

  .quick-filter-row {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    padding: 11px 0;
    border-bottom: 1px solid var(--line, #e2e8f0);
  }

  .quick-filter-row:last-child {
    border-bottom: none;
  }

  .quick-filter-label {
    min-width: 72px;
    color: var(--text, #334155);
    font-weight: 600;
  }

  .quick-options {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .quick-option {
    border: none;
    background: transparent;
    color: var(--muted, #64748b);
    padding: 2px 4px;
    cursor: pointer;
    border-radius: 6px;
  }

  .quick-option:hover {
    color: var(--accent, #2563eb);
    background: transparent;
  }

  .quick-option.active {
    color: var(--accent, #2563eb);
    font-weight: 600;
    background: transparent;
  }

  .advanced-toggle {
    width: fit-content;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid #cbd5e1;
    background: #f8fafc;
    color: #334155;
    font-weight: 600;
    cursor: pointer;
  }

  .toggle-arrow {
    display: inline-block;
    transition: transform 0.18s ease;
  }

  .toggle-arrow.expanded {
    transform: rotate(180deg);
  }

  .district-section-title {
    margin: 0;
    color: #334155;
    font-size: 0.92rem;
    font-weight: 600;
  }

  .advanced-panel {
    display: grid;
    gap: 12px;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    background: #f8fafc;
    padding: 12px;
  }

  .advanced-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .district-panel h3 {
    margin: 0;
  }

  .pull-toolbar {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .pull-status {
    margin: 0;
    color: #475569;
    font-size: 0.95rem;
  }

  .task-history {
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    background: #f8fafc;
    padding: 10px 12px;
    display: grid;
    gap: 8px;
  }

  .task-history h4 {
    margin: 0;
    font-size: 0.95rem;
    color: #334155;
  }

  .task-history-empty {
    margin: 0;
    color: #64748b;
    font-size: 0.9rem;
  }

  .task-history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }

  .task-history-item {
    border: 1px solid #dbe4f0;
    border-radius: 8px;
    background: #ffffff;
    padding: 8px 10px;
    display: grid;
    gap: 6px;
  }

  .task-history-item-header {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .task-history-item p {
    margin: 0;
    color: #475569;
    font-size: 0.9rem;
  }

  .task-history-item .summary {
    color: #334155;
  }

  .district-filters {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    align-items: start;
  }

  .filter-field {
    display: grid;
    gap: 6px;
  }

  .filter-field > span {
    font-size: 0.9rem;
    color: #334155;
  }

  .filter-field select,
  .filter-field input {
    padding: 9px 10px;
    border-radius: 10px;
    border: 1px solid #cbd5e1;
    background: #ffffff;
  }

  .county-filter {
    position: relative;
  }

  .county-dropdown-trigger {
    padding: 9px 10px;
    border-radius: 10px;
    border: 1px solid #cbd5e1;
    background: #ffffff;
    text-align: left;
    cursor: pointer;
    min-height: 40px;
  }

  .county-dropdown-trigger:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .county-dropdown-panel {
    position: absolute;
    z-index: 10;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    padding: 10px;
    border-radius: 12px;
    border: 1px solid #cbd5e1;
    background: #ffffff;
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.12);
    display: grid;
    gap: 8px;
  }

  .county-options {
    max-height: 260px;
    overflow-y: auto;
    display: grid;
    gap: 4px;
  }

  .empty-hint {
    margin: 0;
    color: #64748b;
    font-size: 0.9rem;
    padding: 8px;
  }

  @media (max-width: 900px) {
    .district-filters {
      grid-template-columns: 1fr;
    }

    .advanced-grid {
      grid-template-columns: 1fr;
    }

    .quick-filter-label {
      min-width: auto;
    }
  }

  .list-loading,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    min-height: 180px;
    padding: 24px;
    border: 1px dashed #cbd5e1;
    border-radius: 14px;
    background: #f8fafc;
    color: #334155;
    text-align: center;
  }

  .list-loading .spinner {
    width: 28px;
    height: 28px;
    border: 4px solid rgba(15, 23, 42, 0.16);
    border-top-color: var(--accent, #2563eb);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state p + p {
    margin-top: 8px;
    color: #64748b;
  }

  .district-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 6px;
    border-radius: 8px;
  }

  .district-item input {
    accent-color: #0f172a;
  }

  .panel-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
  }

  .keyword-panel p {
    margin: 10px 0 0;
    color: #334155;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .keyword-panel select {
    min-width: 220px;
    padding: 8px 10px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    border: 1px solid #dde4f0;
    padding: 10px;
  }
</style>

<div class="page-title-row">
  <div class="page-title-left">
    <button type="button" class="back-button" on:click={goBack}>←</button>
    <h2>拉取页</h2>
  </div>
  <div class="page-title-actions">
    <button type="button" on:click={openTaskHistoryDialog}>任务记录</button>
  </div>
</div>
<div class="section-grid">
  <div class="district-panel">
    <h3>地区筛选</h3>
    <div class="pull-toolbar">
      <button type="button" on:click={startPull}>开始拉取</button>
      <button type="button" on:click={queryProgress}>查询进度</button>
      <button
        type="button"
        class="advanced-toggle"
        on:click={() => (isFilterSectionOpen = !isFilterSectionOpen)}
        aria-label={isFilterSectionOpen ? "收起筛选项" : "展开筛选项"}
      >
        <span>筛选项</span>
        <span class={`toggle-arrow ${isFilterSectionOpen ? "expanded" : ""}`}>▾</span>
      </button>
    </div>
    <p class="pull-status">{$pullStatus}</p>

    {#if isFilterSectionOpen}
      <div class="quick-filter-box">
        <div class="quick-filter-row">
          <span class="quick-filter-label">采购内容</span>
          <div class="quick-options">
            {#each quickCategoryOptions as item}
              <button
                type="button"
                class={`quick-option ${quickCategory === item.key ? "active" : ""}`}
                on:click={() => selectQuickCategory(item.key)}
              >
                {item.label}
              </button>
            {/each}
          </div>
        </div>

        <div class="quick-filter-row">
          <span class="quick-filter-label">当前状态</span>
          <div class="quick-options">
            {#each quickStateOptions as item}
              <button
                type="button"
                class={`quick-option ${quickState === item.key ? "active" : ""}`}
                on:click={() => selectQuickState(item.key)}
              >
                {item.label}
              </button>
            {/each}
          </div>
        </div>

        <div class="quick-filter-row">
          <span class="quick-filter-label">控制总价</span>
          <div class="quick-options">
            {#each quickBudgetOptions as item}
              <button
                type="button"
                class={`quick-option ${quickBudget === item.key ? "active" : ""}`}
                on:click={() => selectQuickBudget(item.key)}
              >
                {item.label}
              </button>
            {/each}
          </div>
        </div>

        <div class="quick-filter-row">
          <span class="quick-filter-label">业务实例</span>
          <div class="quick-options">
            {#each quickInstanceOptions as item}
              <button
                type="button"
                class={`quick-option ${quickInstance === item.key ? "active" : ""}`}
                on:click={() => selectQuickInstance(item.key)}
              >
                {item.label}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="advanced-panel">
        <div class="advanced-grid">
          <label class="filter-field">
            <span>排序字段</span>
            <select bind:value={selectedSortField}>
              {#each sortFieldOptions as item}
                <option value={item.value}>{item.label}</option>
              {/each}
            </select>
          </label>
          <label class="filter-field">
            <span>排序方式</span>
            <select bind:value={selectedSortMethod}>
              {#each sortMethodOptions as item}
                <option value={item.value}>{item.label}</option>
              {/each}
            </select>
          </label>
        </div>
      </div>

      <p class="district-section-title">地区筛选</p>

      <div class="district-filters">
        <label class="filter-field">
          <span>省</span>
          <select bind:value={selectedProvinceCode}>
            <option value="">请选择省</option>
            {#each provinces as province}
              <option value={province.code}>{province.name}</option>
            {/each}
          </select>
        </label>

        <label class="filter-field">
          <span>市</span>
          <select bind:value={selectedCityCode} disabled={!selectedProvinceCode || currentCities.length === 0}>
            <option value="">请选择市</option>
            {#each currentCities as city}
              <option value={city.code}>{city.name}</option>
            {/each}
          </select>
        </label>

        <div class="filter-field county-filter">
          <span>区/县（多选）</span>
          <button
            type="button"
            class="county-dropdown-trigger"
            on:click={toggleCountyDropdown}
            disabled={!selectedCityCode}
          >
            {#if !selectedCityCode}
              请先选择市
            {:else}
              已选 {selectedCountiesInCity} / {currentCounties.length} 个区县
            {/if}
          </button>

          {#if isCountyDropdownOpen && selectedCityCode}
            <div class="county-dropdown-panel">
              <input bind:value={$regionKeyword} placeholder="搜索区县名称或编码" />
              <div class="county-options">
                {#if filteredCounties.length === 0}
                  <p class="empty-hint">没有匹配的区县</p>
                {/if}
                {#each filteredCounties as district}
                  <label class="district-item">
                    <input
                      type="checkbox"
                      checked={$selectedDistricts.includes(district.code)}
                      on:change={() => toggleDistrict(district.code)}
                    />
                    <span>{district.name} ({district.code})</span>
                  </label>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <div class="district-meta">
        <div class="action-row">
          <button on:click={addVisibleDistricts}>全选当前匹配</button>
          <button on:click={clearDistricts}>清空地区</button>
        </div>
        <p>已选地区：{$selectedDistricts.length} 个</p>
        <ul>
          {#each $selectedDistrictLabels as item}
            <li>{item.label}</li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>


  <div class="record-section">
    <div class="record-header">
      <div>
        <div>当前任务号：{$currentJobId || "未启动"}</div>
        <div>已应用规则：{activeRuleLabel}</div>
      </div>

      <div class="header-actions">
        <button on:click={() => (isKeywordPanelOpen = !isKeywordPanelOpen)}>
          关键词设置
        </button>
        <button on:click={clearAppliedRule} disabled={activeRuleLabel === "未应用"}>
          清除规则
        </button>
        <button on:click={() => page.update((n) => Math.max(1, n - 1))} disabled={!canGoPrev}>上一页</button>
        <button on:click={() => page.update((n) => Math.min(totalPages, n + 1))} disabled={!canGoNext}>下一页</button>
        <span class="pagination-summary">
          <strong>第 {currentPageNo} / {totalPages} 页，共 {totalRecords} 条</strong>
          <span>当前页显示 {$records.length} 条（每页 {currentPageSize} 条）</span>
        </span>
      </div>
    </div>

    {#if isKeywordPanelOpen}
      <div class="keyword-panel">
        <div class="panel-row">
          <label>
            已保存规则
            <select bind:value={selectedRuleName}>
              <option value="" disabled>请选择规则</option>
              {#each namedRules as rule}
                <option value={rule.name}>{rule.name}</option>
              {/each}
            </select>
          </label>
          <button on:click={applySelectedRule} disabled={!selectedRuleName}>
            应用规则
          </button>
          <button type="button" on:click={() => openKeywordEditor("edit")} disabled={!selectedRuleName}>
            修改规则
          </button>
          <button type="button" on:click={() => openKeywordEditor("add")}>添加规则</button>
        </div>
        <p>{keywordStatus}</p>
      </div>
    {/if}

    {#if isLoadingRecords}
      <div class="list-loading">
        <div class="spinner"></div>
        <p>正在加载列表数据，请稍候...</p>
      </div>
    {:else if $records.length === 0}
      <div class="empty-state">
        <p>当前暂无拉取结果。</p>
        <p>请先启动拉取任务或调整筛选条件后刷新。</p>
      </div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>标题</th>
            <th>地区</th>
            <th>ID</th>
            <th>详情状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {#each $records as record}
            <tr>
              <td>{record.title}</td>
              <td>{record.regionCode}</td>
              <td>{record.sourceId}</td>
              <td>{record.detailStatus}</td>
              <td>
                <button type="button" on:click={() => goDetail(record.sourceId)}>查看详情</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

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

<sl-dialog
  class="start-pull-confirm-dialog"
  label={startPullConfirmTitle}
  bind:this={startPullConfirmDialog}
  on:sl-after-hide={() => resolveStartPullConfirm(false)}
>
  <p>{startPullConfirmMessage}</p>
  <div slot="footer" class="dialog-footer-actions">
    <button type="button" on:click={() => startPullConfirmDialog?.hide()}>继续当前任务</button>
    <button
      type="button"
      on:click={() => {
        resolveStartPullConfirm(true);
        startPullConfirmDialog?.hide();
      }}
    >
      {startPullConfirmActionLabel}
    </button>
  </div>
</sl-dialog>

<sl-dialog
  class="state-warning-dialog"
  label="筛选提示"
  bind:this={stateWarningDialog}
>
  <p>{stateWarningMessage}</p>
  <div slot="footer" class="dialog-footer-actions">
    <button type="button" on:click={() => stateWarningDialog?.hide()}>我知道了</button>
  </div>
</sl-dialog>

<sl-dialog
  class="task-history-dialog"
  label="任务记录"
  bind:this={taskHistoryDialog}
>
  <div class="task-history">
    {#if taskHistory.length === 0}
      <p class="task-history-empty">暂无历史任务，启动拉取后会自动记录。</p>
    {:else}
      <ul class="task-history-list">
        {#each taskHistory as item}
          <li class="task-history-item">
            <div class="task-history-item-header">
              <strong>{item.jobId}</strong>
              <button type="button" on:click={() => useHistoryJob(item.jobId)}>设为当前任务</button>
            </div>
            <p>进度：{item.progressText}</p>
            <p class="summary">筛选：{item.filterSummary}</p>
            <p>快照：当前页记录 {item.snapshotCount} 条（示例ID：{item.snapshotSourceIds.slice(0, 3).join(", ") || "无"}）</p>
            <p>过期时间：{item.expiresAt ? formatDateTime(item.expiresAt) : "待任务数据加载后确定"}</p>
            <p>启动时间：{formatDateTime(item.startedAt)}；最近更新：{formatDateTime(item.updatedAt)}</p>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</sl-dialog>

<sl-dialog
  class="embedded-browser-dialog"
  label={embeddedBrowserTitle}
  bind:this={embeddedBrowserDialog}
  on:sl-after-show={() => (isEmbeddedBrowserOpen = true)}
  on:sl-after-hide={() => {
    isEmbeddedBrowserOpen = false;
    embeddedBrowserHtml = "";
    embeddedBrowserError = "";
    embeddedBrowserSourceUrl = "";
    embeddedBrowserTitle = "";
  }}
>
  {#if embeddedBrowserLoading}
    <p>正在加载内嵌页面，请稍候...</p>
  {:else if embeddedBrowserError}
    <p class="error">{embeddedBrowserError}</p>
    {#if embeddedBrowserSourceUrl}
      <p>
        <a href={embeddedBrowserSourceUrl} target="_blank" rel="noreferrer">打开外部浏览器查看原始页面</a>
      </p>
    {/if}
  {:else}
    <div class="embedded-browser-toolbar">
      <div>源地址：<a href={embeddedBrowserSourceUrl} target="_blank" rel="noreferrer">{embeddedBrowserSourceUrl}</a></div>
    </div>
    <iframe
      class="embedded-browser-frame"
      title="内嵌详情页"
      srcdoc={embeddedBrowserHtml}
      sandbox="allow-same-origin allow-scripts allow-forms allow-popups"
    ></iframe>
  {/if}
</sl-dialog>
