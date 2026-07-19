import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";
import type { MirrorConfig, MirrorHealth } from "@skills-pp/shared";

const MIRROR_CONFIG_KEY = ["mirror-config"] as const;
const MIRROR_HEALTH_KEY = ["mirror-health"] as const;

export function useMirrorConfig() {
  return useQuery<MirrorConfig>({
    queryKey: MIRROR_CONFIG_KEY,
    queryFn: () => ipc.getMirrorConfig(),
    staleTime: Infinity,
  });
}

export function useSetMirrorConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (config: MirrorConfig) => ipc.setMirrorConfig(config),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: MIRROR_CONFIG_KEY });
      // 配置变更时，全局镜像列表会刷新；健康检查结果也会变
      qc.invalidateQueries({ queryKey: MIRROR_HEALTH_KEY });
    },
  });
}

export function useMirrorHealth() {
  return useQuery<MirrorHealth[]>({
    queryKey: MIRROR_HEALTH_KEY,
    queryFn: () => ipc.checkMirrorHealth(),
    staleTime: 1000 * 60 * 5, // 5 分钟
    retry: 1,
  });
}