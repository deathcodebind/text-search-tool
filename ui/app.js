import { JXEMALL_DISTRICT_OPTIONS } from "./jxemall-district-options.js";

const invoke = window.__TAURI_INTERNALS__?.invoke;
const LOGIN_MEMORY_KEY = "text-search-tool.login-memory.v1";
const KEYWORD_GROUPS_KEY = "text-search-tool.keyword-groups.v1";

const selectedDistricts = new Map();
let visibleDistrictCodes = [];
let currentPullPage = 1;
const pullPageSize = 10;

function splitTerms(value) {
  return value
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);
}

function renderHits(resp) {
  const meta = document.getElementById("resultMeta");
  const list = document.getElementById("resultList");
  meta.textContent = `命中 ${resp.total} 条`;
  list.innerHTML = "";

  if (!resp.hits.length) {
    list.innerHTML = "<li>无命中结果</li>";
    return;
  }

  for (const hit of resp.hits) {
    const li = document.createElement("li");
    li.innerHTML = `
      <p class="hit-title">${hit.title}</p>
      <p class="hit-meta">ID: ${hit.sourceId} | 地区: ${hit.regionCode} | 分数: ${hit.score}</p>
      <p class="hit-meta">${hit.snippet || "无摘要"}</p>
    `;
    list.appendChild(li);
  }
}

function setText(id, value) {
  document.getElementById(id).textContent = value;
}

function statePresetToList(preset) {
  if (preset === "not-started") {
    return [3];
  }
  if (preset === "in-progress") {
    return [4];
  }
  if (preset === "expired") {
    return [5, 6, 7, 10, 12, 50];
  }
  return [3, 4, 5, 6, 7, 10, 12, 50];
}

function formatUnixTs(ts) {
  if (!Number.isFinite(ts)) {
    return "-";
  }
  return new Date(ts * 1000).toLocaleString("zh-CN", { hour12: false });
}

function createKeywordGroupRow(group) {
  const tr = document.createElement("tr");
  tr.innerHTML = `
    <td>
      <select class="group-occur">
        <option value="must" ${group.occur === "must" ? "selected" : ""}>must</option>
        <option value="should" ${group.occur === "should" ? "selected" : ""}>should</option>
        <option value="mustNot" ${group.occur === "mustNot" ? "selected" : ""}>mustNot</option>
      </select>
    </td>
    <td><input class="group-terms" type="text" value="${(group.terms || []).join(",")}" /></td>
    <td><input class="group-msm" type="number" min="1" value="${group.minimumShouldMatch || 1}" /></td>
  `;
  return tr;
}

function collectKeywordGroups() {
  const occurEls = Array.from(document.querySelectorAll(".group-occur"));
  const termEls = Array.from(document.querySelectorAll(".group-terms"));
  const msmEls = Array.from(document.querySelectorAll(".group-msm"));

  return occurEls
    .map((occurEl, idx) => ({
      occur: occurEl.value,
      terms: splitTerms(termEls[idx]?.value || ""),
      minimumShouldMatch: Number(msmEls[idx]?.value || "1"),
    }))
    .filter((x) => x.terms.length > 0);
}

function saveKeywordGroups() {
  const payload = {
    rootMinimumShouldMatch: Number(document.getElementById("rootMsm").value || "0"),
    groups: collectKeywordGroups(),
  };
  localStorage.setItem(KEYWORD_GROUPS_KEY, JSON.stringify(payload));
  setText("errorText", "关键词配置已保存");
}

function loadKeywordGroups() {
  const raw = localStorage.getItem(KEYWORD_GROUPS_KEY);
  if (!raw) {
    setText("errorText", "没有已保存的关键词配置");
    return;
  }

  const parsed = JSON.parse(raw);
  document.getElementById("rootMsm").value = String(parsed.rootMinimumShouldMatch || 0);
  const tbody = document.querySelector(".group-table tbody");
  tbody.innerHTML = "";
  for (const group of parsed.groups || []) {
    tbody.appendChild(createKeywordGroupRow(group));
  }
  setText("errorText", "关键词配置已加载");
}

function addKeywordGroup() {
  const tbody = document.querySelector(".group-table tbody");
  tbody.appendChild(
    createKeywordGroupRow({ occur: "should", terms: [""], minimumShouldMatch: 1 }),
  );
}

function removeKeywordGroup() {
  const tbody = document.querySelector(".group-table tbody");
  if (tbody.children.length <= 1) {
    setText("errorText", "至少保留一组关键词");
    return;
  }
  tbody.removeChild(tbody.lastElementChild);
}

