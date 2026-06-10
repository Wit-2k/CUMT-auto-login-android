<script setup lang="ts">
import { ref, onMounted } from "vue";
import LoginPage from "./components/LoginPage.vue";
import StatusPage from "./components/StatusPage.vue";
import { invoke } from "@tauri-apps/api/core";

type Page = "login" | "status";

type LoginConfig = {
  username: string;
  password: string;
  operator: string;
};

const currentPage = ref<Page>("login");
const statusText = ref("");
const isSuccess = ref(false);
let latestLoginToken = 0;
let activeLogin:
  | {
      key: string;
      promise: Promise<void>;
    }
  | null = null;

function showStatus(text: string, success: boolean) {
  statusText.value = text;
  isSuccess.value = success;
  currentPage.value = "status";
}

function showLogin() {
  currentPage.value = "login";
}

function isSameConfig(
  config: LoginConfig | null,
  username: string,
  password: string,
  operator: string
) {
  return (
    config?.username === username &&
    config.password === password &&
    config.operator === operator
  );
}

function normalizeLoginConfig(
  username: string,
  password: string,
  operator: string
): LoginConfig {
  return {
    username: username.trim(),
    password,
    operator,
  };
}

function loginKey(config: LoginConfig) {
  return JSON.stringify([config.username, config.password, config.operator]);
}

async function skipDuplicateLoginIfOnline(
  username: string,
  password: string,
  operator: string
) {
  try {
    const config = await invoke<LoginConfig | null>("load_config");
    if (!isSameConfig(config, username, password, operator)) {
      return false;
    }

    const online = await invoke<boolean>("check_online");
    if (!online) {
      return false;
    }

    return true;
  } catch {
    return false;
  }
}

onMounted(async () => {
  try {
    const online = await invoke<boolean>("check_online");
    if (online) {
      showStatus("✅ 已连接到互联网，无需登录。", true);
      return;
    }
    // 未联网，尝试自动登录
    const config = await invoke<LoginConfig | null>("load_config");
    if (config && config.username && config.password && config.operator) {
      await doLogin(config.username, config.password, config.operator, {
        skipDuplicateWhenOnline: false,
      });
    }
    // 如果没有配置，停留在登录页面
  } catch {
    // 启动检查失败，停留在登录页面
  }
});

async function doLogin(
  username: string,
  password: string,
  operator: string,
  options: { skipDuplicateWhenOnline?: boolean } = {
    skipDuplicateWhenOnline: true,
  }
) {
  const config = normalizeLoginConfig(username, password, operator);
  if (!config.username || !config.password) {
    showStatus("账号或密码不能为空！", false);
    return;
  }

  const key = loginKey(config);
  if (activeLogin?.key === key) {
    await activeLogin.promise;
    return;
  }

  const token = ++latestLoginToken;
  const promise = runLogin(config, options, token);
  activeLogin = { key, promise };

  try {
    await promise;
  } finally {
    if (activeLogin?.promise === promise) {
      activeLogin = null;
    }
  }
}

function isLatestLogin(token: number) {
  return token === latestLoginToken;
}

async function runLogin(
  { username, password, operator }: LoginConfig,
  options: { skipDuplicateWhenOnline?: boolean },
  token: number
) {
  try {
    if (
      options.skipDuplicateWhenOnline &&
      (await skipDuplicateLoginIfOnline(username, password, operator))
    ) {
      if (isLatestLogin(token)) {
        showStatus("✅ 已连接到互联网，无需重复登录。", true);
      }
      return;
    }

    const [success, message] = await invoke<[boolean, string]>(
      "perform_login",
      { username, password, operator }
    );
    if (!isLatestLogin(token)) {
      return;
    }
    if (success) {
      // 登录成功后保存配置
      await invoke("save_config", {
        config: { username, password, operator },
      });
      if (!isLatestLogin(token)) {
        return;
      }
      showStatus(`✅ 登录成功！\n服务器消息：${message}`, true);
    } else {
      showStatus(`❌ 登录失败！\n服务器消息：${message}`, false);
    }
  } catch (e) {
    if (isLatestLogin(token)) {
      showStatus(`❌ 登录异常：${e}`, false);
    }
  }
}
</script>

<template>
  <div class="w-full flex items-center justify-center">
    <LoginPage
      v-if="currentPage === 'login'"
      @login="doLogin"
    />
    <StatusPage
      v-else
      :status-text="statusText"
      :is-success="isSuccess"
      @back="showLogin"
    />
  </div>
</template>
