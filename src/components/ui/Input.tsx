import { InputHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type InputProps = InputHTMLAttributes<HTMLInputElement>;

const base =
  "w-full rounded-xl border border-peach-200/80 bg-white/90 px-4 py-3 text-sm font-medium text-ink-900 shadow-inner shadow-peach-300/15 transition focus:border-peach-300 focus:outline-none focus:ring-2 focus:ring-peach-300/60";

export function Input({ className, ...props }: InputProps) {
  return <input className={cn(base, className)} {...props} />;
}
