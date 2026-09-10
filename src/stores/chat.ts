import { nanoid } from 'nanoid'
import { defineStore } from 'pinia'
import { ref } from 'vue'

import type { ChatConfig, ChatMessageItem } from '@/types/chat'

import { clearChatHistory, getChatConfig, getChatHistory, saveChatConfig } from '@/plugins/chat'

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessageItem[]>([])
  const config = ref<ChatConfig>({
    apiKey: '',
    baseUrl: '',
    model: '',
    reasoning: true,
    userAvatarPath: null,
    weatherProvince: '',
    weatherCity: '',
    weatherApiKey: '',
  })
  const streaming = ref(false)
  const error = ref('')
  const configVisible = ref(false)

  const init = async () => {
    const [cfg, history] = await Promise.all([getChatConfig(), getChatHistory()])

    config.value = {
      ...cfg,
      userAvatarPath: cfg.userAvatarPath ?? null,
    }

    messages.value = history.map(message => ({
      ...message,
      id: nanoid(),
      streaming: false,
    }))
  }

  const appendUser = (content: string) => {
    messages.value.push({
      role: 'user',
      content,
      timestamp: Math.floor(Date.now() / 1000),
      id: nanoid(),
      streaming: false,
    })
  }

  const clear = async () => {
    await clearChatHistory()

    messages.value = []
  }

  const saveConfig = async (nextConfig: ChatConfig) => {
    await saveChatConfig(nextConfig)

    config.value = nextConfig
  }

  return {
    messages,
    config,
    streaming,
    error,
    configVisible,
    init,
    appendUser,
    clear,
    saveConfig,
  }
}, {
  tauri: {
    save: false,
    sync: false,
  },
})
