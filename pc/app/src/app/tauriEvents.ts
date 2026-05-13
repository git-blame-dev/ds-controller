import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event"

import type { AppSettings, DsButton, LogEntry, RuntimeStatus } from "./types"
import { isAppSettings, isDsButtonArray, isLogEntry, isRuntimeStatus } from "./validation"

export const TAURI_EVENT_NAMES = Object.freeze({
settingsChanged: "settings://changed",
runtimeStatusChanged: "receiver://status",
logEntry: "receiver://log",
pressedButtonsChanged: "receiver://buttons",
})

export function listenToSettingsChanged(handler: (settings: AppSettings) => void): Promise<UnlistenFn> {
  return listenToValidatedEvent(TAURI_EVENT_NAMES.settingsChanged, isAppSettings, handler)
}

export function listenToRuntimeStatusChanged(handler: (runtimeStatus: RuntimeStatus) => void): Promise<UnlistenFn> {
  return listenToValidatedEvent(TAURI_EVENT_NAMES.runtimeStatusChanged, isRuntimeStatus, handler)
}

export function listenToLogEntry(handler: (entry: LogEntry) => void): Promise<UnlistenFn> {
  return listenToValidatedEvent(TAURI_EVENT_NAMES.logEntry, isLogEntry, handler)
}

export function listenToPressedButtonsChanged(handler: (pressedButtons: readonly DsButton[]) => void): Promise<UnlistenFn> {
  return listenToValidatedEvent(TAURI_EVENT_NAMES.pressedButtonsChanged, isDsButtonArray, handler)
}

function listenToValidatedEvent<TPayload>(
  eventName: string,
  guard: (value: unknown) => value is TPayload,
  handler: (payload: TPayload) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(eventName, (event: Event<unknown>) => {
    if (!guard(event.payload)) {
      throw new Error(`${eventName} emitted an invalid payload.`)
    }

    handler(event.payload)
  })
}
