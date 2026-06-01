import { invoke } from "@tauri-apps/api/core";

export interface LoginInput {
  baseUrl: string;
  username: string;
  password: string;
}

export interface PullStartInput {
  districtCodes: string[];
  categoryType?: string | null;
  stateList: number[];
  instanceCodes: string[];
  minBudget?: number | null;
  maxBudget?: number | null;
  sortField: string;
  sortMethod: string;
  keywordHint?: string | null;
}

export function loginSubmit(input: LoginInput) {
  return invoke("login_submit", { input });
}

export function pullStart(input: PullStartInput) {
  return invoke("pull_start", { input });
}

export function pullRecords(page = 1, pageSize = 10) {
  return invoke("pull_records", { input: { page, pageSize } });
}

export function pullProgress(jobId: string) {
  return invoke("pull_progress", { jobId });
}

export function previewKeywordGroups(payload: unknown) {
  return invoke("preview_keyword_groups", { input: payload });
}

export function pullRecordDetail(sourceId: string) {
  return invoke("pull_record_detail", { sourceId });
}

export function fetchDetailPageHtml(sourceId: string) {
  return invoke("fetch_detail_page_html", { sourceId });
}

export function pullRetryDetail(sourceId: string) {
  return invoke("pull_retry_detail", { sourceId });
}

export function downloadAttachment(sourceId: string, url: string) {
  return invoke("download_attachment", { sourceId, url });
}

export function openExternalUrl(url: string) {
  return invoke("open_external_url", { url });
}
