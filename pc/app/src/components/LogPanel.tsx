import { useEffect, useRef } from "react"

import type { LogEntry } from "../app/types"
import { cn } from "../lib/utils"

interface LogPanelProps {
readonly logs: readonly LogEntry[]
}

export function LogPanel({ logs }: LogPanelProps) {
const endRef = useRef<HTMLDivElement | null>(null)

useEffect(() => {
endRef.current?.scrollIntoView({ block: "end" })
}, [logs])

return (
<section className="min-h-0 rounded-2xl border border-white/10 bg-card/80 p-5 shadow-2xl shadow-black/25 backdrop-blur-xl">
<div className="flex items-center justify-between gap-4">
<h2 className="text-sm font-semibold text-white">Logs</h2>
<span className="text-xs text-muted-foreground">Latest {logs.length} entries</span>
</div>
<div className="mt-4 h-64 overflow-y-auto rounded-xl border border-white/10 bg-black/40 p-4 font-mono text-xs leading-5">
{logs.length === 0 ? <p className="text-muted-foreground">No logs yet.</p> : null}
{logs.map((entry) => (
<p key={entry.id} className={cn("break-words", levelClass(entry.level))}>
<span className="text-slate-500">[{formatTimestamp(entry.timestamp)}]</span> <span className="uppercase">{entry.level}</span> {entry.message}
</p>
))}
<div ref={endRef} />
</div>
</section>
)
}

function levelClass(level: LogEntry["level"]): string {
switch (level) {
case "packet":
return "text-cyan-200"
case "warning":
return "text-amber-200"
case "error":
return "text-red-200"
case "info":
return "text-slate-300"
}
}

function formatTimestamp(timestamp: string): string {
const millis = Number(timestamp)
if (!Number.isFinite(millis)) {
return timestamp
}

return new Date(millis).toLocaleTimeString()
}
