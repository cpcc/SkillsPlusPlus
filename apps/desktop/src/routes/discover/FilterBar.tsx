import type { SkillSource } from "@skills-pp/shared";
import { ChevronDown } from "lucide-react";

interface Props {
  sources: SkillSource[];
  selectedSource: string;
  onSourceChange: (v: string) => void;
}

function SelectWrapper({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`relative inline-flex ${className ?? ""}`}>
      {children}
      <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3 w-3 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
    </div>
  );
}

const selectCls =
  "appearance-none rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] py-[5px] pl-3 pr-7 text-[12px] text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer";

export function FilterBar({
  sources,
  selectedSource,
  onSourceChange,
}: Props) {
  return (
    <div className="flex flex-wrap gap-2">
      <SelectWrapper>
        <select
          className={selectCls}
          value={selectedSource}
          onChange={(e) => onSourceChange(e.target.value)}
        >
          {sources
            .filter((s) => s.enabled)
            .map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
        </select>
      </SelectWrapper>
    </div>
  );
}
