import type { RuntimeStatus } from "../app/types"
import { ButtonBadges } from "./ButtonBadges"
import { MetricItem } from "./MetricItem"
import { receiverStatusLabel } from "./statusFormat"

interface RuntimeStatusCardProps {
readonly status: RuntimeStatus
}

export function RuntimeStatusCard({ status }: RuntimeStatusCardProps) {
const bindAddress = status.receiver.kind === "running" ? status.receiver.boundAddress : "Not listening"
const sender = status.receiver.kind === "running" ? status.receiver.lockedSender ?? "Waiting" : "None"

return (
<section className="rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<div className="flex items-center justify-between gap-4">
<h2 className="text-sm font-semibold text-white">Status</h2>
<span className="text-xs text-muted-foreground">Live receiver state</span>
</div>
<dl className="mt-4 grid gap-3 sm:grid-cols-4">
<MetricItem label="Receiver" value={receiverStatusLabel(status.receiver)} />
<MetricItem label="Bind" value={bindAddress} />
<MetricItem label="Packets" value={status.packetCount} />
<MetricItem label="Sender" value={sender} />
</dl>
<div className="mt-5 rounded-xl border border-white/10 bg-black/20 p-4">
<p className="mb-3 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Pressed buttons</p>
<ButtonBadges buttons={status.pressedButtons} />
</div>
</section>
)
}
