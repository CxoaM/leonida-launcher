import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/format";
import type { Tone } from "@/types";

const tones: Record<Tone, string> = {
  neutral: "text-muted",
  accent: "text-accent",
  ok: "text-ok",
  warn: "text-warn",
  danger: "text-danger",
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
        "inline-flex items-center gap-1.5 whitespace-nowrap text-[11px] font-medium leading-[1.4]",
        mono && "font-mono text-[10.5px]",
        tones[tone],
        className,
      )}
      {...rest}
    >
      <span aria-hidden className="h-1.5 w-1.5 shrink-0 rounded-full bg-current opacity-80" />
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
        pulse && "animate-live",
      )}
    />
  );
}

export function Pill({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-center font-mono text-[10.5px] leading-[1.4] text-faint",
        className,
      )}
    >
      {children}
    </span>
  );
}
