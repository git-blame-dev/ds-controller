import type { ReceiverStatus, ViGemStatus } from "../app/types"

export type StatusTone = "neutral" | "good" | "warn" | "bad"

export function receiverStatusLabel(status: ReceiverStatus): string {
switch (status.kind) {
case "idle":
return "Idle"
case "starting":
return "Starting"
case "running":
return "Running"
case "stopping":
return "Stopping"
case "error":
return "Error"
}
}

export function receiverStatusTone(status: ReceiverStatus): StatusTone {
switch (status.kind) {
case "running":
return "good"
case "starting":
case "stopping":
return "warn"
case "error":
return "bad"
case "idle":
return "neutral"
}
}

export function viGemLabel(status: ViGemStatus): string {
switch (status.kind) {
case "ready":
return "Ready"
case "error":
return "Error"
case "unknown":
return "Unknown"
}
}
