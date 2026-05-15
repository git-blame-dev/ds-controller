import type { ChangeEvent } from "react"

import type { AppSettings, DsButton, ReceiverStatus, ViGemStatus } from "../app/types"
import type { ValidationResult } from "../app/types"
import { ButtonBadges } from "./ButtonBadges"
import { MetricItem } from "./MetricItem"
import { StatusBadge } from "./StatusBadge"
import { receiverStatusLabel, receiverStatusTone, type StatusTone, viGemLabel } from "./statusFormat"

interface ReceiverCardProps {
readonly draftSettings: AppSettings
readonly portValue: string
readonly receiver: ReceiverStatus
readonly viGem: ViGemStatus
readonly lastPacketAt: string | null
readonly packetCount: number
readonly pressedButtons: readonly DsButton[]
readonly hasUnsavedSettings: boolean
readonly portValidation: ValidationResult<number>
readonly onPortChange: (value: string) => void
readonly onToggle: (key: keyof Omit<AppSettings, "port">, value: boolean) => void
readonly onStart: () => void
readonly onStop: () => void
readonly onApplyRestart: () => void
}

export function ReceiverCard({
draftSettings,
portValue,
receiver,
viGem,
lastPacketAt,
packetCount,
pressedButtons,
hasUnsavedSettings,
portValidation,
onPortChange,
onToggle,
onStart,
onStop,
onApplyRestart,
}: ReceiverCardProps) {
const isRunning = receiver.kind === "running" || receiver.kind === "starting"
const canApply = hasUnsavedSettings && portValidation.ok
const bindAddress = receiver.kind === "running" ? receiver.boundAddress : "Not listening"
const sender = receiver.kind === "running" ? receiver.lockedSender ?? "Waiting for DS" : "None"
const viGemTone = viGemStatusTone(viGem)

return (
<section className="rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<div className="flex flex-wrap items-center justify-between gap-3">
<h1 className="text-sm font-semibold text-white">Receiver</h1>
<div className="flex flex-wrap items-center gap-2">
<StatusBadge label={receiverStatusLabel(receiver)} tone={receiverStatusTone(receiver)} />
<StatusBadge label={`ViGEm ${viGemLabel(viGem)}`} tone={viGemTone} />
</div>
</div>

<div className="mt-4 grid gap-3 lg:grid-cols-[1fr_auto_auto] lg:items-end">
<label className="space-y-2">
<span className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Port</span>
<input
className="h-11 w-full rounded-xl border border-input bg-black/20 px-4 text-sm text-white outline-none ring-cyan-300/0 transition focus:border-cyan-300/70 focus:ring-4 focus:ring-cyan-300/10"
inputMode="numeric"
value={portValue}
onChange={(event: ChangeEvent<HTMLInputElement>) => onPortChange(event.target.value)}
aria-invalid={!portValidation.ok}
/>
</label>
<button
className="h-11 rounded-xl bg-cyan-300 px-5 text-sm font-semibold text-slate-950 shadow-lg shadow-cyan-500/20 transition hover:bg-cyan-200 disabled:cursor-not-allowed disabled:opacity-50"
type="button"
onClick={isRunning ? onStop : onStart}
>
{isRunning ? "Stop Receiver" : "Start Receiver"}
</button>
<button
className="h-11 rounded-xl border border-white/10 bg-white/5 px-5 text-sm font-semibold text-white transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-50"
type="button"
disabled={!canApply}
onClick={onApplyRestart}
>
Apply & Restart
</button>
</div>
{!portValidation.ok ? <p className="mt-2 text-sm text-red-300">{portValidation.error}</p> : null}

<dl className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
<MetricItem label="Bind" value={bindAddress} />
<MetricItem label="Sender" value={sender} />
<MetricItem label="Last packet" value={formatTimestamp(lastPacketAt)} />
<MetricItem label="Packets" value={packetCount} />
</dl>
<div className="mt-4 rounded-xl border border-white/10 bg-black/20 p-3">
<p className="mb-2 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Pressed buttons</p>
<ButtonBadges buttons={pressedButtons} />
</div>
{viGem.kind === "error" ? <p className="mt-3 rounded-xl border border-red-400/20 bg-red-400/10 p-3 text-sm text-red-200">{viGem.message}</p> : null}

<div className="mt-4 grid gap-3 md:grid-cols-3">
<SwitchRow label="Start receiver when app opens" checked={draftSettings.startReceiverWhenAppOpens} onChange={(value) => onToggle("startReceiverWhenAppOpens", value)} />
<SwitchRow label="Lock to first sender" checked={draftSettings.lockToFirstSender} onChange={(value) => onToggle("lockToFirstSender", value)} />
<SwitchRow label="Show packet stream in logs" checked={draftSettings.packetLoggingEnabled} onChange={(value) => onToggle("packetLoggingEnabled", value)} />
</div>
</section>
)
}

function viGemStatusTone(status: ViGemStatus): StatusTone {
switch (status.kind) {
case "ready":
return "good"
case "error":
return "bad"
case "unknown":
return "neutral"
}
}

function formatTimestamp(timestamp: string | null): string {
if (timestamp === null) {
return "None"
}

const millis = Number(timestamp)
if (!Number.isFinite(millis)) {
return timestamp
}

return new Date(millis).toLocaleTimeString()
}

interface SwitchRowProps {
readonly label: string
readonly checked: boolean
readonly onChange: (value: boolean) => void
}

function SwitchRow({ label, checked, onChange }: SwitchRowProps) {
return (
<button
type="button"
role="switch"
aria-checked={checked}
onClick={() => onChange(!checked)}
className="flex items-center justify-between gap-3 rounded-xl border border-white/10 bg-white/[0.03] p-3 text-left text-sm text-slate-200 transition hover:bg-white/[0.06]"
>
<span>{label}</span>
<span className={`flex h-6 w-11 items-center rounded-full p-1 transition ${checked ? "bg-cyan-300" : "bg-slate-700"}`}>
<span className={`h-4 w-4 rounded-full bg-slate-950 transition ${checked ? "translate-x-5" : "translate-x-0"}`} />
</span>
</button>
)
}
