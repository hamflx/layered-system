import { HTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type CardProps = HTMLAttributes<HTMLDivElement>;

const base = "rounded-2xl border border-white/60 bg-white/80 p-5 shadow-lg shadow-peach-300/25 backdrop-blur";

export function Card({ className, ...props }: CardProps) {
  return <div className={cn(base, className)} {...props} />;
}
