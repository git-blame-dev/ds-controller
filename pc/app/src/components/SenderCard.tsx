import type { ReceiverStatus } from "../app/types"

interface SenderCardProps {
readonly receiver: ReceiverStatus
readonly lastPacketAt: string | null
}

export function SenderCard({ receiver, lastPacketAt }: SenderCardProps) {
const sender = receiver.kind === "running" ? receiver.lockedSender : null

return (
<article className="rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<h2 className="text-sm font-semibold text-white">Sender</h2>
<p className="mt-4 truncate text-lg font-semibold text-slate-100">{sender ?? "Waiting for DS"}</p>
<p className="mt-2 text-xs text-muted-foreground">Last packet: {formatTimestamp(lastPacketAt)}</p>
</article>
)
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
