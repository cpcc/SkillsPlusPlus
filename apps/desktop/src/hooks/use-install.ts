import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";
import { useToast } from "../components/ui/toast";
import type { InstallPreview, InstallStrategy } from "@skills-pp/shared";

export const INSTALLED_KEY = ["installed-skills"] as const;

export function useInstalledSkills() {
  return useQuery({
    queryKey: INSTALLED_KEY,
    queryFn: () => ipc.listInstalledSkills(),
  });
}

export function useRefreshInstalledSkills() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.refreshInstalledSkills(),
    onSuccess: (data) => {
      qc.setQueryData(INSTALLED_KEY, data);
    },
  });
}

export function useCheckSkillUpdate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (skillId: string) => ipc.checkSkillUpdate(skillId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
    },
  });
}

export function useImportExistingSkills() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.importExistingSkills(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
    },
  });
}


export function usePreviewInstall() {
  return useMutation({
    mutationFn: ({
      skillName,
      repoUrl,
      directoryId,
      strategy,
    }: {
      skillName: string;
      repoUrl: string;
      directoryId: string;
      strategy: InstallStrategy;
    }): Promise<InstallPreview> =>
      ipc.previewInstall(skillName, repoUrl, directoryId, strategy),
  });
}

export function useInstallSkill() {
  const qc = useQueryClient();
  const toast = useToast();
  return useMutation({
    mutationFn: (params: {
      skillId?: string;
      skillName: string;
      repoUrl: string;
      directoryId: string;
      overwrite: boolean;
      strategy?: InstallStrategy;
      archiveUrl?: string;
    }) => ipc.installSkill(params),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      toast(`「${vars.skillName}」安装成功`, undefined, "default");
    },
    onError: (error: Error, vars) => {
      toast(`「${vars.skillName}」安装失败`, error.message, "error");
    },
  });
}

export function useUninstallSkill() {
  const qc = useQueryClient();
  const toast = useToast();
  return useMutation({
    mutationFn: ({
      skillName,
      directoryId,
    }: {
      skillName: string;
      directoryId: string;
    }) => ipc.uninstallSkill(skillName, directoryId),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      toast(`「${vars.skillName}」已卸载`, undefined, "default");
    },
    onError: (error: Error, vars) => {
      toast(`「${vars.skillName}」卸载失败`, error.message, "error");
    },
  });
}

export function useReinstallSkill() {
  const qc = useQueryClient();
  const toast = useToast();
  return useMutation({
    mutationFn: (params: {
      skillId?: string;
      skillName: string;
      repoUrl: string;
      directoryId: string;
      strategy?: InstallStrategy;
      archiveUrl?: string;
    }) => ipc.reinstallSkill(params),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      toast(`「${vars.skillName}」重装成功`, undefined, "default");
    },
    onError: (error: Error, vars) => {
      toast(`「${vars.skillName}」重装失败`, error.message, "error");
    },
  });
}
