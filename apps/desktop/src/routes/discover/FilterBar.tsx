import type { SkillSource } from "@skills-pp/shared";

interface Props {
  sources: SkillSource[];
  selectedSource: string;
  selectedTool: string;
  onSourceChange: (v: string) => void;
  onToolChange: (v: string) => void;
  allTools: string[];
}

export function FilterBar({
  sources,
  selectedSource,
  selectedTool,
  onSourceChange,
  onToolChange,
  allTools,
}: Props) {
  return (
    <div className="flex flex-wrap gap-3">
      <select
        className="rounded-lg border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
        value={selectedSource}
        onChange={(e) => onSourceChange(e.target.value)}
      >
        <option value="">全部来源</option>
        {sources
          .filter((s) => s.enabled)
          .map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
      </select>

      <select
        className="rounded-lg border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
        value={selectedTool}
        onChange={(e) => onToolChange(e.target.value)}
      >
        <option value="">全部工具</option>
        {allTools.map((t) => (
          <option key={t} value={t}>
            {t}
          </option>
        ))}
      </select>
    </div>
  );
}