function renderPulledRecords(resp) {
  const meta = document.getElementById("pullResultMeta");
  const list = document.getElementById("pullResultList");
  list.innerHTML = "";
  document.getElementById("pullPageMeta").textContent = `第 ${resp.page} 页 / 每页 ${resp.pageSize} 条`;

  meta.textContent = `拉取数据：数据库共 ${resp.total} 条，当前展示 ${resp.records.length} 条`;
  if (!resp.records.length) {
    list.innerHTML = "<li>暂无拉取结果</li>";
    return;
  }

  for (const record of resp.records) {
    const canRetry = record.detailStatus === "failed" || record.detailStatus === "timeout";
    const li = document.createElement("li");
    li.innerHTML = `
      <p class="hit-title">${record.title}</p>
      <p class="hit-meta">ID: ${record.sourceId} | 区划: ${record.regionCode}</p>
      <p class="hit-meta">发布时间: ${formatUnixTs(record.publishedAt)} | 过期时间: ${formatUnixTs(record.expiresAt)}</p>
      <p class="hit-meta">链接: ${record.sourceUrl}</p>
      <div class="actions">
        <button class="secondary detail-open-btn" data-source-id="${record.sourceId}">查看详情</button>
        <span class="meta">详情状态: ${record.detailStatus} | 重试次数(不含首次): ${record.detailAttempts}</span>
        ${canRetry ? `<button class="secondary detail-retry-btn" data-source-id="${record.sourceId}">重新拉取详情</button>` : ""}
      </div>
      ${record.detailMessage ? `<p class="hit-meta">状态信息: ${record.detailMessage}</p>` : ""}
      <p class="hit-meta">状态更新时间: ${record.detailUpdatedAt ? formatUnixTs(record.detailUpdatedAt) : "-"}</p>
    `;
    list.appendChild(li);
  }

  for (const btn of list.querySelectorAll(".detail-open-btn")) {
    btn.addEventListener("click", () => openDetailPage(btn.dataset.sourceId));
  }

  for (const btn of list.querySelectorAll(".detail-retry-btn")) {
    btn.addEventListener("click", async () => {
      try {
        const result = await invoke("pull_retry_detail", { sourceId: btn.dataset.sourceId });
        setText("pullStatus", result.message);
        await loadPullRecords();
      } catch (e) {
        setText("pullStatus", `详情重试失败: ${typeof e === "string" ? e : "未知错误"}`);
      }
    });
  }
}

function setActiveStep(step) {
  for (const btn of document.querySelectorAll(".step-btn")) {
    btn.classList.toggle("active", btn.dataset.step === step);
  }
  for (const page of document.querySelectorAll(".step-page")) {
    page.classList.toggle("active", page.dataset.page === step);
  }
}

function bindStepNavigation() {
  for (const btn of document.querySelectorAll(".step-btn")) {
    btn.addEventListener("click", () => setActiveStep(btn.dataset.step));
  }
  document.getElementById("goPullBtn").addEventListener("click", () => setActiveStep("pull"));
  document.getElementById("goKeywordBtn").addEventListener("click", () => setActiveStep("keyword"));
  document.getElementById("goLoginBtn").addEventListener("click", () => setActiveStep("login"));
  document.getElementById("goPullBackBtn").addEventListener("click", () => setActiveStep("pull"));
  document.getElementById("detailBackToPullBtn").addEventListener("click", () => setActiveStep("pull"));
}

function renderSelectedRegions() {
  const container = document.getElementById("selectedRegions");
  container.innerHTML = "";

  if (!selectedDistricts.size) {
    setText("regionSummary", "已选地区：暂无");
    return;
  }

  setText("regionSummary", `已选地区：${selectedDistricts.size} 个`);
  for (const [code, label] of selectedDistricts.entries()) {
    const chip = document.createElement("span");
    chip.className = "region-chip";
    chip.innerHTML = `${label} <button type="button" data-code="${code}">x</button>`;
    container.appendChild(chip);
  }

  for (const removeBtn of container.querySelectorAll("button[data-code]")) {
    removeBtn.addEventListener("click", () => {
      selectedDistricts.delete(removeBtn.dataset.code);
      renderSelectedRegions();
      renderDistrictOptions();
    });
  }
}

