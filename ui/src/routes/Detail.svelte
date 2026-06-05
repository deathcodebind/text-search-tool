<script lang="ts">
  import { downloadAttachment, fetchDetailPageHtml, openExternalUrl, pullRecordDetail, pullRecordStatus, pullRetryDetail } from "../lib/api";
  import { params as routeParams } from "svelte-spa-router";

  export let params: { sourceId?: string } | null | undefined = undefined;

  let sourceId = "";
  let detail: any = null;
  let status = "";
  let embeddedSourceUrl = "";
  let sourceUrlLoading = false;
  let sourceUrlError = "";
  let openingExternal = false;
  let attachmentStatus = "";
  let downloadingUrl = "";
  let detailLoading = false;
  let resolvedSourceUrl = "";

  function ensureDetailTypeForBrowser(url: string) {
    const trimmed = (url || "").trim();
    if (!trimmed) {
      return "";
    }

    try {
      const parsed = new URL(trimmed);
      const requisitionId = (parsed.searchParams.get("requisitionId") || "").trim();
      if (!requisitionId) {
        return trimmed;
      }

      const type = (parsed.searchParams.get("type") || "BIDDING_INVITATION").trim() || "BIDDING_INVITATION";
      const rebuilt = new URL(`${parsed.origin}${parsed.pathname}`);

      // Put `type` first so it is never the trailing parameter.
      rebuilt.searchParams.set("type", type);
      rebuilt.searchParams.set("requisitionId", requisitionId);

      for (const [key, value] of parsed.searchParams.entries()) {
        if (key === "type" || key === "requisitionId") {
          continue;
        }
        rebuilt.searchParams.append(key, value);
      }

      rebuilt.hash = parsed.hash;
      return rebuilt.toString();
    } catch {
      // Fallback for non-standard URLs: at least ensure type exists.
      const hasRequisitionId = /[?&]requisitionId=/i.test(trimmed);
      const hasType = /[?&]type=/i.test(trimmed);
      if (!hasRequisitionId || hasType) {
        return trimmed;
      }
      const [base, fragment] = trimmed.split("#", 2);
      const separator = base.includes("?") ? "&" : "?";
      const next = `${base}${separator}type=BIDDING_INVITATION`;
      return fragment ? `${next}#${fragment}` : next;
    }
  }

  async function wait(ms: number) {
    await new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function pollDetailAfterRetry(maxAttempts = 40) {
    for (let i = 0; i < maxAttempts; i += 1) {
      await wait(1200);
      try {
        const state: any = await pullRecordStatus(sourceId);
        const stateText = String(state?.status || "").toLowerCase();
        const attempts = Number(state?.attempts || 0);

        if (state?.hasDetail || stateText === "succeeded") {
          const nextDetail = await pullRecordDetail(sourceId);
          detail = nextDetail;
          status = "详情已加载";
          return true;
        }

        if (stateText === "failed" || stateText === "timeout" || stateText === "canceled") {
          status = `详情补拉失败：${state?.message || stateText}`;
          return false;
        }

        const progressHint = attempts > 0 ? `（重试 ${attempts} 次）` : "";
        status = `详情补拉中：${state?.status || "queued"}${progressHint}`;
      } catch {
        // keep polling on transient errors
      }
    }
    return false;
  }

  function attachmentNameFromUrl(url: string, index: number) {
    try {
      const u = new URL(url);
      const path = u.pathname || "";
      const last = path.split("/").filter(Boolean).pop();
      if (last) {
        return decodeURIComponent(last);
      }
    } catch {
      // keep fallback name when url parsing fails
    }
    return `附件 ${index + 1}`;
  }

  $: sourceId = params?.sourceId || ($routeParams as { sourceId?: string } | null)?.sourceId || "";

  async function loadSourcePageUrl() {
    if (!sourceId) {
      return;
    }
    sourceUrlLoading = true;
    sourceUrlError = "";
    try {
      const result: any = await fetchDetailPageHtml(sourceId);
      embeddedSourceUrl = result?.sourcePageUrl || "";
      if (!embeddedSourceUrl) {
        sourceUrlError = "未获取到有效页面地址";
        return;
      }
    } catch (error) {
      sourceUrlError = `页面地址获取失败：${error}`;
    } finally {
      sourceUrlLoading = false;
    }
  }

  async function openInSystemBrowser() {
    if (!resolvedSourceUrl && !embeddedSourceUrl) {
      await loadSourcePageUrl();
    }
    const finalUrl = ensureDetailTypeForBrowser(detail?.sourcePageUrl || embeddedSourceUrl);
    if (!finalUrl) {
      return;
    }
    embeddedSourceUrl = finalUrl;
    openingExternal = true;
    try {
      await openExternalUrl(finalUrl);
      status = "已调用系统浏览器打开原页面，请在浏览器完成登录后查看。";
    } catch (error) {
      status = `打开系统浏览器失败：${error}`;
    } finally {
      openingExternal = false;
    }
  }

  async function handleDownloadAttachment(url: string) {
    if (!sourceId || !url) {
      return;
    }
    downloadingUrl = url;
    attachmentStatus = "附件下载中...";
    try {
      const result: any = await downloadAttachment(sourceId, url);
      attachmentStatus = `附件已下载：${result?.filePath || "本地文件"}`;
    } catch (error) {
      attachmentStatus = `附件下载失败：${error}`;
    } finally {
      downloadingUrl = "";
    }
  }

  async function loadDetail() {
    if (!sourceId) {
      status = "未指定 sourceId";
      return;
    }
    detailLoading = true;
    try {
      detail = await pullRecordDetail(sourceId);
      status = "详情已加载";
    } catch (error) {
      const message = String(error);
      if (message.includes("缺少会话 cookie")) {
        window.location.hash = "#/login";
        return;
      }
      if (message.includes("detail not found")) {
        status = "详情未入库，正在尝试补拉详情内容...";
        try {
          await pullRetryDetail(sourceId);
          const loaded = await pollDetailAfterRetry();
          if (!loaded) {
            status = "详情补拉仍在进行中，请稍后重试；可先在系统浏览器查看原页面。";
          }
        } catch (retryError) {
          status = `详情补拉失败：${retryError}`;
        }
      } else {
        status = `详情未入库，可直接打开系统浏览器查看：${message}`;
      }
    } finally {
      detailLoading = false;
      if (detail?.sourcePageUrl) {
        embeddedSourceUrl = detail.sourcePageUrl;
      } else {
        await loadSourcePageUrl();
      }
    }
  }

  $: if (sourceId) {
    loadDetail();
  }

  $: resolvedSourceUrl = ensureDetailTypeForBrowser(detail?.sourcePageUrl || embeddedSourceUrl);
</script>

<style>
  :global(body) {
    padding-top: 64px;
  }

  .fixed-back {
    position: fixed;
    top: 12px;
    left: 12px;
    z-index: 1000;
    border: none;
    border-radius: 999px;
    background: #0f172a;
    color: #ffffff;
    padding: 10px 14px;
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.2);
    cursor: pointer;
  }

  .fixed-back:hover {
    background: #1e293b;
  }
  .detail-card {
    background: #ffffff;
    border-radius: 12px;
    padding: 18px;
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.08);
  }

  pre {
    white-space: pre-wrap;
    word-break: break-word;
    background: #f8fafc;
    padding: 16px;
    border-radius: 8px;
  }

  .embedded-section {
    margin-top: 14px;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 10px;
    background: #f8fafc;
  }

  .attachment-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .hint {
    color: #64748b;
    font-size: 0.9rem;
  }
