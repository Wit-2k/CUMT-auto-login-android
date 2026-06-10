<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const emit = defineEmits<{
  login: [username: string, password: string, operator: string];
}>();

const username = ref("");
const password = ref("");
const showPassword = ref(false);
const operator = ref("@telecom");
const loading = ref(false);

const operators = [
  { label: "电信", value: "@telecom" },
  { label: "移动", value: "@cmcc" },
  { label: "联通", value: "@unicom" },
];

const operatorReverseMap: Record<string, string> = {
  "@telecom": "电信",
  "@cmcc": "移动",
  "@unicom": "联通",
};

function handleLogin() {
  if (loading.value) return;
  loading.value = true;
  emit("login", username.value, password.value, operator.value);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    handleLogin();
  }
}

onMounted(async () => {
  try {
    const config = await invoke<{
      username: string;
      password: string;
      operator: string;
    } | null>("load_config");
    if (config) {
      username.value = config.username || "";
      password.value = config.password || "";
      if (config.operator && operatorReverseMap[config.operator]) {
        operator.value = config.operator;
      }
    }
  } catch {
    // 加载配置失败，使用默认值
  }
});

// 暴露 resetLoading 方法给父组件
defineExpose({
  resetLoading: () => {
    loading.value = false;
  },
});
</script>

<template>
  <div
    class="bg-white/95 backdrop-blur-sm rounded-2xl shadow-2xl p-6 sm:p-8 w-full max-w-[360px]"
    @keydown="handleKeydown"
  >
    <!-- 标题 -->
    <div class="text-center mb-6">
      <div class="text-3xl mb-2">📶</div>
      <h1 class="text-xl font-bold text-gray-800">校园网自动登录</h1>
      <p class="text-sm text-gray-500 mt-1">中国矿业大学</p>
    </div>

    <!-- 表单 -->
    <div class="space-y-4">
      <!-- 学号 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1"
          >学号</label
        >
        <input
          v-model="username"
          type="text"
          inputmode="numeric"
          placeholder="请输入学号"
          class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all text-sm"
          autofocus
        />
      </div>

      <!-- 密码 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1"
          >密码</label
        >
        <div class="relative">
          <input
            v-model="password"
            :type="showPassword ? 'text' : 'password'"
            inputmode="text"
            placeholder="请输入密码"
            class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all text-sm pr-20"
          />
          <label
            class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1 text-xs text-gray-500 cursor-pointer"
          >
            <input
              type="checkbox"
              v-model="showPassword"
              class="w-3 h-3 rounded"
            />
            显示
          </label>
        </div>
      </div>

      <!-- 运营商 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1"
          >运营商</label
        >
        <select
          v-model="operator"
          class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all text-sm bg-white appearance-none"
        >
          <option
            v-for="op in operators"
            :key="op.value"
            :value="op.value"
          >
            {{ op.label }}
          </option>
        </select>
      </div>

      <!-- 登录按钮 -->
      <button
        @click="handleLogin"
        :disabled="loading"
        class="w-full py-3 mt-2 rounded-lg text-white font-medium text-base transition-all disabled:opacity-60 disabled:cursor-not-allowed bg-blue-600 hover:bg-blue-700 active:bg-blue-800 cursor-pointer"
      >
        {{ loading ? "登录中..." : "登 录" }}
      </button>
    </div>
  </div>
</template>
