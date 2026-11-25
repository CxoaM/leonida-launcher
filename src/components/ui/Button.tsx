import type { ButtonHTMLAttributes, ReactNode } from "react";
import { LoaderCircle as Loader } from "lucide-react";
import { cn } from "@/lib/format";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

const variants: Record<Variant, string> = {
  primary:
    "bg-ink text-base hover:bg-[#fbf7f0] active:bg-[#ddd6cc] disabled:hover:bg-ink",
  secondary:
    "border border-line bg-transparent text-ink hover:border-[#423d35] hover:bg-raised active:bg-[#26231f]",
  ghost: "text-muted hover:bg-raised hover:text-ink active:bg-[#26231f]",
  danger:
    "border border-[#4a312e] bg-[#241a18] text-danger hover:border-[#603a35] hover:bg-[#2d201e]",
};

const sizes: Record<Size, string> = {
  sm: "h-7 gap-1.5 rounded-[7px] px-2.5 text-[12px]",
  md: "h-8 gap-2 rounded-[var(--radius-control)] px-3 text-[12.5px]",
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
        "focus-ring inline-flex select-none items-center justify-center font-medium whitespace-nowrap transition-[background-color,border-color,color,opacity] duration-150",
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
        "focus-ring inline-flex h-7 w-7 items-center justify-center rounded-[6px] text-muted transition-colors",
        "hover:bg-raised hover:text-ink disabled:cursor-not-allowed disabled:opacity-40",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
