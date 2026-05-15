export const MAX_LOG_ENTRIES = 1000

export const DS_BUTTONS = ["a", "b", "x", "y", "l", "r", "start", "select", "up", "down", "left", "right"] as const

export type DsButton = (typeof DS_BUTTONS)[number]

export interface AppSettings {
  readonly port: number
  readonly startReceiverWhenAppOpens: boolean
  readonly lockToFirstSender: boolean
  readonly packetLoggingEnabled: boolean
}

export type ReceiverStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "starting" }
  | { readonly kind: "running"; readonly boundAddress: string; readonly lockedSender: string | null }
  | { readonly kind: "stopping" }
  | { readonly kind: "error"; readonly message: string }

export type ViGemStatus =
  | { readonly kind: "unknown" }
  | { readonly kind: "ready" }
  | { readonly kind: "error"; readonly message: string }

export interface RuntimeStatus {
readonly receiver: ReceiverStatus
readonly viGem: ViGemStatus
readonly pressedButtons: readonly DsButton[]
readonly packetCount: number
readonly lastPacketAt: string | null
}

export type LogLevel = "info" | "packet" | "warning" | "error"

export interface LogEntry {
  readonly id: string
  readonly timestamp: string
  readonly level: LogLevel
  readonly message: string
}

export interface AppState {
  readonly settings: AppSettings
  readonly draftSettings: AppSettings
  readonly hasUnsavedSettings: boolean
  readonly runtimeStatus: RuntimeStatus
  readonly logs: readonly LogEntry[]
}

export type AppAction =
  | { readonly type: "settingsLoaded"; readonly settings: AppSettings }
  | { readonly type: "draftSettingsChanged"; readonly settings: Partial<AppSettings> }
  | { readonly type: "settingsSaved"; readonly settings: AppSettings }
  | { readonly type: "packetLoggingSaved"; readonly enabled: boolean }
  | { readonly type: "runtimeStatusReceived"; readonly runtimeStatus: RuntimeStatus }
  | { readonly type: "pressedButtonsReceived"; readonly pressedButtons: readonly DsButton[] }
  | { readonly type: "logReceived"; readonly entry: LogEntry }
  | { readonly type: "logsReceived"; readonly entries: readonly LogEntry[] }

export type ValidationResult<TValue> =
  | { readonly ok: true; readonly value: TValue }
  | { readonly ok: false; readonly error: string }
