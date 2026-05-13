import type { ViGemStatus } from "../app/types"
import { StatusBadge } from "./StatusBadge"
import { viGemLabel } from "./statusFormat"

interface ViGemStatusCardProps {
readonly status: ViGemStatus
}

export function ViGemStatusCard({ status }: ViGemStatusCardProps) {
const tone = status.kind === "ready" ? "good" : status.kind === "error" ? "bad" : "neutral"

return (
<article className="rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<div className="flex items-center justify-between gap-3">
<h2 className="text-sm font-semibold text-white">ViGEm</h2>
<StatusBadge label={viGemLabel(status)} tone={tone} />
</div>
<p className="mt-4 text-sm text-muted-foreground">
{status.kind === "ready" ? "Virtual Xbox 360 controller ready." : status.kind === "error" ? status.message : "Waiting for backend status."}
</p>
</article>
)
}