</style>

<h2>详情页</h2>
<button class="fixed-back" on:click={() => (window.location.hash = "#/pull")}>← 返回拉取页</button>
<p>{status}</p>
{#if detailLoading}
  <p class="hint">详情补拉中，请稍候...</p>
{/if}
{#if detail}
  <div class="detail-card">
    <p><strong>记录 ID:</strong> {detail.sourceId}</p>
    <p><strong>标题:</strong> {detail.title || "（无标题）"}</p>
    <p>
      <strong>源页面:</strong>
      {#if resolvedSourceUrl}
        <a href={resolvedSourceUrl} target="_blank" rel="noreferrer">{resolvedSourceUrl}</a>
      {:else}
        （无）
      {/if}
    </p>
    <p><strong>详情文本:</strong></p>
    <pre>{detail.detailText}</pre>
    <p><strong>原始 JSON:</strong></p>
    <pre>{detail.rawJson}</pre>
    <p><strong>附件:</strong></p>
    <ul>
      {#each detail.attachmentUrls || [] as url, index}
        <li>
          <div class="attachment-row">
            <a href={url} target="_blank" rel="noreferrer" title={url}>
              {attachmentNameFromUrl(url, index)}
            </a>
            <button
              type="button"
              disabled={downloadingUrl === url}
              on:click={() => handleDownloadAttachment(url)}
            >
              {downloadingUrl === url ? "下载中..." : "下载附件"}
            </button>
          </div>
        </li>
      {/each}
    </ul>
    {#if attachmentStatus}
      <p class="hint">{attachmentStatus}</p>
    {/if}
  </div>
{/if}

<div class="detail-card embedded-section">
  <p><strong>原页面访问</strong></p>
  {#if resolvedSourceUrl}
    <p class="hint">源地址：<a href={resolvedSourceUrl} target="_blank" rel="noreferrer">{resolvedSourceUrl}</a></p>
  {/if}
  <p class="hint">提示：该站点禁止被内嵌显示，请使用系统浏览器访问并手动登录。</p>
  <button type="button" disabled={openingExternal || sourceUrlLoading} on:click={openInSystemBrowser}>
    {#if sourceUrlLoading}
      获取地址中...
    {:else if openingExternal}
      打开中...
    {:else}
      在系统浏览器打开
    {/if}
  </button>
  {#if sourceUrlError}
    <p>{sourceUrlError}</p>
  {/if}
</div>
