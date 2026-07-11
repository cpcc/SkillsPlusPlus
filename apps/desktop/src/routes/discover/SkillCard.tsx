import type { SkillItem } from "@skills-pp/shared";
import { Star } from "lucide-react";
import { useNavigate } from "react-router-dom";

interface Props {
  skill: SkillItem;
}

export function SkillCard({ skill }: Props) {
  const navigate = useNavigate();

  return (
    <button
      className="group w-full rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-4 text-left transition-all hover:border-[var(--color-border-strong)] hover:bg-[var(--color-surface-overlay)] active:scale-[0.99]"
      onClick={() =>
        navigate(`/skill/${encodeURIComponent(skill.id)}`, { state: { skill } })
      }
    >
      {/* Name + Stars */}
      <div className="flex items-center gap-2">
        <span className="truncate text-[13px] font-semibold text-[var(--color-text-primary)] group-hover:text-[var(--color-accent-text)]">
          {skill.name}
        </span>
        {skill.stars != null && (
          <span className="flex shrink-0 items-center gap-1 text-[11px] text-[var(--color-warning)]">
            <Star className="h-3 w-3" fill="currentColor" />
            {skill.stars}
          </span>
        )}
      </div>

      {/* Author */}
      {skill.author && (
        <p className="mt-0.5 text-[11px] text-[var(--color-text-tertiary)]">
          by {skill.author}
        </p>
      )}

      {/* Description */}
      {skill.description && (
        <p className="mt-2 line-clamp-2 text-[12px] leading-relaxed text-[var(--color-text-secondary)]">
          {skill.description}
        </p>
      )}

      {/* Tags（若有 category，左侧高亮徽章） */}
      <div className="mt-3 flex flex-wrap gap-1.5">
        {skill.category && (
          <span
            className="rounded-full border border-[var(--color-accent-muted)] bg-[var(--color-accent-subtle)] px-2 py-[1px] text-[11px] font-medium text-[var(--color-accent-text)]"
          >
            {skill.category}
          </span>
        )}
        {skill.tags.slice(0, 3).map((tag) => (
          <span
            key={tag}
            className="rounded-full border border-[var(--color-border-subtle)] px-2 py-[1px] text-[11px] text-[var(--color-text-tertiary)]"
          >
            {tag}
          </span>
        ))}
      </div>
    </button>
  );
}
