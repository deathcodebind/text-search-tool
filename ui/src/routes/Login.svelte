<script lang="ts">
  import { loginStore } from "../lib/stores";
  import { loginSubmit } from "../lib/api";
  import { onDestroy, onMount } from "svelte";

  let baseUrl = "https://login.jxemall.com";
  let username = "";
  let password = "";
  let remember = true;
  let status = "";
  let loginState = { baseUrl: "", username: "", remember: false };

  const subscription = loginStore.subscribe((value) => {
    baseUrl = value.baseUrl;
    username = value.username;
    remember = value.remember;
    loginState = value;
  });

  onMount(() => {
    if (loginState.username) {
      window.location.hash = "#/pull";
    }
  });

  async function submit() {
    try {
      status = "登录中...";
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

  button {
    width: fit-content;
  }
</style>

<h2>登录</h2>
<form on:submit|preventDefault={submit}>
  <label>
    登录 Base URL
    <input bind:value={baseUrl} placeholder="https://login.jxemall.com" />
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
