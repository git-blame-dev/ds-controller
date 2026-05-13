interface MetricItemProps {
readonly label: string
readonly value: string | number
}

export function MetricItem({ label, value }: MetricItemProps) {
return (
<div className="rounded-xl border border-white/10 bg-white/[0.03] p-4">
<dt className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">{label}</dt>
<dd className="mt-2 truncate text-sm font-semibold text-slate-100">{value}</dd>
</div>
)
}
