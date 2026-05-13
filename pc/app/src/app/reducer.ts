import type { AppAction, AppSettings, AppState, LogEntry, RuntimeStatus } from "./types"
import { MAX_LOG_ENTRIES } from "./types"

export const DEFAULT_APP_SETTINGS: AppSettings = Object.freeze({
  port: 26760,
  startReceiverWhenAppOpens: true,
  lockToFirstSender: true,
  packetLoggingEnabled: false,
})

const DEFAULT_RUNTIME_STATUS: RuntimeStatus = Object.freeze({
  receiver: Object.freeze({ kind: "idle" }),
  viGem: Object.freeze({ kind: "unknown" }),
pressedButtons: Object.freeze([]),
packetCount: 0,
lastPacketAt: null,
})

export function createInitialAppState(): AppState {
  return {
    settings: DEFAULT_APP_SETTINGS,
    draftSettings: DEFAULT_APP_SETTINGS,
    hasUnsavedSettings: false,
    runtimeStatus: DEFAULT_RUNTIME_STATUS,
    logs: [],
  }
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "settingsLoaded":
    case "settingsSaved":
      return {
        ...state,
        settings: action.settings,
        draftSettings: action.settings,
        hasUnsavedSettings: false,
      }
    case "draftSettingsChanged": {
      const draftSettings = { ...state.draftSettings, ...action.settings }

      return {
        ...state,
        draftSettings,
        hasUnsavedSettings: !settingsEqual(state.settings, draftSettings),
      }
    }
    case "runtimeStatusReceived":
      return { ...state, runtimeStatus: action.runtimeStatus }
    case "pressedButtonsReceived":
      return {
        ...state,
        runtimeStatus: {
          ...state.runtimeStatus,
          pressedButtons: action.pressedButtons,
        },
      }
    case "logReceived":
      return { ...state, logs: capLogEntries([...state.logs, action.entry]) }
    case "logsReceived":
      return { ...state, logs: capLogEntries([...state.logs, ...action.entries]) }
  }
}

function capLogEntries(entries: readonly LogEntry[]): readonly LogEntry[] {
  return entries.slice(-MAX_LOG_ENTRIES)
}

function settingsEqual(left: AppSettings, right: AppSettings): boolean {
  return (
    left.port === right.port &&
    left.startReceiverWhenAppOpens === right.startReceiverWhenAppOpens &&
    left.lockToFirstSender === right.lockToFirstSender &&
    left.packetLoggingEnabled === right.packetLoggingEnabled
  )
}
