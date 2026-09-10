import type { Event } from '@tauri-apps/api/event'

import { PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { availableMonitors } from '@tauri-apps/api/window'
import { useDebounceFn } from '@vueuse/core'
import { isNumber } from 'es-toolkit/compat'
import { onMounted, ref, watch } from 'vue'

import { WINDOW_LABEL } from '@/constants'
import { useAppStore } from '@/stores/app'
import { useCatStore } from '@/stores/cat'
import { getCursorMonitor } from '@/utils/monitor'

export type WindowState = Record<string, Partial<PhysicalPosition & PhysicalSize> | undefined>

const appWindow = getCurrentWebviewWindow()
const { label } = appWindow

export function useWindowState() {
  const appStore = useAppStore()
  const catStore = useCatStore()
  const isRestored = ref(false)

  onMounted(() => {
    appWindow.onMoved(onChange)

    appWindow.onResized(onChange)

    appWindow.onScaleChanged(clampToMonitor)
  })

  const clampToMonitor = useDebounceFn(async () => {
    if (label !== WINDOW_LABEL.MAIN || !catStore.window.keepInScreen) return

    const monitor = await getCursorMonitor()

    if (!monitor) return

    const { position: monitorPos, size: monitorSize } = monitor
    const windowSize = await appWindow.outerSize()
    const windowPos = await appWindow.outerPosition()

    const minX = monitorPos.x
    const maxX = monitorPos.x + monitorSize.width - windowSize.width
    const minY = monitorPos.y
    const maxY = monitorPos.y + monitorSize.height - windowSize.height

    const clampedX = Math.max(minX, Math.min(windowPos.x, maxX))
    const clampedY = Math.max(minY, Math.min(windowPos.y, maxY))

    if (clampedX === windowPos.x && clampedY === windowPos.y) return

    return appWindow.setPosition(new PhysicalPosition(clampedX, clampedY))
  }, 500)

  watch(() => catStore.window.keepInScreen, clampToMonitor)

  const onChange = async (event: Event<PhysicalPosition | PhysicalSize>) => {
    const minimized = await appWindow.isMinimized()

    if (minimized) return

    appStore.windowState[label] ??= {}

    Object.assign(appStore.windowState[label], event.payload)

    clampToMonitor()
  }

  const restoreState = async () => {
    const { x, y, width, height } = appStore.windowState[label] ?? {}

    // 聊天窗口固定为默认尺寸（380×560），不恢复历史保存的尺寸，避免出现异常的大窗口
    const isChat = label === WINDOW_LABEL.CHAT

    const currentSize = await appWindow.outerSize()

    const restoreWidth = isChat ? currentSize.width : (width ?? currentSize.width)
    const restoreHeight = isChat ? currentSize.height : (height ?? currentSize.height)

    if (isNumber(x) && isNumber(y)) {
      const monitors = await availableMonitors()

      const monitor = monitors.find((monitor) => {
        const { position, size } = monitor

        const inBoundsX = x >= position.x && x <= position.x + size.width
        const inBoundsY = y >= position.y && y <= position.y + size.height

        return inBoundsX && inBoundsY
      })

      if (monitor) {
        const { position: monitorPos, size: monitorSize } = monitor

        // 夹紧窗口位置，确保整个窗口完整落在屏幕内，避免恢复到越界位置导致画面被截断
        const maxX = monitorPos.x + monitorSize.width - restoreWidth
        const maxY = monitorPos.y + monitorSize.height - restoreHeight

        const clampedX = Math.max(monitorPos.x, Math.min(x, maxX))
        const clampedY = Math.max(monitorPos.y, Math.min(y, maxY))

        await appWindow.setPosition(new PhysicalPosition(clampedX, clampedY))
      }
    }

    if (!isChat && width && height) {
      await appWindow.setSize(new PhysicalSize(width, height))
    }

    isRestored.value = true

    clampToMonitor()
  }

  return {
    isRestored,
    restoreState,
  }
}
