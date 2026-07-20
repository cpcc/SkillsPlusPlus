import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";
import type { ImportResult, SyncConfig, SyncResult } from "@skills-pp/shared";

// ─── Phase 1: 本地导出/导入 ──────────────────────────────────────────────────

/**
 * 导出同步快照。
 * 成功后触发浏览器下载（Blob + <a download>），生成 `.skillspp-sync.json` 文件。
 */
export function useExportSnapshot() {
  return useMutation({
    mutationFn: async () => {
      const json = await ipc.exportSyncSnapshot();
      // 用 Blob 触发下载
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      const ts = new Date().toISOString().slice(0, 10);
      a.download = `skillspp-sync-${ts}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      return json;
    },
  });
}

/**
 * 导入同步快照。
 * 用户通过 <input type="file"> 选择 JSON 文件，内容传给后端合并。
 */
export function useImportSnapshot() {
  const qc = useQueryClient();

  return useMutation<ImportResult, Error, string>({
    mutationFn: (json: string) => ipc.importSyncSnapshot(json),
    onSuccess: () => {
      // 导入后刷新所有相关查询
      qc.invalidateQueries({ queryKey: ["installed-skills"] });
      qc.invalidateQueries({ queryKey: ["directories"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
      qc.invalidateQueries({ queryKey: ["mirror-config"] });
      qc.invalidateQueries({ queryKey: ["mirror-health"] });
    },
  });
}

// ─── Phase 2: WebDAV 云同步 ──────────────────────────────────────────────────

/** 默认同步配置 */
export const DEFAULT_SYNC_CONFIG: SyncConfig = {
  webdavUrl: "",
  webdavUsername: "",
  webdavPassword: "",
  webdavRemotePath: "/skillspp",
  autoSync: false,
  autoSyncInterval: 30,
};

/** 获取同步配置 */
export function useSyncConfig() {
  return useQuery({
    queryKey: ["sync-config"],
    queryFn: () => ipc.getSyncConfig(),
    staleTime: 0,
  });
}

/** 保存同步配置 */
export function useSetSyncConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (config: SyncConfig) => ipc.setSyncConfig(config),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["sync-config"] });
    },
  });
}

/** 获取同步状态 */
export function useSyncStatus() {
  return useQuery({
    queryKey: ["sync-status"],
    queryFn: () => ipc.getSyncStatus(),
    staleTime: 0,
  });
}

/** 立即同步 */
export function useSyncNow() {
  const qc = useQueryClient();
  return useMutation<SyncResult, Error, void>({
    mutationFn: () => ipc.syncNow(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["sync-status"] });
      qc.invalidateQueries({ queryKey: ["installed-skills"] });
      qc.invalidateQueries({ queryKey: ["directories"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
      qc.invalidateQueries({ queryKey: ["mirror-config"] });
    },
  });
}

/** 测试 WebDAV 连接 */
export function useTestWebdavConnection() {
  return useMutation({
    mutationFn: (config: SyncConfig) => ipc.testWebdavConnection(config),
  });
}
