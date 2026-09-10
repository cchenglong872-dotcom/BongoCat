import { invoke } from '@tauri-apps/api/core'

import type { ChatConfig, ChatMessage } from '@/types/chat'

import { INVOKE_KEY } from '../constants'

export function openChatWindow() {
  return invoke<void>(INVOKE_KEY.OPEN_CHAT_WINDOW)
}

export function getChatConfig() {
  return invoke<ChatConfig>(INVOKE_KEY.GET_CHAT_CONFIG)
}

export function saveChatConfig(config: ChatConfig) {
  return invoke<void>(INVOKE_KEY.SAVE_CHAT_CONFIG, { config })
}

export function saveChatAvatar(sourcePath: string) {
  return invoke<string>(INVOKE_KEY.SAVE_CHAT_AVATAR, { sourcePath })
}

export function clearChatAvatar() {
  return invoke<void>(INVOKE_KEY.CLEAR_CHAT_AVATAR)
}

export function getChatHistory() {
  return invoke<ChatMessage[]>(INVOKE_KEY.GET_CHAT_HISTORY)
}

export function clearChatHistory() {
  return invoke<void>(INVOKE_KEY.CLEAR_CHAT_HISTORY)
}

export function sendChatMessage(content: string) {
  return invoke<void>(INVOKE_KEY.SEND_CHAT_MESSAGE, { content })
}
