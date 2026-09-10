export const GITHUB_LINK = 'https://github.com/ayangweb/BongoCat'

export const LISTEN_KEY = {
  SHOW_WINDOW: 'show-window',
  HIDE_WINDOW: 'hide-window',
  DEVICE_CHANGED: 'device-changed',
  GAMEPAD_CHANGED: 'gamepad-changed',
  START_MOTION: 'start-motion',
  SET_EXPRESSION: 'set-expression',
  CHAT_STREAM: 'chat-stream',
  CHAT_DONE: 'chat-done',
}

export const INVOKE_KEY = {
  COPY_DIR: 'copy_dir',
  START_DEVICE_LISTENING: 'start_device_listening',
  START_GAMEPAD_LISTING: 'start_gamepad_listing',
  STOP_GAMEPAD_LISTING: 'stop_gamepad_listing',
  OPEN_CHAT_WINDOW: 'open_chat_window',
  GET_CHAT_CONFIG: 'get_chat_config',
  SAVE_CHAT_CONFIG: 'save_chat_config',
  SAVE_CHAT_AVATAR: 'save_chat_avatar',
  CLEAR_CHAT_AVATAR: 'clear_chat_avatar',
  GET_CHAT_HISTORY: 'get_chat_history',
  CLEAR_CHAT_HISTORY: 'clear_chat_history',
  SEND_CHAT_MESSAGE: 'send_chat_message',
}

export const LANGUAGE = {
  ZH_CN: 'zh-CN',
  ZH_TW: 'zh-TW',
  EN_US: 'en-US',
  VI_VN: 'vi-VN',
  PT_BR: 'pt-BR',
} as const

export const WINDOW_LABEL = {
  MAIN: 'main',
  PREFERENCE: 'preference',
  CHAT: 'chat',
} as const
