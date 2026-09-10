import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { nanoid } from 'nanoid'

import { LISTEN_KEY } from '@/constants'
import { sendChatMessage } from '@/plugins/chat'
import { useChatStore } from '@/stores/chat'

import { useTauriListen } from './useTauriListen'

const appWindow = getCurrentWebviewWindow()

export function useChat() {
  const chatStore = useChatStore()

  useTauriListen<string>(LISTEN_KEY.CHAT_STREAM, ({ payload }) => {
    if (appWindow.label !== 'chat') return

    const current = chatStore.messages[chatStore.messages.length - 1]

    if (current?.role !== 'assistant' || !current.streaming) {
      chatStore.messages.push({
        role: 'assistant',
        content: '',
        timestamp: Math.floor(Date.now() / 1000),
        id: nanoid(),
        streaming: true,
      })
    }

    const last = chatStore.messages[chatStore.messages.length - 1]!

    last.content += payload
  })

  useTauriListen<{ content: string, error?: string }>(LISTEN_KEY.CHAT_DONE, ({ payload }) => {
    if (appWindow.label !== 'chat') return

    const current = chatStore.messages[chatStore.messages.length - 1]

    if (current?.role !== 'assistant' || !current.streaming) {
      chatStore.messages.push({
        role: 'assistant',
        content: payload.content,
        timestamp: Math.floor(Date.now() / 1000),
        id: nanoid(),
        streaming: false,
      })
    } else {
      current.content = payload.content
      current.streaming = false
    }

    chatStore.error = payload.error ?? ''

    chatStore.streaming = false
  })

  const send = async (content: string) => {
    const text = content.trim()

    if (!text || chatStore.streaming) return

    chatStore.error = ''
    chatStore.appendUser(text)
    chatStore.streaming = true

    try {
      await sendChatMessage(text)
    } catch (err) {
      chatStore.streaming = false
      chatStore.error = String(err)

      const last = chatStore.messages[chatStore.messages.length - 1]

      if (last?.role === 'user' && !last.streaming) {
        chatStore.messages.pop()
      }
    }
  }

  return {
    send,
  }
}
