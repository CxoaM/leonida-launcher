import type { ButtonHTMLAttributes, ReactNode } from "react";
import { LoaderCircle as Loader } from "lucide-react";
import { cn } from "@/lib/format";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-[#0B0D10] hover:bg-[#a3b9ff] active:bg-[#7f9df8] shadow-[0_1px_0_rgb(255_255_255/0.25)_inset,0_2px_8px_rgb(142_169_255/0.18)]",
  secondary:
    "bg-raised text-ink border border-line hover:border-[#333d4c] hover:bg-[#202834] active:bg-[#1d242e]",
  ghost: "text-muted hover:text-ink hover:bg-[#1b222b] active:bg-[#202834]",
  danger:
    "bg-[#2a1c1e] text-danger border border-[#4a2b2d] hover:bg-[#342225] hover:border-[#5c3437]",
};

const sizes: Record<Size, string> = {
  sm: "h-8 px-2.5 text-[12px] gap-1.5 rounded-[7px]",
  md: "h-9 px-3.5 text-[13px] gap-2 rounded-[var(--radius-control)]",
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  icon?: ReactNode;
  block?: boolean;
}

export function Button({
  variant = "secondary",
  size = "md",
  loading = false,
  icon,
  block = false,
  className,
  children,
  disabled,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      disabled={disabled || loading}
      className={cn(
        "focus-ring inline-flex select-none items-center justify-center font-medium whitespace-nowrap transition-[background,border-color,color,opacity] duration-150",
        "disabled:cursor-not-allowed disabled:opacity-45",
        variants[variant],
        sizes[size],
        block && "w-full",
        className,
      )}
      {...rest}
    >
      {loading ? <Loader size={size === "sm" ? 13 : 14} className="animate-spin" /> : icon}
      {children}
    </button>
  );
}

export function IconButton({
  label,
  className,
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={cn(
        "focus-ring inline-flex h-7 w-7 items-center justify-center rounded-[7px] text-muted transition-colors",
        "hover:bg-[#1f27310f] hover:text-ink disabled:cursor-not-allowed disabled:opacity-40",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
