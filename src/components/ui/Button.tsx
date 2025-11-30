import { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type ButtonVariant = "primary" | "secondary" | "danger";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
};

const base =
  "inline-flex items-center justify-center rounded-xl px-4 py-2 text-sm font-semibold transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 disabled:cursor-not-allowed disabled:opacity-60";

const variants: Record<ButtonVariant, string> = {
  primary:
    "border border-peach-300 bg-peach-300 text-ink-900 shadow-md shadow-peach-400/40 hover:-translate-y-0.5 hover:bg-peach-400 hover:text-white focus-visible:outline-peach-400",
  secondary:
    "border border-peach-200/80 bg-white/90 text-ink-900 shadow-sm shadow-peach-300/25 hover:-translate-y-0.5 hover:border-peach-300 hover:bg-peach-50 focus-visible:outline-peach-300",
  danger:
    "border border-rose-500 bg-rose-500 text-white shadow-md shadow-rose-400/40 hover:-translate-y-0.5 hover:bg-rose-600 focus-visible:outline-rose-400",
};

export function Button({ variant = "primary", className, type = "button", ...props }: ButtonProps) {
  return <button type={type} className={cn(base, variants[variant], className)} {...props} />;
}
