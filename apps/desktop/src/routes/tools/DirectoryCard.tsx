import type { AiToolDirectory } from "@skills-pp/shared";
import {
  Folder,
  CheckCircle,
  XCircle,
  AlertCircle,
  Star,
  MoreHorizontal,
} from "lucide-react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";

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
      <span className="flex items-center gap-1 text-xs text-gray-400">
        <AlertCircle className="h-3 w-3" />
        未找到
      </span>
    );
  }
  if (!dir.writable) {
    return (
      <span className="flex items-center gap-1 text-xs text-yellow-600">
        <XCircle className="h-3 w-3" />
        只读
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-xs text-green-600">
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
      className={`rounded-lg border bg-white p-4 transition-opacity ${
        !dir.enabled ? "opacity-50" : ""
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-3">
          <Folder className="mt-0.5 h-4 w-4 shrink-0 text-brand-500" />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-900">
                {dir.toolName}
              </span>
              {dir.isDefault && (
                <span className="flex items-center gap-1 rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700">
                  <Star className="h-3 w-3" />
                  默认
                </span>
              )}
            </div>
            <p
              className="mt-0.5 truncate font-mono text-xs text-gray-400"
              title={dir.path}
            >
              {dir.path}
            </p>
            <div className="mt-1 flex items-center gap-3">
              <StatusBadge dir={dir} />
              {dir.isDetected && (
                <span className="text-xs text-gray-400">
                  {dir.skillCount ?? 0} 个 skill
                </span>
              )}
            </div>
          </div>
        </div>

        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <button className="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600">
              <MoreHorizontal className="h-4 w-4" />
            </button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content
              className="z-50 min-w-40 rounded-lg border border-gray-200 bg-white py-1 shadow-lg"
              sideOffset={4}
            >
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                onSelect={() => onOpenFolder(dir.path)}
              >
                打开目录
              </DropdownMenu.Item>
              {!dir.isDefault && dir.isDetected && (
                <DropdownMenu.Item
                  className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                  onSelect={() => onSetDefault(dir.id)}
                >
                  设为默认
                </DropdownMenu.Item>
              )}
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                onSelect={() => onToggle(dir.id, !dir.enabled)}
              >
                {dir.enabled ? "禁用" : "启用"}
              </DropdownMenu.Item>
              <DropdownMenu.Separator className="my-1 h-px bg-gray-100" />
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-red-600 hover:bg-red-50"
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
