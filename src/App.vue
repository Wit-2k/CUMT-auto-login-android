<script setup lang="ts">
import { ref, onMounted } from "vue";
import LoginPage from "./components/LoginPage.vue";
import StatusPage from "./components/StatusPage.vue";
import { invoke } from "@tauri-apps/api/core";

type Page = "login" | "status";

const currentPage = ref<Page>("login");
const statusText = ref("");
const isSuccess = ref(false);

function showStatus(text: string, success: boolean) {
  statusText.value = text;
  isSuccess.value = success;
  currentPage.value = "status";
}

function showLogin() {
  currentPage.value = "login";
}

onMounted(async () => {
  try {
    const online = await invoke<boolean>("check_online");
    if (online) {
      showStatus("✅ 已连接到互联网，无需登录。", true);
      return;
    }
    // 未联网，尝试自动登录
    const config = await invoke<{
      username: string;
      password: string;
      operator: string;
    } | null>("load_config");
    if (config && config.username && config.password && config.operator) {
      await doLogin(config.username, config.password, config.operator);
    }
    // 如果没有配置，停留在登录页面
  } catch {
    // 启动检查失败，停留在登录页面
  }
});

async function doLogin(
  username: string,
  password: string,
  operator: string
) {
  if (!username || !password) {
    showStatus("账号或密码不能为空！", false);
    return;
  }
  try {
    const [success, message] = await invoke<[boolean, string]>(
      "perform_login",
      { username, password, operator }
    );
    if (success) {
      // 登录成功后保存配置
      await invoke("save_config", {
        config: { username, password, operator },
      });
      showStatus(`✅ 登录成功！\n服务器消息：${message}`, true);
    } else {
      showStatus(`❌ 登录失败！\n服务器消息：${message}`, false);
    }
  } catch (e) {
    showStatus(`❌ 登录异常：${e}`, false);
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
