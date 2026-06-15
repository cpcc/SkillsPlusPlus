import type { ComponentType } from "react";
import {
  Claude,
  Cursor,
  OpenAI,
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
import { Boxes } from "lucide-react";

type ToolIconSize = "xs" | "sm" | "md";

const SIZE_MAP: Record<ToolIconSize, number> = {
  xs: 14,
  sm: 16,
  md: 20,
};

/** Maps canonical tool_name to brand icon + whether it has a .Color variant. */
const TOOL_ICON_MAP: Record<
  string,
  { Icon: ComponentType<{ size?: number; className?: string; style?: React.CSSProperties }>; hasColor: boolean }
> = {
  claude: { Icon: Claude, hasColor: true },
  cursor: { Icon: Cursor, hasColor: false },
  codex: { Icon: OpenAI, hasColor: false },
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
  // 通用共享目录：使用 lucide-react 的 Boxes 图标作为视觉标识
  // （Indigo 强调色由 CSS var 控制）
  agents: { Icon: Boxes, hasColor: false },
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
};

const MONO_SIZE_CLS: Record<ToolIconSize, string> = {
  xs: "h-3.5 w-3.5 text-[8px] rounded-[5px]",
  sm: "h-4 w-4 text-[9px] rounded-[6px]",
  md: "h-5 w-5 text-[11px] rounded-[7px]",
};

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
    // 通用 Agents 目录无品牌主色，使用应用强调色（Indigo）
    const resolvedColor = colorPrimary ?? (key === "agents" ? "var(--color-accent)" : undefined);
    return <Icon size={px} className={`shrink-0 ${className}`} style={{ color: resolvedColor }} />;
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
