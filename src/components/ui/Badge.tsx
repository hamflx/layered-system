import { HTMLAttributes } from "react";
import { cn } from "../../lib/cn";
import { BadgeTone } from "../../lib/tone";

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: BadgeTone;
};

const base = "inline-flex items-center gap-1 rounded-full px-3 py-1 text-xs font-semibold shadow-sm";

const tones: Record<BadgeTone, string> = {
  positive: "border border-emerald-200 bg-emerald-50 text-emerald-700",
  warn: "border border-amber-200 bg-amber-50 text-amber-700",
  danger: "border border-rose-200 bg-rose-50 text-rose-700",
  info: "border border-peach-200 bg-peach-50 text-ink-900",
  neutral: "border border-peach-200/80 bg-white/90 text-ink-900",
  muted: "border border-peach-200 bg-white/70 text-ink-700",
};

export function Badge({ tone = "neutral", className, ...props }: BadgeProps) {
  return <span className={cn(base, tones[tone], className)} {...props} />;
}
