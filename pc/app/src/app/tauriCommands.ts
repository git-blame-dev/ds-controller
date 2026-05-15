import { invoke } from "@tauri-apps/api/core"

import type { AppSettings, RuntimeStatus } from "./types"
import { isAppSettings, parseRuntimeStatus } from "./validation"

export async function getSettings(): Promise<AppSettings> {
  const payload = await invoke<unknown>("get_settings")
  return validateCommandPayload(payload, isAppSettings, "get_settings")
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
const payload = await invoke<unknown>("save_settings", { settings })
return validateCommandPayload(payload, isAppSettings, "save_settings")
}

export async function setPacketLoggingEnabled(enabled: boolean): Promise<AppSettings> {
const payload = await invoke<unknown>("set_packet_logging_enabled", { enabled })
return validateCommandPayload(payload, isAppSettings, "set_packet_logging_enabled")
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
const payload = await invoke<unknown>("get_runtime_status")
return parseCommandPayload(payload, parseRuntimeStatus, "get_runtime_status")
}

export async function startReceiver(): Promise<RuntimeStatus> {
const payload = await invoke<unknown>("start_receiver")
return parseCommandPayload(payload, parseRuntimeStatus, "start_receiver")
}

export async function stopReceiver(): Promise<RuntimeStatus> {
const payload = await invoke<unknown>("stop_receiver")
return parseCommandPayload(payload, parseRuntimeStatus, "stop_receiver")
}

export async function restartReceiver(): Promise<RuntimeStatus> {
const payload = await invoke<unknown>("restart_receiver")
return parseCommandPayload(payload, parseRuntimeStatus, "restart_receiver")
}

function validateCommandPayload<TValue>(
  payload: unknown,
  guard: (value: unknown) => value is TValue,
  commandName: string,
): TValue {
  if (guard(payload)) {
    return payload
  }

throw new Error(`${commandName} returned an invalid payload.`)
}

function parseCommandPayload<TValue>(
payload: unknown,
parser: (value: unknown) => { readonly ok: true; readonly value: TValue } | { readonly ok: false; readonly error: string },
commandName: string,
): TValue {
const result = parser(payload)
if (result.ok) {
return result.value
}

throw new Error(`${commandName} returned an invalid payload: ${result.error}`)
}
