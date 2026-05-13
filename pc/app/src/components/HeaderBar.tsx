import type { ReceiverStatus } from "../app/types"
import { StatusBadge } from "./StatusBadge"
import { receiverStatusLabel, receiverStatusTone } from "./statusFormat"

interface HeaderBarProps {
readonly receiver: ReceiverStatus
}

export function HeaderBar({ receiver }: HeaderBarProps) {
return (
<header className="flex items-start justify-between gap-4">
<div>
<p className="text-xs font-semibold uppercase tracking-[0.36em] text-cyan-300/80">PC Receiver</p>
<h1 className="mt-3 text-4xl font-semibold tracking-tight text-white">DS Controller</h1>
<p className="mt-2 text-sm text-muted-foreground">Use your Nintendo DS as a wireless controller.</p>
</div>
<StatusBadge label={receiverStatusLabel(receiver)} tone={receiverStatusTone(receiver)} />
</header>
)
}
