import { useQuery } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";
import type { UpdateInfo } from "@skills-pp/shared";

/**
 * 检查应用更新（GitHub Releases latest）。
 *
 * - staleTime 1h：避免短时间内重复打 GitHub API（未认证限速 60 次/小时/IP）
 * - 失败/无更新时静默，由调用方根据 `hasUpdate` 决定 UI
 */
export function useUpdateCheck() {
  return useQuery<UpdateInfo>({
    queryKey: ["app-update"],
    queryFn: () => ipc.checkAppUpdate(),
    staleTime: 1000 * 60 * 60, // 1 hour
    retry: 1,
    refetchOnWindowFocus: false,
  });
}
