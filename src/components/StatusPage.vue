<script setup lang="ts">
defineProps<{
  statusText: string;
  isSuccess: boolean;
}>();

const emit = defineEmits<{
  back: [];
}>();
</script>

<template>
  <div
    class="bg-white/95 backdrop-blur-sm rounded-2xl shadow-2xl p-6 sm:p-8 w-full max-w-[360px] text-center"
  >
    <!-- 状态图标 -->
    <div class="text-5xl mb-4">
      {{ isSuccess ? "✅" : "❌" }}
    </div>

    <!-- 状态标题 -->
    <h2
      class="text-lg font-bold mb-4"
      :class="isSuccess ? 'text-green-600' : 'text-red-500'"
    >
      {{ isSuccess ? "登录成功" : "登录失败" }}
    </h2>

    <!-- 状态消息 -->
    <div
      class="bg-gray-50 rounded-lg p-4 mb-6 text-sm text-gray-700 whitespace-pre-line text-left min-h-[60px]"
    >
      {{ statusText }}
    </div>

    <!-- 返回按钮 -->
    <button
      @click="emit('back')"
      class="w-full py-3 rounded-lg text-white font-medium text-base transition-all bg-gray-600 hover:bg-gray-700 active:bg-gray-800 cursor-pointer"
    >
      切换/修改账号
    </button>
  </div>
</template>