function renderDistrictOptions() {
  const keyword = document.getElementById("regionKeyword").value.trim().toLowerCase();
  const filtered = keyword
    ? JXEMALL_DISTRICT_OPTIONS.filter(
        (x) => x.code.includes(keyword) || x.name.toLowerCase().includes(keyword),
      )
    : JXEMALL_DISTRICT_OPTIONS;

  visibleDistrictCodes = filtered.map((x) => x.code);
  const container = document.getElementById("regionDistrictList");
  container.innerHTML = "";

  if (!filtered.length) {
    container.innerHTML = '<p class="meta">没有匹配到地区，请调整关键字</p>';
    return;
  }

  for (const district of filtered) {
    const item = document.createElement("label");
    item.className = "district-item";
    item.innerHTML = `
      <input type="checkbox" value="${district.code}" data-name="${district.name}" ${selectedDistricts.has(district.code) ? "checked" : ""} />
      <span>${district.name} (${district.code})</span>
    `;
    container.appendChild(item);
  }

  for (const checkbox of container.querySelectorAll("input[type='checkbox']")) {
    checkbox.addEventListener("change", () => {
      const code = checkbox.value;
      const name = checkbox.dataset.name || code;
      if (checkbox.checked) {
        selectedDistricts.set(code, `${name} (${code})`);
      } else {
        selectedDistricts.delete(code);
      }
      renderSelectedRegions();
    });
  }
}

function initRegionPicker() {
  document.getElementById("regionKeyword").addEventListener("input", renderDistrictOptions);

  document.getElementById("addRegionBtn").addEventListener("click", () => {
    for (const code of visibleDistrictCodes) {
      const option = JXEMALL_DISTRICT_OPTIONS.find((x) => x.code === code);
      if (option) {
        selectedDistricts.set(code, `${option.name} (${code})`);
      }
    }
    renderSelectedRegions();
    renderDistrictOptions();
  });

  document.getElementById("clearRegionBtn").addEventListener("click", () => {
    selectedDistricts.clear();
    renderSelectedRegions();
    renderDistrictOptions();
  });

  selectedDistricts.set("360103", "西湖区 (360103)");
  selectedDistricts.set("360111", "青山湖区 (360111)");
  selectedDistricts.set("369900", "江西省本级 (369900)");
  selectedDistricts.set("980701", "江西培训市 (980701)");
  renderSelectedRegions();
  renderDistrictOptions();
}

function loadLoginMemory() {
  try {
    const raw = localStorage.getItem(LOGIN_MEMORY_KEY);
    if (!raw) {
      return;
    }
    const data = JSON.parse(raw);
    if (data.baseUrl) {
      document.getElementById("loginBaseUrl").value = data.baseUrl;
    }
    if (data.username) {
      document.getElementById("loginUsername").value = data.username;
      setText("lastLoginHint", `上次登录账号：${data.username}`);
    }
  } catch {
    setText("lastLoginHint", "历史账号读取失败");
  }
}

function saveLoginMemory(baseUrl, username) {
  const remember = document.getElementById("rememberLogin").checked;
  if (!remember) {
    localStorage.removeItem(LOGIN_MEMORY_KEY);
    setText("lastLoginHint", "已清除历史账号记录");
    return;
  }

  const payload = { baseUrl, username };
  localStorage.setItem(LOGIN_MEMORY_KEY, JSON.stringify(payload));
  setText("lastLoginHint", `上次登录账号：${username || "未填写"}`);
}

async function loadSnapshot() {
  if (!invoke) {
    document.getElementById("errorText").textContent = "未检测到 Tauri 运行环境";
    return;
  }

  const snapshot = await invoke("app_snapshot");
  setText("appName", snapshot.appName);
  setText("appVersion", snapshot.version);
  setText("capabilities", snapshot.capabilities.join(" / "));
}

async function loginSubmit() {
  try {
    const baseUrl = document.getElementById("loginBaseUrl").value.trim();
    const username = document.getElementById("loginUsername").value.trim();
    const password = document.getElementById("loginPassword").value.trim();

    const result = await invoke("login_submit", {
      input: {
        baseUrl,
        username,
        password,
        cookieHeader: null,
      },
    });

    saveLoginMemory(baseUrl, username);
    setText("loginStatus", `登录成功: ${result.credentialRef}`);
    setActiveStep("pull");
  } catch (e) {
    setText("loginStatus", `登录失败: ${typeof e === "string" ? e : "未知错误"}`);
  }
}

let currentPullJobId = "";

async function loadPullRecords() {
  try {
    const resp = await invoke("pull_records", {
      input: { page: currentPullPage, pageSize: pullPageSize },
    });
    renderPulledRecords(resp);
  } catch (e) {
    setText("pullResultMeta", `拉取数据读取失败: ${typeof e === "string" ? e : "未知错误"}`);
    document.getElementById("pullResultList").innerHTML = "";
  }
}

