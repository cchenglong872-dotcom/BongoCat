<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { appDataDir } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { Button, Drawer, Input, InputPassword, message, Popconfirm, Switch } from 'antdv-next'
import { nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import VueMarkdown from 'vue-markdown-render'

import { useChat } from '@/composables/useChat'
import { clearChatAvatar, saveChatAvatar } from '@/plugins/chat'
import { useChatStore } from '@/stores/chat'
import { useGeneralStore } from '@/stores/general'
import { join } from '@/utils/path'

const chatStore = useChatStore()
const generalStore = useGeneralStore()
const { t, te } = useI18n()
const { send } = useChat()

const draft = ref('')
const scrollContainer = ref<HTMLElement>()
const avatarUrl = ref('')
const pendingAvatarPath = ref('')
const removeAvatarPending = ref(false)
const avatarLoading = ref(false)

const configForm = ref({
  apiKey: '',
  baseUrl: '',
  model: '',
  reasoning: true,
  userAvatarPath: null as string | null,
  weatherProvince: '',
  weatherCity: '',
  weatherApiKey: '',
})

function scrollToBottom() {
  nextTick(() => {
    const el = scrollContainer.value

    if (el) el.scrollTop = el.scrollHeight
  })
}

async function refreshAvatarUrl(path = chatStore.config.userAvatarPath) {
  if (!path) {
    avatarUrl.value = ''
    return
  }

  avatarUrl.value = convertFileSrc(join(await appDataDir(), path))
}

function handleAvatarError() {
  avatarUrl.value = ''
}

async function handleChooseAvatar() {
  const selected = await open({
    directory: false,
    filters: [{ extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'ico'], name: t('pages.chat.avatarFileType') }],
    multiple: false,
  })

  if (typeof selected !== 'string') return

  pendingAvatarPath.value = selected
  removeAvatarPending.value = false
  avatarUrl.value = convertFileSrc(selected)
}

function handleRemoveAvatar() {
  pendingAvatarPath.value = ''
  removeAvatarPending.value = true
  avatarUrl.value = ''
}

async function handleSend() {
  const content = draft.value.trim()

  if (!content || chatStore.streaming) return

  draft.value = ''
  scrollToBottom()
  await send(content)
}

async function handleClear() {
  await chatStore.clear()
}

async function handleSaveConfig() {
  avatarLoading.value = true

  try {
    let userAvatarPath = configForm.value.userAvatarPath

    if (pendingAvatarPath.value) {
      userAvatarPath = await saveChatAvatar(pendingAvatarPath.value)
    } else if (removeAvatarPending.value) {
      await clearChatAvatar()
      userAvatarPath = null
    }

    await chatStore.saveConfig({
      apiKey: configForm.value.apiKey.trim(),
      baseUrl: configForm.value.baseUrl.trim(),
      model: configForm.value.model.trim(),
      reasoning: configForm.value.reasoning,
      userAvatarPath,
      weatherProvince: configForm.value.weatherProvince.trim(),
      weatherCity: configForm.value.weatherCity.trim(),
      weatherApiKey: configForm.value.weatherApiKey.trim(),
    })

    pendingAvatarPath.value = ''
    removeAvatarPending.value = false
    configForm.value.userAvatarPath = userAvatarPath
    await refreshAvatarUrl(userAvatarPath)
    chatStore.configVisible = false
    message.success(t('pages.chat.saved'))
  } catch (error) {
    const key = `pages.chat.${String(error)}`

    message.error(te(key) ? t(key) : t('pages.chat.avatarSaveFailed'))
  } finally {
    avatarLoading.value = false
  }
}

function resetConfigForm() {
  configForm.value = {
    apiKey: chatStore.config.apiKey,
    baseUrl: chatStore.config.baseUrl,
    model: chatStore.config.model,
    reasoning: chatStore.config.reasoning,
    userAvatarPath: chatStore.config.userAvatarPath,
    weatherProvince: chatStore.config.weatherProvince,
    weatherCity: chatStore.config.weatherCity,
    weatherApiKey: chatStore.config.weatherApiKey,
  }
  pendingAvatarPath.value = ''
  removeAvatarPending.value = false
  refreshAvatarUrl()
}

watch(() => chatStore.configVisible, (visible) => {
  if (visible) resetConfigForm()
})

watch(() => chatStore.messages.length, scrollToBottom)

watch(() => generalStore.appearance.isDark, (value) => {
  document.documentElement.classList.toggle('dark', value)
}, { immediate: true })

