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
return parseRuntimeStatus(value).ok
}

export function parseRuntimeStatus(value: unknown): ValidationResult<RuntimeStatus> {
if (!isRecord(value)) {
return { ok: false, error: "Runtime status must be an object." }
}

const receiver = parseReceiverStatus(value.receiver)
if (!receiver.ok) return receiver

const viGem = parseViGemStatus(value.viGem)
if (!viGem.ok) return viGem

const pressedButtons = parseDsButtonArray(value.pressedButtons)
if (!pressedButtons.ok) return pressedButtons

const packetCount = parsePacketCount(value.packetCount)
if (!packetCount.ok) return packetCount

const lastPacketAt = value.lastPacketAt
if (!(typeof lastPacketAt === "string" || lastPacketAt === null)) {
return { ok: false, error: "Runtime status lastPacketAt must be a string or null." }
}

return {
ok: true,
value: {
receiver: receiver.value,
viGem: viGem.value,
pressedButtons: pressedButtons.value,
packetCount: packetCount.value,
lastPacketAt,
},
}
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

function parseReceiverStatus(value: unknown): ValidationResult<ReceiverStatus> {
if (!isRecord(value) || typeof value.kind !== "string") {
return { ok: false, error: "Receiver status must include a string kind." }
}

switch (value.kind) {
case "idle":
case "starting":
case "stopping":
return { ok: true, value: { kind: value.kind } }
case "running":
{
const boundAddress = value.boundAddress
const lockedSender = value.lockedSender
if (typeof boundAddress !== "string" || !(typeof lockedSender === "string" || lockedSender === null)) {
return { ok: false, error: "Running receiver status has invalid sender fields." }
}
return { ok: true, value: { kind: "running", boundAddress, lockedSender } }
}
case "error":
return typeof value.message === "string"
? { ok: true, value: { kind: "error", message: value.message } }
: { ok: false, error: "Receiver error status must include a message." }
default:
return { ok: false, error: `Unknown receiver status kind: ${value.kind}` }
}
}

function parseViGemStatus(value: unknown): ValidationResult<ViGemStatus> {
if (!isRecord(value) || typeof value.kind !== "string") {
return { ok: false, error: "ViGEm status must include a string kind." }
}

switch (value.kind) {
case "unknown":
case "ready":
return { ok: true, value: { kind: value.kind } }
case "error":
return typeof value.message === "string"
? { ok: true, value: { kind: "error", message: value.message } }
: { ok: false, error: "ViGEm error status must include a message." }
default:
return { ok: false, error: `Unknown ViGEm status kind: ${value.kind}` }
}
}

function parseDsButtonArray(value: unknown): ValidationResult<readonly DsButton[]> {
if (!isDsButtonArray(value)) {
return { ok: false, error: "Pressed buttons must be a DS button array." }
}

return { ok: true, value }
}

function parsePacketCount(value: unknown): ValidationResult<number> {
if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0) {
return { ok: true, value }
}

return { ok: false, error: "Packet count must be a non-negative safe integer." }
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
