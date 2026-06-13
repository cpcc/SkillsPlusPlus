import { useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
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
    <div>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">发现</h2>
          <p className="mt-1 text-sm text-gray-500">
            {isLoading || refresh.isPending
              ? "加载中..."
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
          className="flex items-center gap-2 rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-60"
        >
          <RefreshCw
            className={`h-4 w-4 ${refresh.isPending ? "animate-spin" : ""}`}
          />
          刷新来源
        </button>
      </div>

      {/* Search */}
      <div className="relative mt-4">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          className="w-full rounded-lg border border-gray-300 py-2 pl-9 pr-4 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
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
      <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {filtered.map((s) => (
          <SkillCard key={s.id} skill={s} />
        ))}
      </div>

      {!isLoading && !refresh.isPending && filtered.length === 0 && (
        <div className="mt-16 text-center text-sm text-gray-400">
          {skills.length === 0
            ? "点击「刷新来源」加载 skill 数据"
            : "没有匹配的 skill"}
        </div>
      )}
    </div>
  );
}