onMounted(async () => {
  await chatStore.init()
  await refreshAvatarUrl()
})
</script>

<template>
  <div class="h-screen flex flex-col bg-[#f5f6f7] dark:bg-[#141418]">
    <!-- 顶栏 -->
    <header class="h-12 flex shrink-0 items-center justify-between border-b border-black/5 px-3.5 bg-white dark:border-white/5 dark:bg-[#1b1b20]">
      <div class="min-w-0 flex items-center gap-3">
        <div class="flex items-center gap-2">
          <div class="size-8 flex items-center justify-center border-[#07c160]/20 bg-[#07c160]/10 text-[#07c160] border rounded-lg">
            <span class="i-lucide:cat text-lg" />
          </div>
          <span class="truncate text-neutral-800 font-semibold text-sm dark:text-white">{{ $t('pages.chat.title') }}</span>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-1.5">
        <Popconfirm
          :cancel-text="$t('pages.chat.cancel')"
          :ok-text="$t('pages.chat.confirm')"
          :title="$t('pages.chat.clearConfirm')"
          @confirm="handleClear"
        >
          <Button
            class="!text-neutral-600 dark:!text-neutral-400 hover:!text-neutral-800 dark:hover:!text-neutral-200"
            size="small"
            type="text"
          >
            {{ $t('pages.chat.clear') }}
          </Button>
        </Popconfirm>
        <Button
          class="!text-neutral-600 dark:!text-neutral-400 hover:!text-neutral-800 dark:hover:!text-neutral-200"
          size="small"
          type="text"
          @click="chatStore.configVisible = true"
        >
          {{ $t('pages.chat.settings') }}
        </Button>
      </div>
    </header>

    <!-- 消息列表 -->
    <div
      ref="scrollContainer"
      class="message-container flex-1 overflow-y-auto px-4 py-6"
    >
      <div
        v-if="chatStore.messages.length === 0"
        class="h-full flex flex-col items-center justify-center gap-4 text-neutral-400 text-sm"
      >
        <div class="empty-avatar size-14 flex items-center justify-center border-[#07c160]/20 bg-[#07c160]/10 text-[#07c160] border rounded-2xl">
          <span class="i-lucide:cat text-3xl" />
        </div>
        <div class="text-base">
          {{ $t('pages.chat.empty') }}
        </div>
      </div>

      <div class="mx-auto max-w-3xl">
        <div
          v-for="item in chatStore.messages"
          :key="item.id"
          class="message-item mb-6 flex items-start gap-3"
          :class="item.role === 'user' ? 'flex-row-reverse' : 'flex-row'"
        >
          <!-- 头像 -->
          <div
            class="avatar size-9 flex shrink-0 items-center justify-center overflow-hidden border rounded-full"
            :class="item.role === 'assistant'
              ? 'border-[#07c160]/20 bg-[#07c160]/10 text-[#07c160]'
              : 'border-black/5 bg-[#eef1f4] text-neutral-500 dark:border-white/10 dark:bg-[#303038] dark:text-neutral-300'"
          >
            <span
              v-if="item.role === 'assistant'"
              class="i-lucide:cat text-lg"
            />
            <img
              v-else-if="avatarUrl"
              :alt="$t('pages.chat.userAvatar')"
              class="h-full w-full object-cover"
              :src="avatarUrl"
              @error="handleAvatarError"
            >
            <span
              v-else
              class="i-lucide:user-round text-base"
            />
          </div>

          <!-- 消息气泡 -->
          <div
            class="message-bubble flex flex-col gap-1"
            :class="item.role === 'user' ? 'items-end' : 'items-start'"
          >
            <div
              class="message-content max-w-xl px-3.5 py-2.5 leading-relaxed border text-sm"
              :class="item.role === 'assistant'
                ? 'border-black/5 bg-white text-neutral-800 rounded-xl rounded-tl-sm dark:border-white/10 dark:bg-[#222228] dark:text-neutral-100'
                : 'border-[#07c160] bg-[#07c160] text-white rounded-xl rounded-tr-sm'"
            >
              <template v-if="item.role === 'assistant'">
                <VueMarkdown
                  v-if="item.content"
                  :source="item.content"
                />
                <div
                  v-else-if="item.streaming"
                  class="flex items-center gap-1.5 py-1 text-[#07c160]"
                >
                  <span class="typing-dot" />
                  <span class="typing-dot" />
                  <span class="typing-dot" />
                </div>
              </template>
              <template v-else>
                <div class="whitespace-pre-wrap break-words">
                  {{ item.content }}
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <div
        v-if="chatStore.error"
        class="bg-red-50 text-red-600 dark:bg-red-900/20 dark:text-red-400 mx-auto mt-4 max-w-3xl px-4 py-2 text-center text-xs rounded-lg"
      >
        {{ chatStore.error }}
      </div>
    </div>

    <!-- 输入区 -->
    <footer class="shrink-0 border-t border-black/5 p-3 bg-white dark:border-white/5 dark:bg-[#1b1b20]">
      <div class="mx-auto max-w-3xl">
        <textarea
          v-model="draft"
          class="input-textarea h-20 w-full resize-none border-black/10 px-4 py-3 text-neutral-800 outline-none transition-all bg-white border text-sm rounded-xl dark:border-white/10 focus:border-[#07c160]/50 dark:bg-[#22222a] dark:text-white placeholder:text-neutral-400 focus:ring-2 focus:ring-[#07c160]/20 dark:placeholder:text-neutral-500"
          :placeholder="$t('pages.chat.placeholder')"
          @keydown.enter.exact.prevent="handleSend"
        />
        <div class="mt-3 flex items-center justify-between">
          <span class="text-neutral-400 text-xs">{{ $t('pages.chat.enterHint') }}</span>
          <Button
            class="!bg-[#07c160] !transition-colors hover:!bg-[#06ad56]"
            :disabled="chatStore.streaming"
            :loading="chatStore.streaming"
            size="large"
            type="primary"
            @click="handleSend"
          >
            {{ $t('pages.chat.send') }}
          </Button>
        </div>
      </div>
    </footer>

    <!-- 设置抽屉 -->
    <Drawer
      v-model:open="chatStore.configVisible"
      :footer="null"
      :title="$t('pages.chat.settingsTitle')"
      width="360"
    >
      <div class="space-y-5">
        <div class="flex items-center gap-3 bg-neutral-50 p-3 rounded-xl dark:bg-white/5">
          <div class="size-14 flex shrink-0 items-center justify-center overflow-hidden border-black/10 bg-[#eef1f4] text-neutral-500 border rounded-full dark:border-white/10 dark:bg-[#303038] dark:text-neutral-300">
            <img
              v-if="avatarUrl"
              :alt="$t('pages.chat.userAvatar')"
              class="h-full w-full object-cover"
              :src="avatarUrl"
              @error="handleAvatarError"
            >
            <span
              v-else
              class="i-lucide:user-round text-2xl"
            />
          </div>
          <div class="min-w-0 flex-1">
            <div class="text-neutral-800 font-medium text-sm dark:text-white">
              {{ $t('pages.chat.userAvatar') }}
            </div>
            <div class="mt-1 text-neutral-400 text-xs">
              {{ $t('pages.chat.userAvatarHint') }}
            </div>
            <div class="mt-2 flex gap-2">
              <Button
                :disabled="avatarLoading"
                size="small"
                @click="handleChooseAvatar"
              >
                {{ $t('pages.chat.chooseAvatar') }}
              </Button>
              <Button
                v-if="avatarUrl || configForm.userAvatarPath"
                :disabled="avatarLoading"
                size="small"
                type="text"
                @click="handleRemoveAvatar"
              >
                {{ $t('pages.chat.removeAvatar') }}
              </Button>
            </div>
          </div>
        </div>
        <div>
          <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
            {{ $t('pages.chat.apiKey') }}
          </div>
          <InputPassword
            v-model:value="configForm.apiKey"
            placeholder="sk-..."
            size="large"
          />
        </div>
        <div>
          <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
            {{ $t('pages.chat.baseUrl') }}
          </div>
          <Input
            v-model:value="configForm.baseUrl"
            placeholder="https://api.example.com"
            size="large"
          />
        </div>
        <div>
          <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
            {{ $t('pages.chat.model') }}
          </div>
          <Input
            v-model:value="configForm.model"
            :placeholder="$t('pages.chat.modelPlaceholder')"
            size="large"
          />
        </div>
        <!-- 天气查询默认设置 -->
        <div class="bg-neutral-50 p-3 rounded-lg dark:bg-white/5">
          <div class="text-neutral-800 font-medium text-sm dark:text-white">
            {{ $t('pages.chat.weather') }}
          </div>
          <div class="mt-0.5 text-neutral-500 text-xs dark:text-neutral-400">
            {{ $t('pages.chat.weatherHint') }}
          </div>
          <div class="mt-3 flex gap-2">
            <div class="min-w-0 flex-1">
              <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
                {{ $t('pages.chat.province') }}
              </div>
              <Input
                v-model:value="configForm.weatherProvince"
                :placeholder="$t('pages.chat.provincePlaceholder')"
                size="large"
              />
            </div>
            <div class="min-w-0 flex-1">
              <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
                {{ $t('pages.chat.city') }}
              </div>
              <Input
                v-model:value="configForm.weatherCity"
                :placeholder="$t('pages.chat.cityPlaceholder')"
                size="large"
              />
            </div>
          </div>
          <div class="mt-3">
            <div class="mb-2 text-neutral-600 font-medium text-xs dark:text-neutral-400">
              {{ $t('pages.chat.weatherApiKey') }}
            </div>
            <InputPassword
              v-model:value="configForm.weatherApiKey"
              :placeholder="$t('pages.chat.weatherApiKeyPlaceholder')"
              size="large"
            />
          </div>
        </div>
        <div class="flex items-center justify-between gap-3 bg-neutral-50 p-3 rounded-lg dark:bg-white/5">
          <div class="min-w-0">
            <div class="text-neutral-800 font-medium text-sm dark:text-white">
              {{ $t('pages.chat.reasoning') }}
            </div>
            <div class="mt-0.5 text-neutral-500 text-xs dark:text-neutral-400">
              {{ $t('pages.chat.reasoningHint') }}
            </div>
          </div>
          <Switch v-model:checked="configForm.reasoning" />
        </div>
        <div class="flex justify-end gap-2 pt-2">
          <Button
            :disabled="avatarLoading"
            @click="chatStore.configVisible = false"
          >
            {{ $t('pages.chat.cancel') }}
          </Button>
          <Button
            class="!bg-[#07c160]"
            :loading="avatarLoading"
            type="primary"
            @click="handleSaveConfig"
          >
            {{ $t('pages.chat.save') }}
          </Button>
        </div>
      </div>
    </Drawer>
  </div>
