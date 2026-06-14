import { useMemo } from "react";
import { marked } from "marked";
import { FileText, AlertCircle } from "lucide-react";

interface Props {
  markdown: string | null | undefined;
  isLoading: boolean;
  isError: boolean;
}

export function SkillMdContent({ markdown, isLoading, isError }: Props) {
  const html = useMemo(
    () =>
      markdown
        ? marked.parse(markdown, { breaks: true, gfm: true }) as string
        : null,
    [markdown],
  );

  if (isLoading) {
    return (
      <div className="animate-pulse space-y-3 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-5">
        <div className="h-3 w-3/4 rounded bg-[var(--color-border-subtle)]" />
        <div className="h-3 w-full rounded bg-[var(--color-border-subtle)]" />
        <div className="h-3 w-5/6 rounded bg-[var(--color-border-subtle)]" />
        <div className="h-3 w-2/3 rounded bg-[var(--color-border-subtle)]" />
        <div className="h-3 w-4/5 rounded bg-[var(--color-border-subtle)]" />
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex items-center gap-3 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-danger-subtle)]">
          <AlertCircle className="h-4 w-4 text-[var(--color-danger)]" />
        </div>
        <p className="text-[13px] text-[var(--color-text-secondary)]">
          加载失败
        </p>
      </div>
    );
  }

  if (!html) {
    return (
      <div className="flex items-center gap-3 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-surface-hover)]">
          <FileText className="h-4 w-4 text-[var(--color-text-tertiary)]" />
        </div>
        <p className="text-[13px] text-[var(--color-text-tertiary)]">
          该 Skill 未提供 SKILL.md
        </p>
      </div>
    );
  }

  return (
    <div
      className="prose-skill-md rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-5"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
