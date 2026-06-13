import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { RefreshCw, PlusCircle, FolderTree, Info, Database, Monitor } from "lucide-react";
import {
  useDirectories,
  useScanDirectories,
  useAddDirectory,
  useToggleDirectory,
  useSetDefaultDirectory,
  useDeleteDirectory,
} from "../../hooks/use-directories";
import type { AiToolDirectory } from "@skills-pp/shared";
import { useAppInfo } from "../../hooks/use-app-info";
import { useToast } from "../../components/ui/toast";
import { DirectoryCard } from "./DirectoryCard";
import { AddDirectoryDialog } from "./AddDirectoryDialog";

function groupByTool(dirs: AiToolDirectory[]): Map<string, AiToolDirectory[]> {
  const map = new Map<string, AiToolDirectory[]>();
  for (const d of dirs) {
    const list = map.get(d.toolName) ?? [];
    list.push(d);
    map.set(d.toolName, list);
  }
  return map;
}

export default function SettingsPage() {
  const { data: appData, isLoading: appLoading, error: appError } = useAppInfo();

  const { data: dirs = [], isLoading: dirsLoading } = useDirectories();
  const scan = useScanDirectories();
  const add = useAddDirectory();
  const toggle = useToggleDirectory();
  const setDefault = useSetDefaultDirectory();
  const del = useDeleteDirectory();
  const toast = useToast();
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    scan.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleOpenFolder(path: string) {
    openPath(path).catch(() => toast("无法打开目录", path, "error"));
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
    <div className="mx-auto max-w-[760px]">
      <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
        设置
      </h1>
      <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
        目录管理、来源站配置与日志
      </p>

      {/* Directories */}
      <div className="mt-8">
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
            目录管理
            <span className="ml-2 font-normal normal-case tracking-normal text-[var(--color-text-tertiary)]">
              {dirsLoading || scan.isPending
                ? "扫描中..."
                : `共 ${dirs.length} 个，已找到 ${detectedCount} 个`}
            </span>
          </h3>
          <div className="flex items-center gap-2">
            <button
              onClick={() => scan.mutate()}
              disabled={scan.isPending}
              className="flex items-center gap-1.5 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-2.5 py-[5px] text-[12px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] disabled:opacity-40"
            >
              <RefreshCw
                className={`h-3 w-3 ${scan.isPending ? "animate-spin" : ""}`}
              />
              重新扫描
            </button>
            <button
              onClick={() => setDialogOpen(true)}
              className="flex items-center gap-1.5 rounded-[var(--radius-md)] bg-[var(--color-accent-muted)] px-2.5 py-[5px] text-[12px] font-medium text-white transition-colors hover:bg-[var(--color-accent)] active:scale-[0.98]"
            >
              <PlusCircle className="h-3 w-3" />
              新增目录
            </button>
          </div>
        </div>

        {dirsLoading && dirs.length === 0 ? (
          <div className="flex justify-center py-8">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-[var(--color-border-default)] border-t-[var(--color-accent)]" />
          </div>
        ) : dirs.length === 0 ? (
          <div className="flex flex-col items-center gap-3 rounded-[var(--radius-lg)] border border-dashed border-[var(--color-border-default)] py-10 text-center">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)]">
              <FolderTree className="h-4 w-4 text-[var(--color-text-tertiary)]" />
            </div>
            <p className="text-[12px] text-[var(--color-text-tertiary)]">
              未找到任何 AI 工具目录，点击「新增目录」手动添加
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {Array.from(grouped.entries()).map(([toolName, toolDirs]) => (
              <div key={toolName}>
                <h4 className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
                  {toolName}
                </h4>
                <div className="space-y-1.5">
                  {toolDirs.map((d) => (
                    <DirectoryCard
                      key={d.id}
                      dir={d}
                      onToggle={handleToggle}
                      onSetDefault={handleSetDefault}
                      onDelete={handleDelete}
                      onOpenFolder={handleOpenFolder}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* App Info */}
      <div className="mt-10">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          应用信息
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          {appLoading && (
            <div className="px-5 py-4">
              <div className="animate-pulse space-y-3">
                <div className="h-4 w-32 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-24 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-48 rounded bg-[var(--color-border-subtle)]" />
              </div>
            </div>
          )}
          {appError && (
            <div className="flex items-center gap-3 px-5 py-4">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-danger-subtle)]">
                <Info className="h-4 w-4 text-[var(--color-danger)]" />
              </div>
              <p className="text-[13px] text-[var(--color-danger)]">
                加载失败：{String(appError)}
              </p>
            </div>
          )}
          {appData && (
            <>
              <InfoRow icon={Info} label="版本">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {appData.version}
                </span>
              </InfoRow>
              <InfoRow icon={Monitor} label="平台">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {appData.platform}
                </span>
              </InfoRow>
              <InfoRow icon={Database} label="数据库">
                <span className="truncate font-mono text-[11px] text-[var(--color-text-tertiary)]">
                  {appData.dbPath}
                </span>
              </InfoRow>
            </>
          )}
        </div>
      </div>

      <AddDirectoryDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={handleAdd}
        isPending={add.isPending}
      />
    </div>
  );
}

function InfoRow({
  icon: Icon,
  label,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-4 px-5 py-3.5">
      <Icon className="h-4 w-4 shrink-0 text-[var(--color-text-tertiary)]" />
      <span className="w-20 shrink-0 text-[12px] font-medium text-[var(--color-text-tertiary)]">
        {label}
      </span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