</template>

<style scoped>
/* 消息容器滚动条美化 */
.message-container::-webkit-scrollbar {
  width: 6px;
}

.message-container::-webkit-scrollbar-track {
  background: transparent;
}

.message-container::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
}

.dark .message-container::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
}

.message-container::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.3);
}

.dark .message-container::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

/* 消息项动画 */
.message-item {
  animation: fadeSlideIn 0.3s ease-out;
}

@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 打字动画 */
.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.4;
  animation: typing 1.2s infinite;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typing {
  0%,
  100% {
    opacity: 0.4;
    transform: translateY(0);
  }
  50% {
    opacity: 1;
    transform: translateY(-3px);
  }
}

/* Markdown 样式优化 */
:deep(.markdown-body) {
  font-size: 14px;
  line-height: 1.6;
}

:deep(.markdown-body p) {
  margin: 0.5em 0;
}

:deep(.markdown-body p:first-child) {
  margin-top: 0;
}

:deep(.markdown-body p:last-child) {
  margin-bottom: 0;
}

:deep(.markdown-body code) {
  background: rgba(0, 0, 0, 0.05);
  padding: 0.2em 0.4em;
  border-radius: 3px;
  font-size: 0.9em;
}

.dark :deep(.markdown-body code) {
  background: rgba(255, 255, 255, 0.1);
}

:deep(.markdown-body pre) {
  background: rgba(0, 0, 0, 0.05);
  padding: 0.8em;
  border-radius: 6px;
  overflow-x: auto;
  margin: 0.5em 0;
}

.dark :deep(.markdown-body pre) {
  background: rgba(255, 255, 255, 0.05);
}

:deep(.markdown-body pre code) {
  background: transparent;
  padding: 0;
}

:deep(.markdown-body ul),
:deep(.markdown-body ol) {
  margin: 0.5em 0;
  padding-left: 1.5em;
}

:deep(.markdown-body li) {
  margin: 0.25em 0;
}

:deep(.markdown-body blockquote) {
  border-left: 3px solid rgba(0, 0, 0, 0.1);
  padding-left: 1em;
  margin: 0.5em 0;
  color: rgba(0, 0, 0, 0.6);
}

.dark :deep(.markdown-body blockquote) {
  border-left-color: rgba(255, 255, 255, 0.2);
  color: rgba(255, 255, 255, 0.6);
}
</style>
