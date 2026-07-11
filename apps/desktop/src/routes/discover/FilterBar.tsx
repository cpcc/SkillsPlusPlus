import { useLayoutEffect, useRef, useState } from "react";
import type { SkillSource } from "@skills-pp/shared";
import { ChevronDown } from "lucide-react";

interface Props {
  sources: SkillSource[];
  selectedSource: string;
  onSourceChange: (v: string) => void;
  selectedCategory: string;
  onCategoryChange: (v: string) => void;
}

const CATEGORIES = [
  "全部", "自媒体", "金融", "法律", "互联网", "科研", "教育",
  "健康医疗", "通用工具", "办公效率", "内容创作", "开发编程",
  "数据分析", "知识管理", "商业运营", "IT 运维与安全", "生活服务", "其它",
];

const MORE_BTN_RESERVE = 56; // 「展开」按钮预留宽度（含 gap）
const GAP = 4; // gap-1 ≈ 4px

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

const tabCls = (active: boolean) =>
  `text-[12px] px-3 py-[5px] rounded-[var(--radius-md)] whitespace-nowrap transition-colors ${
    active
      ? "text-[var(--color-accent-text)] bg-[var(--color-accent-subtle)]"
      : "text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
  }`;

const moreCls =
  "flex items-center gap-1 text-[12px] px-3 py-[5px] rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] transition-colors whitespace-nowrap ml-auto shrink-0";

export function FilterBar({
  sources,
  selectedSource,
  onSourceChange,
  selectedCategory,
  onCategoryChange,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const widthsRef = useRef<number[]>([]);
  // 未展开时本应可见的 tab 数（与 expanded 解耦，决定是否需要「展开/收起」按钮）
  const [collapsedVisibleCount, setCollapsedVisibleCount] = useState<number>(
    CATEGORIES.length,
  );
  const [expanded, setExpanded] = useState(false);

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // 首次：记录每个 tab 的 offsetWidth（文字不变，永久缓存）
    if (widthsRef.current.length !== CATEGORIES.length) {
      const widths: number[] = [];
      for (let i = 0; i < CATEGORIES.length; i++) {
        widths.push(tabRefs.current[i]?.offsetWidth ?? 0);
      }
      widthsRef.current = widths;
    }

    // 计算「未展开」状态下能放下的 tab 数（与当前 expanded 无关）
    const compute = () => {
      const containerWidth = container.clientWidth;
      const limit = containerWidth - MORE_BTN_RESERVE;
      let acc = 0;
      let count = 0;
      for (let i = 0; i < CATEGORIES.length; i++) {
        const w = widthsRef.current[i] ?? 0;
        if (i > 0) acc += GAP;
        if (acc + w > limit) break;
        acc += w;
        count = i + 1;
      }
      setCollapsedVisibleCount(count);
    };

    compute();

    const ro = new ResizeObserver(() => compute());
    ro.observe(container);
    return () => ro.disconnect();
  }, []);

  const hasOverflow = collapsedVisibleCount < CATEGORIES.length;
  // 未展开时只显示前 collapsedVisibleCount 个；展开时全部可见（由 flex-wrap 自然换行）
  const visibleCount = expanded ? CATEGORIES.length : collapsedVisibleCount;

  return (
    <nav className="border-b border-[var(--color-border-subtle)] pb-2">
      <div className="flex items-center gap-3">
        <SelectWrapper>
          <select
            key={`${selectedSource}:${sources.filter((s) => s.enabled).map((s) => s.id).join("|")}`}
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

        <div
          ref={containerRef}
          className={`flex flex-1 items-center gap-1 min-w-0 ${
            expanded ? "flex-wrap" : "overflow-hidden"
          }`}
        >
          {CATEGORIES.map((cat, i) => (
            <button
              key={cat}
              ref={(el) => {
                tabRefs.current[i] = el;
              }}
              hidden={!expanded && i >= visibleCount}
              onClick={() => onCategoryChange(cat)}
              className={tabCls(selectedCategory === cat)}
            >
              {cat}
            </button>
          ))}
          {hasOverflow && (
            <button
              onClick={() => setExpanded((v) => !v)}
              className={moreCls}
              aria-expanded={expanded}
            >
              {expanded ? "收起" : "展开"}
              <ChevronDown
                className={`h-3 w-3 ${expanded ? "rotate-180" : ""}`}
              />
            </button>
          )}
        </div>
      </div>
    </nav>
  );
}