async function openDetailPage(sourceId) {
  try {
    const detail = await invoke("pull_record_detail", { sourceId });
    document.getElementById("detailMeta").textContent = `详情已加载: ${detail.sourceId}`;
    document.getElementById("detailRawJson").textContent = detail.rawJson || "暂无原始数据";

    const sourceLink = document.getElementById("detailSourceLink");
    sourceLink.href = detail.sourcePageUrl || "#";

    const attachmentList = document.getElementById("detailAttachmentList");
    attachmentList.innerHTML = "";
    if (!detail.attachmentUrls || !detail.attachmentUrls.length) {
      attachmentList.innerHTML = "<li>暂无附件</li>";
    } else {
      for (const url of detail.attachmentUrls) {
        const li = document.createElement("li");
        li.innerHTML = `<a href="${url}" target="_blank" rel="noreferrer">下载附件</a>`;
        attachmentList.appendChild(li);
      }
    }

    setActiveStep("detail");
  } catch (e) {
    setText("detailMeta", `详情加载失败: ${typeof e === "string" ? e : "未知错误"}`);
    setActiveStep("detail");
  }
}

async function pullStart() {
  try {
    const minBudgetRaw = document.getElementById("pullMinBudget").value.trim();
    const maxBudgetRaw = document.getElementById("pullMaxBudget").value.trim();

    const result = await invoke("pull_start", {
      input: {
        districtCodes: Array.from(selectedDistricts.keys()),
        categoryType: document.getElementById("pullCategoryType").value || null,
        stateList: statePresetToList(document.getElementById("pullStatePreset").value),
        instanceCodes: splitTerms(document.getElementById("pullInstances").value),
        minBudget: minBudgetRaw ? Number(minBudgetRaw) : null,
        maxBudget: maxBudgetRaw ? Number(maxBudgetRaw) : null,
        sortField: document.getElementById("pullSortField").value,
        sortMethod: document.getElementById("pullSortMethod").value,
        keywordHint: document.getElementById("pullKeyword").value.trim() || null,
      },
    });
    currentPullJobId = result.jobId;
    currentPullPage = 1;
    document.getElementById("currentJobId").value = currentPullJobId;
    setText("pullStatus", `任务已启动: ${currentPullJobId}`);
    await loadPullRecords();
  } catch (e) {
    setText("pullStatus", `启动失败: ${typeof e === "string" ? e : "未知错误"}`);
  }
}

async function pullProgress() {
  if (!currentPullJobId) {
    setText("pullStatus", "请先启动拉取任务");
    return;
  }

  try {
    const result = await invoke("pull_progress", { jobId: currentPullJobId });
    setText(
      "pullStatus",
      `状态: ${result.status}, 进度: ${result.processed}/${result.total}, ${result.message}`,
    );
    if (result.status === "succeeded") {
      await loadPullRecords();
    }
  } catch (e) {
    setText("pullStatus", `查询失败: ${typeof e === "string" ? e : "未知错误"}`);
  }
}

async function previewBoolQuery() {
  const error = document.getElementById("errorText");
  error.textContent = "";

  try {
    const groups = collectKeywordGroups();

    const resp = await invoke("preview_keyword_groups", {
      input: {
        groups,
        rootMinimumShouldMatch: Number(document.getElementById("rootMsm").value || "0"),
        page: 1,
        pageSize: 20,
      },
    });
    renderHits(resp);
  } catch (e) {
    error.textContent = typeof e === "string" ? e : "预览失败";
  }
}

document.getElementById("loginBtn").addEventListener("click", loginSubmit);
document.getElementById("pullBtn").addEventListener("click", pullStart);
document.getElementById("pullProgressBtn").addEventListener("click", pullProgress);
document.getElementById("pullRecordsBtn").addEventListener("click", loadPullRecords);
document.getElementById("pullPrevPageBtn").addEventListener("click", async () => {
  if (currentPullPage <= 1) {
    return;
  }
  currentPullPage -= 1;
  await loadPullRecords();
});
document.getElementById("pullNextPageBtn").addEventListener("click", async () => {
  currentPullPage += 1;
  await loadPullRecords();
});
document.getElementById("previewBtn").addEventListener("click", previewBoolQuery);
document.getElementById("addKeywordGroupBtn").addEventListener("click", addKeywordGroup);
document.getElementById("removeKeywordGroupBtn").addEventListener("click", removeKeywordGroup);
document.getElementById("saveKeywordGroupsBtn").addEventListener("click", saveKeywordGroups);
document.getElementById("loadKeywordGroupsBtn").addEventListener("click", loadKeywordGroups);
bindStepNavigation();
initRegionPicker();
loadLoginMemory();
loadSnapshot();
