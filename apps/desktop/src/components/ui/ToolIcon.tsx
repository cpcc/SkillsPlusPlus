import type { ComponentType } from "react";
import {
  Claude,
  Cursor,
  Codex,
  Gemini,
  Kimi,
  OpenCode,
  Antigravity,
  OpenClaw,
  CodeBuddy,
  GithubCopilot,
  Amp,
  Cline,
} from "@lobehub/icons";

type ToolIconSize = "xs" | "sm" | "md";

const SIZE_MAP: Record<ToolIconSize, number> = {
  xs: 14,
  sm: 16,
  md: 20,
};

/**
 * 规范顺序的 Agents 通用目录共享工具列表（与 DirectoryCard.tsx tooltip 保持一致）。
 * 共 14 个，UI 中展示前 5 个品牌图标 + 「+N」徽标。
 */
const AGENTS_TOOL_NAMES = [
  "Amp",
  "Antigravity",
  "Cline",
  "Codex",
  "Cursor",
  "Deep Agents",
  "Dexto",
  "Firebender",
  "Gemini CLI",
  "GitHub Copilot",
  "Kimi Code CLI",
  "OpenCode",
  "Warp",
  "Zed",
] as const;

const VISIBLE_COUNT = 5;

/** Maps canonical tool_name to brand icon + whether it has a .Color variant. */
const TOOL_ICON_MAP: Record<
  string,
  { Icon: ComponentType<{ size?: number; className?: string; style?: React.CSSProperties }>; hasColor: boolean }
> = {
  claude: { Icon: Claude, hasColor: true },
  cursor: { Icon: Cursor, hasColor: false },
  codex: { Icon: Codex, hasColor: true },
  "github copilot": { Icon: GithubCopilot, hasColor: false },
  opencode: { Icon: OpenCode, hasColor: false },
  antigravity: { Icon: Antigravity, hasColor: true },
  "antigravity cli": { Icon: Antigravity, hasColor: true },
  "gemini cli": { Icon: Gemini, hasColor: true },
  "kimi code cli": { Icon: Kimi, hasColor: true },
  openclaw: { Icon: OpenClaw, hasColor: true },
  codebuddy: { Icon: CodeBuddy, hasColor: true },
  amp: { Icon: Amp, hasColor: true },
  cline: { Icon: Cline, hasColor: false },
};

/** Fallback monogram for unknown tools. */
const FALLBACK_META: Record<string, { bg: string; mono: string }> = {
  claude: { bg: "#CC785C", mono: "Cl" },
  cursor: { bg: "#111111", mono: "Cu" },
  codex: { bg: "#0D1117", mono: "Cd" },
  opencode: { bg: "#6E56CF", mono: "Oc" },
  "github copilot": { bg: "#1F2328", mono: "Co" },
  antigravity: { bg: "#4285F4", mono: "An" },
  "antigravity cli": { bg: "#4285F4", mono: "AC" },
  "gemini cli": { bg: "#1FA8F4", mono: "Ge" },
  "kimi code cli": { bg: "#1D1D1F", mono: "Km" },
  openclaw: { bg: "#FF6B35", mono: "OC" },
  codebuddy: { bg: "#0053E0", mono: "CB" },
  // 通用共享目录（无品牌图标，使用 Indigo 强调色作为视觉标识）
  agents: { bg: "#6366f1", mono: "Ag" },
  amp: { bg: "#000000", mono: "Am" },
  cline: { bg: "#9b88f3", mono: "Cl" },
  warp: { bg: "#01A4FF", mono: "Wa" },
  // Agents 目录中缺失品牌图标的工具（颜色基于品牌猜测）
  "deep agents": { bg: "#1A1A1A", mono: "DA" },
  dexto: { bg: "#7C3AED", mono: "Dx" },
  firebender: { bg: "#FF6B35", mono: "Fb" },
  zed: { bg: "#08A5E0", mono: "Z" },
};

const MONO_SIZE_CLS: Record<ToolIconSize, string> = {
  xs: "h-3.5 w-3.5 text-[8px] rounded-[5px]",
  sm: "h-4 w-4 text-[9px] rounded-[6px]",
  md: "h-5 w-5 text-[11px] rounded-[7px]",
};

/**
 * Agents 通用目录的图标簇：前 5 个品牌图标重叠堆叠 + 「+N」徽标。
 * 用于替换单个 Boxes 图标，直观表达「14 个 AI 工具共享读取」的语义。
 */
function AgentsDirectoryIcon({ className }: { className?: string }) {
  const visible = AGENTS_TOOL_NAMES.slice(0, VISIBLE_COUNT);
  const rest = AGENTS_TOOL_NAMES.length - VISIBLE_COUNT;
  const restList = AGENTS_TOOL_NAMES.slice(VISIBLE_COUNT).join(" / ");
  return (
    <span
      className={`inline-flex items-center ${className ?? ""}`}
      aria-label="Agents 通用目录（共享读取的 AI 工具）"
      role="img"
    >
      {visible.map((name, i) => (
        <span
          key={name}
          className="inline-flex items-center justify-center rounded-full bg-[var(--color-surface-raised)] ring-1 ring-[var(--color-border-subtle)]"
          style={{ marginLeft: i === 0 ? 0 : -6, zIndex: visible.length - i }}
        >
          <ToolIcon toolName={name} size="xs" />
        </span>
      ))}
      <span
        className="ml-1 inline-flex h-[14px] items-center rounded-full bg-[var(--color-accent-subtle)] px-1 text-[9px] font-semibold text-[var(--color-accent-text)]"
        title={`另外 ${rest} 个：${restList}`}
      >
        +{rest}
      </span>
    </span>
  );
}

export function ToolIcon({
  toolName,
  size = "sm",
  className = "",
}: {
  toolName: string;
  size?: ToolIconSize;
  className?: string;
}) {
  const key = toolName?.trim().toLowerCase();

  // Agents 通用目录：走图标簇而非单图标
  if (key === "agents") {
    return <AgentsDirectoryIcon className={className} />;
  }

  const px = SIZE_MAP[size];
  const entry = key ? TOOL_ICON_MAP[key] : undefined;

  if (entry) {
    const { Icon, hasColor } = entry;
    if (hasColor) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const ColorIcon = (Icon as any).Color as ComponentType<{ size?: number; className?: string }> | undefined;
      if (ColorIcon) {
        return <ColorIcon size={px} className={`shrink-0 ${className}`} />;
      }
    }
    // Mono variant + brand color via CSS
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const colorPrimary = (Icon as any).colorPrimary as string | undefined;
    return <Icon size={px} className={`shrink-0 ${className}`} style={{ color: colorPrimary }} />;
  }

  // Unknown tool: monogram fallback
  const meta = key ? FALLBACK_META[key] : undefined;
  const bg = meta?.bg ?? "#6E7681";
  const label = meta?.mono ?? (toolName?.trim()?.[0] ?? "?").toUpperCase();

  return (
    <span
      className={`inline-flex items-center justify-center font-bold leading-none text-white shrink-0 ${MONO_SIZE_CLS[size]} ${className}`}
      style={{ backgroundColor: bg }}
      aria-hidden
    >
      {label}
    </span>
  );
}
