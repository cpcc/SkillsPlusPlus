import type { AiToolDirectory } from "@skills-pp/shared";
import {
  CheckCircle,
  XCircle,
  AlertCircle,
  Star,
  MoreHorizontal,
} from "lucide-react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { ToolIcon } from "../../components/ui/ToolIcon";

interface Props {
  dir: AiToolDirectory;
  onToggle: (id: string, enabled: boolean) => void;
  onSetDefault: (id: string) => void;
  onDelete: (id: string) => void;
  onOpenFolder: (path: string) => void;
}

function StatusBadge({ dir }: { dir: AiToolDirectory }) {
  if (!dir.isDetected) {
    return (
      <span className="flex items-center gap-1 text-[11px] text-[var(--color-text-tertiary)]">
        <AlertCircle className="h-3 w-3" />
        未找到
      </span>
    );
  }
  if (!dir.writable) {
    return (
      <span className="flex items-center gap-1 text-[11px] text-[var(--color-warning)]">
        <XCircle className="h-3 w-3" />
        只读
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-[11px] text-[var(--color-success)]">
      <CheckCircle className="h-3 w-3" />
      可用
    </span>
  );
}

export function DirectoryCard({
  dir,
  onToggle,
  onSetDefault,
  onDelete,
  onOpenFolder,
}: Props) {
  return (
    <div
      className={`rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-4 py-3 transition-all ${
        !dir.enabled ? "opacity-40" : "hover:border-[var(--color-border-default)]"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-3">
          <ToolIcon toolName={dir.toolName} size="sm" className="mt-0.5" />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-[13px] font-medium text-[var(--color-text-primary)]">
                {dir.toolName}
              </span>
              {dir.isDefault && (
                <span className="flex items-center gap-1 rounded-full bg-[var(--color-accent-subtle)] px-2 py-[1px] text-[11px] text-[var(--color-accent)]">
                  <Star className="h-2.5 w-2.5" fill="currentColor" />
                  默认
                </span>
              )}
            </div>
            <p
              className="mt-0.5 truncate font-mono text-[11px] text-[var(--color-text-tertiary)]"
              title={dir.path}
            >
              {dir.path}
            </p>
            <div className="mt-1.5 flex items-center gap-3">
              <StatusBadge dir={dir} />
              {dir.isDetected && (
                <span className="text-[11px] text-[var(--color-text-tertiary)]">
                  {dir.skillCount ?? 0} 个 skill
                </span>
              )}
            </div>
          </div>
        </div>

        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <button className="rounded-[var(--radius-sm)] p-1 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]">
              <MoreHorizontal className="h-4 w-4" />
            </button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content
              className="z-50 min-w-[160px] rounded-[var(--radius-lg)] border border-[var(--color-border-default)] bg-[var(--color-surface-overlay)] py-1 shadow-xl shadow-black/20"
              sideOffset={4}
            >
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-[6px] text-[13px] text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
                onSelect={() => onOpenFolder(dir.path)}
              >
                打开目录
              </DropdownMenu.Item>
              {!dir.isDefault && dir.isDetected && (
                <DropdownMenu.Item
                  className="flex cursor-pointer items-center gap-2 px-3 py-[6px] text-[13px] text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
                  onSelect={() => onSetDefault(dir.id)}
                >
                  设为默认
                </DropdownMenu.Item>
              )}
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-[6px] text-[13px] text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
                onSelect={() => onToggle(dir.id, !dir.enabled)}
              >
                {dir.enabled ? "禁用" : "启用"}
              </DropdownMenu.Item>
              <DropdownMenu.Separator className="my-1 h-px bg-[var(--color-border-subtle)]" />
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-[6px] text-[13px] text-[var(--color-danger)] transition-colors hover:bg-[var(--color-danger-subtle)]"
                onSelect={() => onDelete(dir.id)}
              >
                删除
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      </div>
    </div>
  );
}
