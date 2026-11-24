import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { Eye, FolderOpen } from "lucide-react";
import { cn } from "@/lib/format";
import { pickFolder, revealPath } from "@/lib/tauri";
import { useT } from "@/i18n";
import { IconButton } from "./Button";

export function Field({
  label,
  hint,
  htmlFor,
  children,
  className,
  action,
  tone,
}: {
  label: ReactNode;
  hint?: ReactNode;
  htmlFor?: string;
  children: ReactNode;
  className?: string;
  action?: ReactNode;
  tone?: "danger" | "warn";
}) {
  return (
    <div className={cn("space-y-1.5", className)}>
      <div className="flex items-center justify-between gap-3">
        <label
          htmlFor={htmlFor}
          className={cn(
            "text-[12.5px] font-medium text-ink",
            tone === "danger" && "text-danger",
            tone === "warn" && "text-warn",
          )}
        >
          {label}
        </label>
        {action}
      </div>
      {children}
      {hint ? <p className="text-[11.5px] leading-[1.45] text-faint">{hint}</p> : null}
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled = false,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-start justify-between gap-6 py-3">
      <div className="min-w-0">
        <div className="text-[12.5px] font-medium text-ink">{label}</div>
        {description ? (
          <p className="mt-0.5 max-w-[52ch] text-[11.5px] leading-[1.45] text-faint">{description}</p>
        ) : null}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={typeof label === "string" ? label : undefined}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "focus-ring relative h-[22px] w-[38px] shrink-0 rounded-full border transition-colors duration-150",
          checked ? "border-accent/50 bg-accent/85" : "border-line bg-[#232b35]",
          disabled && "cursor-not-allowed opacity-45",
        )}
      >
        <span
          className={cn(
            "absolute top-[2px] h-[16px] w-[16px] rounded-full bg-[#0b0d10] shadow-sm transition-all duration-150",
            checked ? "left-[19px] bg-[#0b0d10]" : "left-[2px] bg-[#8d97a5]",
          )}
        />
      </button>
    </div>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  meta,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
  meta?: ReactNode;
}) {
  const id = useId();
  return (
    <label
      htmlFor={id}
      className={cn(
        "flex cursor-pointer items-start gap-3 rounded-[10px] border px-3 py-2.5 transition-colors",
        checked ? "border-accent/35 bg-[#141a22]" : "border-line-soft bg-[#11161c] hover:border-line",
        disabled && "cursor-not-allowed opacity-60",
      )}
    >
      <input
        id={id}
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span
        aria-hidden
        className={cn(
          "mt-[1px] flex h-[16px] w-[16px] shrink-0 items-center justify-center rounded-[4px] border transition-colors",
          checked ? "border-accent bg-accent text-[#0b0d10]" : "border-[#39424f] bg-[#0d1116]",
        )}
      >
        {checked ? (
          <svg width="10" height="8" viewBox="0 0 10 8" fill="none">
            <path
              d="M1 4.2 3.6 6.8 9 1.2"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        ) : null}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center justify-between gap-3">
          <span className="text-[12.5px] font-medium text-ink">{label}</span>
          {meta}
        </span>
        {description ? (
          <span className="mt-0.5 block text-[11.5px] leading-[1.45] text-faint">{description}</span>
        ) : null}
      </span>
    </label>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  disabled = false,
}: {
  value: T;
  options: Array<{ value: T; label: ReactNode }>;
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <div
      className={cn(
        "inline-flex rounded-[9px] border border-line bg-[#0d1116] p-[3px]",
        disabled && "opacity-50",
      )}
      role="radiogroup"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          disabled={disabled}
          onClick={() => onChange(option.value)}
          className={cn(
            "focus-ring rounded-[6px] px-2.5 py-1 text-[12px] font-medium transition-colors",
            option.value === value
              ? "bg-[#252e3a] text-ink shadow-[inset_0_1px_0_rgb(255_255_255/0.04)]"
              : "text-muted hover:text-ink",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/**
 * Text input that keeps a local draft while typing and only commits on blur or
 * Enter, so a half-typed path never reaches the backend.
 */
export function PathField({
  value,
  onCommit,
  placeholder,
  disabled = false,
  invalid = false,
  hint,
  action,
  className,
  allowReveal = true,
}: {
  value: string;
  onCommit: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  invalid?: boolean;
  hint?: ReactNode;
  action?: ReactNode;
  className?: string;
  allowReveal?: boolean;
}) {
  const t = useT();
  const [draft, setDraft] = useState(value);
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) setDraft(value);
  }, [value]);

  const commit = () => {
    const next = draft.trim();
    if (next !== value) onCommit(next);
  };

  const browse = async () => {
    const picked = await pickFolder(draft || value);
    if (picked) {
      setDraft(picked);
      onCommit(picked);
    }
  };

  return (
    <div className={cn("space-y-1.5", className)}>
      <div className="flex items-stretch gap-1.5">
        <input
          className={cn("input input-mono min-w-0 flex-1", invalid && "border-danger/60")}
          value={draft}
          title={draft || undefined}
          placeholder={placeholder ?? t("common.path")}
          disabled={disabled}
          spellCheck={false}
          onChange={(event) => setDraft(event.target.value)}
          onFocus={() => {
            focused.current = true;
          }}
          onBlur={() => {
            focused.current = false;
            commit();
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
              (event.target as HTMLInputElement).blur();
            }
            if (event.key === "Escape") {
              setDraft(value);
              (event.target as HTMLInputElement).blur();
            }
          }}
        />
        {allowReveal ? (
          <IconButton
            label={t("common.open")}
            disabled={!value}
            onClick={() => void revealPath(value)}
            className="h-9 w-9 rounded-[var(--radius-control)] border border-line bg-[#0d1116]"
          >
            <Eye size={14} />
          </IconButton>
        ) : null}
        <IconButton
          label={t("common.browse")}
          disabled={disabled}
          onClick={() => void browse()}
          className="h-9 w-9 rounded-[var(--radius-control)] border border-line bg-[#0d1116]"
        >
          <FolderOpen size={14} />
        </IconButton>
      </div>
      {hint ? <p className="text-[11.5px] leading-[1.45] text-faint">{hint}</p> : null}
      {action}
    </div>
  );
}
