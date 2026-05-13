import type { AppSettings, DsButton, LogEntry, LogLevel, ReceiverStatus, RuntimeStatus, ValidationResult, ViGemStatus } from "./types"
import { DS_BUTTONS } from "./types"

const PORT_ERROR = "Port must be a whole number between 1 and 65535."
const LOG_LEVELS = new Set<LogLevel>(["info", "packet", "warning", "error"])
const DS_BUTTON_SET = new Set<string>(DS_BUTTONS)

export function validatePortInput(input: string): ValidationResult<number> {
  const normalizedInput = input.trim()

  if (!/^\d+$/.test(normalizedInput)) {
    return { ok: false, error: PORT_ERROR }
  }

  const port = Number(normalizedInput)

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return { ok: false, error: PORT_ERROR }
  }

  return { ok: true, value: port }
}

export function isAppSettings(value: unknown): value is AppSettings {
  if (!isRecord(value)) {
    return false
  }

  return (
    isValidPort(value.port) &&
    typeof value.startReceiverWhenAppOpens === "boolean" &&
    typeof value.lockToFirstSender === "boolean" &&
    typeof value.packetLoggingEnabled === "boolean"
  )
}

export function isRuntimeStatus(value: unknown): value is RuntimeStatus {
  if (!isRecord(value)) {
    return false
  }

return (
isReceiverStatus(value.receiver) &&
isViGemStatus(value.viGem) &&
isDsButtonArray(value.pressedButtons) &&
typeof value.packetCount === "number" &&
Number.isInteger(value.packetCount) &&
value.packetCount >= 0 &&
(typeof value.lastPacketAt === "string" || value.lastPacketAt === null)
)
}

export function isLogEntry(value: unknown): value is LogEntry {
  if (!isRecord(value)) {
    return false
  }

  return (
    typeof value.id === "string" &&
    typeof value.timestamp === "string" &&
    isLogLevel(value.level) &&
    typeof value.message === "string"
  )
}

export function isLogEntryArray(value: unknown): value is readonly LogEntry[] {
  return Array.isArray(value) && value.every(isLogEntry)
}

export function isDsButtonArray(value: unknown): value is readonly DsButton[] {
  return Array.isArray(value) && value.every(isDsButton)
}

function isReceiverStatus(value: unknown): value is ReceiverStatus {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return false
  }

  switch (value.kind) {
    case "idle":
    case "starting":
    case "stopping":
      return true
    case "running":
      return typeof value.boundAddress === "string" && (typeof value.lockedSender === "string" || value.lockedSender === null)
    case "error":
      return typeof value.message === "string"
    default:
      return false
  }
}

function isViGemStatus(value: unknown): value is ViGemStatus {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return false
  }

  switch (value.kind) {
    case "unknown":
    case "ready":
      return true
    case "error":
      return typeof value.message === "string"
    default:
      return false
  }
}

function isLogLevel(value: unknown): value is LogLevel {
  return typeof value === "string" && LOG_LEVELS.has(value as LogLevel)
}

function isDsButton(value: unknown): value is DsButton {
  return typeof value === "string" && DS_BUTTON_SET.has(value)
}

function isValidPort(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 65535
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}
