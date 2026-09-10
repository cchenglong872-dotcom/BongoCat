export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

export interface ChatConfig {
  apiKey: string
  baseUrl: string
  model: string
  reasoning: boolean
  userAvatarPath: string | null
  weatherProvince: string
  weatherCity: string
  weatherApiKey: string
}

export interface ChatMessageItem extends ChatMessage {
  id: string
  streaming: boolean
}
