import type { ChangeEvent } from "react"

import type { AppSettings, DsButton, ReceiverStatus, VirtualControllerStatus } from "../app/types"
import type { ValidationResult } from "../app/types"
import { ButtonBadges } from "./ButtonBadges"
import { MetricItem } from "./MetricItem"
import { StatusBadge } from "./StatusBadge"
import { receiverStatusLabel, receiverStatusTone, type StatusTone, virtualControllerLabel } from "./statusFormat"

interface ReceiverCardProps {
readonly draftSettings: AppSettings
readonly portValue: string
readonly receiver: ReceiverStatus
readonly virtualController: VirtualControllerStatus
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
virtualController,
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
const sender = receiver.kind === "running" ? receiver.lastSender ?? "Waiting for DS" : "None"
const virtualControllerTone = virtualControllerStatusTone(virtualController)

return (
<section aria-label="Receiver" className="rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<div className="flex flex-wrap items-center gap-3">
<label className="flex h-11 min-w-40 flex-1 items-center gap-3 rounded-xl border border-input bg-black/20 px-4 ring-cyan-300/0 transition focus-within:border-cyan-300/70 focus-within:ring-4 focus-within:ring-cyan-300/10">
<span className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Port</span>
<input
className="min-w-0 flex-1 bg-transparent text-sm tabular-nums text-white outline-none"
inputMode="numeric"
value={portValue}
onChange={(event: ChangeEvent<HTMLInputElement>) => onPortChange(event.target.value)}
aria-invalid={!portValidation.ok}
/>
</label>
<div className="flex min-h-11 flex-wrap items-center gap-2">
<StatusBadge label={receiverStatusLabel(receiver)} tone={receiverStatusTone(receiver)} />
<StatusBadge label={`Virtual controller ${virtualControllerLabel(virtualController)}`} tone={virtualControllerTone} />
</div>
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
<MetricItem label="Last sender" value={sender} />
<MetricItem label="Last packet" value={formatTimestamp(lastPacketAt)} />
<MetricItem label="Packets" value={packetCount} />
</dl>
<div className="mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-white/10 bg-black/20 p-3">
<p className="shrink-0 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Pressed buttons</p>
<div className="min-w-36 flex-1">
<ButtonBadges buttons={pressedButtons} />
</div>
<div className="ml-auto flex flex-wrap items-center gap-2">
<SwitchRow label="Auto-start receiver" checked={draftSettings.startReceiverWhenAppOpens} onChange={(value) => onToggle("startReceiverWhenAppOpens", value)} />
<SwitchRow label="Packet stream in logs" checked={draftSettings.packetLoggingEnabled} onChange={(value) => onToggle("packetLoggingEnabled", value)} />
</div>
</div>
{virtualController.kind === "error" ? <p className="mt-3 rounded-xl border border-red-400/20 bg-red-400/10 p-3 text-sm text-red-200">{virtualController.message}</p> : null}
</section>
)
}

function virtualControllerStatusTone(status: VirtualControllerStatus): StatusTone {
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
className="flex shrink-0 items-center gap-2 rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2 text-left text-sm text-slate-200 transition hover:bg-white/[0.06]"
>
<span>{label}</span>
<span className={`flex h-6 w-11 items-center rounded-full p-1 transition ${checked ? "bg-cyan-300" : "bg-slate-700"}`}>
<span className={`h-4 w-4 rounded-full bg-slate-950 transition ${checked ? "translate-x-5" : "translate-x-0"}`} />
</span>
</button>
)
}
