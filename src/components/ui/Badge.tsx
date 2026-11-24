import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/format";
import type { Tone } from "@/types";

const tones: Record<Tone, string> = {
  neutral: "bg-[#1c232c] text-muted border-[#2a3340]",
  accent: "bg-accent-soft text-accent border-accent-line",
  ok: "bg-[#16251d] text-ok border-[#234333]",
  warn: "bg-[#292013] text-warn border-[#4a3a1d]",
  danger: "bg-[#2a1a1c] text-danger border-[#482a2d]",
};

export function Badge({
  tone = "neutral",
  children,
  className,
  mono = false,
  ...rest
}: HTMLAttributes<HTMLSpanElement> & { tone?: Tone; mono?: boolean }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border px-2 py-[2px] text-[11px] font-medium leading-[1.4]",
        mono && "font-mono text-[10.5px] tracking-tight",
        tones[tone],
        className,
      )}
      {...rest}
    >
      {children}
    </span>
  );
}

export function Dot({ tone = "neutral", pulse = false }: { tone?: Tone; pulse?: boolean }) {
  const color: Record<Tone, string> = {
    neutral: "text-faint bg-faint",
    accent: "text-accent bg-accent",
    ok: "text-ok bg-ok",
    warn: "text-warn bg-warn",
    danger: "text-danger bg-danger",
  };
  return (
    <span
      aria-hidden
      className={cn(
        "inline-block h-1.5 w-1.5 shrink-0 rounded-full",
        color[tone],
        pulse && "animate-pulse-dot",
      )}
    />
  );
}

export function Pill({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-[6px] border border-line-soft bg-[#141920] px-1.5 py-[1px] font-mono text-[10.5px] text-muted",
        className,
      )}
    >
      {children}
    </span>
  );
}
