import { useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { RefreshCw, Search, Sparkles } from "lucide-react";
import type { SkillItem } from "@skills-pp/shared";
import {
  useSkills,
  useRefreshAllSources,
  useOnlineSearch,
} from "../../hooks/use-skills";
import { useSources } from "../../hooks/use-sources";
import { SkillCard } from "./SkillCard";
import { FilterBar } from "./FilterBar";
import { useToast } from "../../components/ui/toast";

const PAGE = 24;

function useDebounced<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(t);
  }, [value, delayMs]);
  return debounced;
}

/** Infinite-scroll hook: slices `items` into pages, returns visible slice + sentinel ref. */
function useInfiniteScroll<T>(items: T[], resetKey: string) {
  const sentinelRef = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(PAGE);

  useEffect(() => {
    setVisible(PAGE);
  }, [resetKey]);

  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const io = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        setVisible((v) => Math.min(v + PAGE, items.length));
      }
    });
    io.observe(el);
    return () => io.disconnect();
  }, [items.length, visible]);

  return {
    slice: items.slice(0, visible),
    sentinelRef,
    hasMore: visible < items.length,
  };
}

export default function DiscoverPage() {
  const { data: skills = [], isLoading } = useSkills();
  const { data: sources = [] } = useSources();
  const refresh = useRefreshAllSources();
  const toast = useToast();

  const [searchParams, setSearchParams] = useSearchParams();
  const query = searchParams.get("q") ?? "";
  const setQuery = (q: string) =>
    setSearchParams(q ? { q } : {}, { replace: true });
  const [selectedSource, setSelectedSource] = useState("skills_sh");
  const [selectedCategory, setSelectedCategory] = useState("全部");
  const debouncedQuery = useDebounced(query, 300);

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
    const q = debouncedQuery.toLowerCase();
    return skills.filter((s) => {
      if (selectedSource && s.sourceId !== selectedSource) return false;
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
  }, [skills, debouncedQuery, selectedSource]);

  // 在线搜索：查询长度 >= 2 时始终与本地并行搜索
  const enableOnline = debouncedQuery.trim().length >= 2;
  const onlineQuery = useOnlineSearch(debouncedQuery, enableOnline);
  const rawOnline: SkillItem[] = enableOnline ? (onlineQuery.data ?? []) : [];

  // 在线结果去重：排除本地已有的 skill（按 repoUrl + name 判定）
  const onlineResults = useMemo(() => {
    if (rawOnline.length === 0) return [];
    const localKeys = new Set(filtered.map((s) => `${s.repoUrl ?? ""}|${s.name}`));
    return rawOnline.filter((s) => !localKeys.has(`${s.repoUrl ?? ""}|${s.name}`));
  }, [rawOnline, filtered]);

  // 独立分页
  const localScroll = useInfiniteScroll(filtered, `${debouncedQuery}|${selectedSource}`);
  const onlineScroll = useInfiniteScroll(onlineResults, debouncedQuery);

  const showOnlineSection =
    enableOnline && !onlineQuery.isLoading && onlineResults.length > 0;
  const showEmptyState =
    filtered.length === 0 &&
    onlineResults.length === 0 &&
    !onlineQuery.isLoading &&
    (debouncedQuery.trim().length < 2 || enableOnline);

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
          onSourceChange={setSelectedSource}
          selectedCategory={selectedCategory}
          onCategoryChange={setSelectedCategory}
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
          {/* 本地结果 */}
          {filtered.length > 0 && (
            <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {localScroll.slice.map((s) => (
                <SkillCard key={s.id} skill={s} />
              ))}
            </div>
          )}
          {localScroll.hasMore && (
            <div ref={localScroll.sentinelRef} className="h-4" />
          )}

          {/* 在线搜索加载 */}
          {enableOnline && onlineQuery.isLoading && (
            <div className="mt-4 flex items-center gap-2 text-[13px] text-[var(--color-text-tertiary)]">
              <RefreshCw className="h-3 w-3 animate-spin" />
              正在搜索 skills.sh...
            </div>
          )}

          {/* 在线搜索结果 */}
          {showOnlineSection && (
            <>
              <p className="mt-6 text-[12px] font-medium uppercase tracking-wide text-[var(--color-text-tertiary)]">
                来自 skills.sh 的在线搜索结果（{onlineResults.length}）
              </p>
              <div className="mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {onlineScroll.slice.map((s) => (
                  <SkillCard key={s.id} skill={s} />
                ))}
              </div>
              {onlineScroll.hasMore && (
                <div ref={onlineScroll.sentinelRef} className="h-4" />
              )}
            </>
          )}

          {showEmptyState && (
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
