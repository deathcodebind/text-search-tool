<script lang="ts">
  import { loginStore } from "../lib/stores";
  import { loginSubmit } from "../lib/api";
  import { onDestroy } from "svelte";

  const SITE_OPTIONS = [
    {
      key: "jxemall",
      label: "江西省政府采购电子卖场",
      baseUrl: "https://login.jxemall.com",
    },
  ];

  let selectedSite = "jxemall";
  let username = "";
  let password = "";
  let remember = true;
  let status = "";
  let loginState = { baseUrl: "", username: "", remember: false };

  function baseUrlFromSite(siteKey: string) {
    return SITE_OPTIONS.find((item) => item.key === siteKey)?.baseUrl || SITE_OPTIONS[0].baseUrl;
  }

  function siteFromBaseUrl(baseUrl: string) {
    if (baseUrl.includes("jxemall.com")) {
      return "jxemall";
    }
    return "jxemall";
  }

  const subscription = loginStore.subscribe((value) => {
    selectedSite = siteFromBaseUrl(value.baseUrl || "");
    username = value.username;
    remember = value.remember;
    loginState = value;
  });

  async function submit() {
    try {
      status = "登录中...";
      const baseUrl = baseUrlFromSite(selectedSite);
      const result = await loginSubmit({ baseUrl, username, password });
      status = `登录成功：${result}`;
      loginStore.set({ baseUrl, username, remember });
      window.location.hash = "#/pull";
    } catch (error) {
      status = `登录失败：${error}`;
    }
  }

  onDestroy(() => {
    subscription();
  });
</script>

<style>
  form {
    display: grid;
    gap: 14px;
    max-width: 640px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  input {
    padding: 10px 12px;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
  }

  select {
    padding: 10px 12px;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    background: #ffffff;
  }

  button {
    width: fit-content;
  }
</style>

<h2>登录</h2>
<form on:submit|preventDefault={submit}>
  <label>
    网站
    <select bind:value={selectedSite}>
      {#each SITE_OPTIONS as site}
        <option value={site.key}>{site.label}</option>
      {/each}
    </select>
  </label>

  <label>
    用户名
    <input bind:value={username} placeholder="输入用户名" />
  </label>

  <label>
    密码
    <input type="password" bind:value={password} placeholder="输入登录密码" />
  </label>

  <label>
    <input type="checkbox" bind:checked={remember} /> 记住账号信息
  </label>

  <button type="submit">提交登录</button>
  <p>{status}</p>
</form>
