import { cn } from "../lib/utils"
import type { StatusTone } from "./statusFormat"

interface StatusBadgeProps {
readonly label: string
readonly tone: StatusTone
}

export function StatusBadge({ label, tone }: StatusBadgeProps) {
return (
<span
className={cn(
"inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-medium backdrop-blur",
tone === "good" && "border-emerald-400/30 bg-emerald-400/10 text-emerald-200",
tone === "warn" && "border-amber-400/30 bg-amber-400/10 text-amber-200",
tone === "bad" && "border-red-400/30 bg-red-400/10 text-red-200",
tone === "neutral" && "border-white/10 bg-white/5 text-slate-300",
)}
>
<span className={cn("h-2 w-2 rounded-full", dotClass(tone))} />
{label}
</span>
)
}

function dotClass(tone: StatusTone): string {
switch (tone) {
case "good":
return "bg-emerald-300 shadow-[0_0_16px_rgba(110,231,183,0.7)]"
case "warn":
return "bg-amber-300 shadow-[0_0_16px_rgba(252,211,77,0.7)]"
case "bad":
return "bg-red-300 shadow-[0_0_16px_rgba(252,165,165,0.7)]"
case "neutral":
return "bg-slate-400"
}
}
