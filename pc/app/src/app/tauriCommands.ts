import { invoke } from "@tauri-apps/api/core"

import type { AppSettings, RuntimeStatus } from "./types"
import { isAppSettings, isRuntimeStatus } from "./validation"

export async function getSettings(): Promise<AppSettings> {
  const payload = await invoke<unknown>("get_settings")
  return validateCommandPayload(payload, isAppSettings, "get_settings")
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  const payload = await invoke<unknown>("save_settings", { settings })
  return validateCommandPayload(payload, isAppSettings, "save_settings")
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  const payload = await invoke<unknown>("get_runtime_status")
  return validateCommandPayload(payload, isRuntimeStatus, "get_runtime_status")
}

export async function startReceiver(): Promise<RuntimeStatus> {
  const payload = await invoke<unknown>("start_receiver")
  return validateCommandPayload(payload, isRuntimeStatus, "start_receiver")
}

export async function stopReceiver(): Promise<RuntimeStatus> {
  const payload = await invoke<unknown>("stop_receiver")
  return validateCommandPayload(payload, isRuntimeStatus, "stop_receiver")
}

export async function restartReceiver(): Promise<RuntimeStatus> {
  const payload = await invoke<unknown>("restart_receiver")
  return validateCommandPayload(payload, isRuntimeStatus, "restart_receiver")
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
