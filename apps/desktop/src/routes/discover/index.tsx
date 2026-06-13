import { useEffect, useMemo, useState } from "react";
import { RefreshCw, Search, Sparkles } from "lucide-react";
import type { SkillItem } from "@skills-pp/shared";
import { useSkills, useRefreshAllSources } from "../../hooks/use-skills";
import { useSources } from "../../hooks/use-sources";
import { SkillCard } from "./SkillCard";
import { FilterBar } from "./FilterBar";
import { useToast } from "../../components/ui/toast";

function useUniqueTools(skills: SkillItem[]) {
  return useMemo(() => {
    const set = new Set<string>();
    for (const s of skills) s.compatibleTools?.forEach((t) => set.add(t));
    return Array.from(set).sort();
  }, [skills]);
}

export default function DiscoverPage() {
  const { data: skills = [], isLoading } = useSkills();
  const { data: sources = [] } = useSources();
  const refresh = useRefreshAllSources();
  const toast = useToast();

  const [query, setQuery] = useState("");
  const [selectedSource, setSelectedSource] = useState("");
  const [selectedTool, setSelectedTool] = useState("");

  const allTools = useUniqueTools(skills);

  // Auto-refresh on first mount if cache is empty
  useEffect(() => {
    if (!isLoading && skills.length === 0) {
      refresh.mutate(undefined, {
        onError: (e) => toast("刷新失败", String(e), "error"),
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLoading]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return skills.filter((s) => {
      if (selectedSource && s.sourceId !== selectedSource) return false;
      if (selectedTool && !s.compatibleTools?.includes(selectedTool)) return false;
      if (q) {
        return (
          s.name.toLowerCase().includes(q) ||
          s.description?.toLowerCase().includes(q) ||
          s.author?.toLowerCase().includes(q) ||
          s.tags.some((t) => t.toLowerCase().includes(q))
        );
      }
      return true;
    });
  }, [skills, query, selectedSource, selectedTool]);

  return (
    <div className="mx-auto max-w-[960px]">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
            发现
          </h1>
          <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
            {isLoading || refresh.isPending
              ? "正在加载..."
              : `${filtered.length} / ${skills.length} 个 skill`}
          </p>
        </div>
        <button
          onClick={() =>
            refresh.mutate(undefined, {
              onError: (e) => toast("刷新失败", String(e), "error"),
            })
          }
          disabled={refresh.isPending}
          className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-[6px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] disabled:opacity-40"
        >
          <RefreshCw
            className={`h-3.5 w-3.5 ${refresh.isPending ? "animate-spin" : ""}`}
          />
          刷新来源
        </button>
      </div>

      {/* Search */}
      <div className="relative mt-5">
        <Search className="absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
        <input
          type="text"
          className="w-full rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] py-2 pl-9 pr-4 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] transition-colors focus:border-[var(--color-accent)] focus:outline-none"
          placeholder="搜索 skill 名称、描述或标签..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {/* Filters */}
      <div className="mt-3">
        <FilterBar
          sources={sources}
          selectedSource={selectedSource}
          selectedTool={selectedTool}
          onSourceChange={setSelectedSource}
          onToolChange={setSelectedTool}
          allTools={allTools}
        />
      </div>

      {/* Results */}
      {isLoading ? (
        <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="animate-pulse rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-4"
            >
              <div className="h-4 w-24 rounded bg-[var(--color-border-subtle)]" />
              <div className="mt-2 h-3 w-32 rounded bg-[var(--color-border-subtle)]" />
              <div className="mt-3 h-3 w-full rounded bg-[var(--color-border-subtle)]" />
              <div className="mt-1.5 h-3 w-20 rounded bg-[var(--color-border-subtle)]" />
            </div>
          ))}
        </div>
      ) : (
        <>
          <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {filtered.map((s) => (
              <SkillCard key={s.id} skill={s} />
            ))}
          </div>

          {filtered.length === 0 && (
            <div className="mt-20 flex flex-col items-center gap-3 text-center">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--color-surface-raised)] border border-[var(--color-border-subtle)]">
                <Sparkles className="h-5 w-5 text-[var(--color-text-tertiary)]" />
              </div>
              <div>
                <p className="text-[13px] font-medium text-[var(--color-text-secondary)]">
                  {skills.length === 0 ? "暂无 skill 数据" : "没有匹配的 skill"}
                </p>
                <p className="mt-1 text-[12px] text-[var(--color-text-tertiary)]">
                  {skills.length === 0
                    ? "点击「刷新来源」从各站点加载 skill"
                    : "尝试调整搜索关键词或筛选条件"}
                </p>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
