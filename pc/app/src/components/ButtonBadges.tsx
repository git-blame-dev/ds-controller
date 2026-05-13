import type { DsButton } from "../app/types"

const BUTTON_LABELS: Record<DsButton, string> = {
a: "A",
b: "B",
x: "X",
y: "Y",
l: "L",
r: "R",
start: "Start",
select: "Select",
up: "Up",
down: "Down",
left: "Left",
right: "Right",
}

interface ButtonBadgesProps {
readonly buttons: readonly DsButton[]
}

export function ButtonBadges({ buttons }: ButtonBadgesProps) {
if (buttons.length === 0) {
return <p className="text-sm text-muted-foreground">No buttons pressed</p>
}

return (
<div className="flex flex-wrap gap-2">
{buttons.map((button) => (
<span key={button} className="rounded-full border border-cyan-300/25 bg-cyan-300/10 px-3 py-1 text-xs font-semibold text-cyan-100">
{BUTTON_LABELS[button]}
</span>
))}
</div>
)
}
