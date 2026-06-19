import { useEffect, useState } from "react";
import { RefreshCw, PlusCircle, FolderTree } from "lucide-react";
import type { AiToolDirectory } from "@skills-pp/shared";
import { ipc } from "../../lib/ipc";
import {
  useDirectories,
  useScanDirectories,
  useAddDirectory,
  useToggleDirectory,
  useSetDefaultDirectory,
  useDeleteDirectory,
} from "../../hooks/use-directories";
import { DirectoryCard } from "./DirectoryCard";
import { AddDirectoryDialog } from "./AddDirectoryDialog";
import { DirectoryContentsDrawer } from "./DirectoryContentsDrawer";
import { useToast } from "../../components/ui/toast";

function groupByTool(dirs: AiToolDirectory[]): Map<string, AiToolDirectory[]> {
  const map = new Map<string, AiToolDirectory[]>();
  for (const d of dirs) {
    const list = map.get(d.toolName) ?? [];
    list.push(d);
    map.set(d.toolName, list);
  }
  return map;
}

export default function ToolsPage() {
  const { data: dirs = [], isLoading } = useDirectories();
  const scan = useScanDirectories();
  const add = useAddDirectory();
  const toggle = useToggleDirectory();
  const setDefault = useSetDefaultDirectory();
  const del = useDeleteDirectory();
  const toast = useToast();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [drawerDir, setDrawerDir] = useState<AiToolDirectory | null>(null);

  useEffect(() => {
    scan.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleOpenFolder(path: string) {
    ipc.openSkillDir(path).catch(() => toast("无法打开目录", path, "error"));
  }

  function handleAdd(toolName: string, path: string) {
    add.mutate(
      { toolName, path },
      {
        onSuccess: () => {
          setDialogOpen(false);
          toast("目录已添加", path);
        },
        onError: (e) => toast("添加失败", String(e), "error"),
      },
    );
  }

  function handleToggle(id: string, enabled: boolean) {
    toggle.mutate(
      { id, enabled },
      { onError: (e) => toast("操作失败", String(e), "error") },
    );
  }

  function handleSetDefault(id: string) {
    setDefault.mutate(id, {
      onSuccess: () => toast("已设为默认目录"),
      onError: (e) => toast("操作失败", String(e), "error"),
    });
  }

  function handleDelete(id: string) {
    del.mutate(id, {
      onError: (e) => toast("删除失败", String(e), "error"),
    });
  }

  const grouped = groupByTool(dirs);
  const detectedCount = dirs.filter((d) => d.isDetected).length;

  return (
    <div className="mx-auto max-w-[960px]">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
            工具与目录
          </h1>
          <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
            {isLoading || scan.isPending
              ? "正在扫描..."
              : `共 ${dirs.length} 个目录，已找到 ${detectedCount} 个`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => scan.mutate()}
            disabled={scan.isPending}
            className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-[6px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] disabled:opacity-40"
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${scan.isPending ? "animate-spin" : ""}`}
            />
            重新扫描
          </button>
          <button
            onClick={() => setDialogOpen(true)}
            className="flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--color-accent-muted)] px-3 py-[6px] text-[13px] font-medium text-white transition-colors hover:bg-[var(--color-accent)] active:scale-[0.98]"
          >
            <PlusCircle className="h-3.5 w-3.5" />
            新增目录
          </button>
        </div>
      </div>

      {/* Content */}
      {isLoading && dirs.length === 0 ? (
        <div className="mt-12 flex flex-col items-center gap-3">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-[var(--color-border-default)] border-t-[var(--color-accent)]" />
          <p className="text-[12px] text-[var(--color-text-tertiary)]">扫描中...</p>
        </div>
      ) : dirs.length === 0 ? (
        <div className="mt-12 flex flex-col items-center gap-4 rounded-[var(--radius-lg)] border border-dashed border-[var(--color-border-default)] p-16 text-center">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)]">
            <FolderTree className="h-5 w-5 text-[var(--color-text-tertiary)]" />
          </div>
          <div>
            <p className="text-[13px] font-medium text-[var(--color-text-secondary)]">
              未找到任何 AI 工具目录
            </p>
            <p className="mt-1 text-[12px] text-[var(--color-text-tertiary)]">
              点击「新增目录」手动添加，或确认本机已安装 AI 工具后重新扫描
            </p>
          </div>
        </div>
      ) : (
        <div className="mt-6 space-y-6">
          {Array.from(grouped.entries()).map(([toolName, toolDirs]) => (
            <div key={toolName}>
              <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
                {toolName}
              </h3>
              <div className="space-y-1.5">
                {toolDirs.map((d) => (
                  <DirectoryCard
                    key={d.id}
                    dir={d}
                    onToggle={handleToggle}
                    onSetDefault={handleSetDefault}
                    onDelete={handleDelete}
                    onOpenFolder={handleOpenFolder}
                    onOpenContents={setDrawerDir}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <AddDirectoryDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={handleAdd}
        isPending={add.isPending}
      />

      <DirectoryContentsDrawer
        directory={drawerDir}
        onClose={() => setDrawerDir(null)}
      />
    </div>
  );
}
