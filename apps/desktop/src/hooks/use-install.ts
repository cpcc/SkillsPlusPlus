import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";
import type { InstallPreview } from "@skills-pp/shared";

export const INSTALLED_KEY = ["installed-skills"] as const;
export const TASKS_KEY = ["install-tasks"] as const;

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

export function useInstallTasks() {
  return useQuery({
    queryKey: TASKS_KEY,
    queryFn: () => ipc.listInstallTasks(),
  });
}

export function usePreviewInstall() {
  return useMutation({
    mutationFn: ({
      skillName,
      repoUrl,
      directoryId,
    }: {
      skillName: string;
      repoUrl: string;
      directoryId: string;
    }): Promise<InstallPreview> =>
      ipc.previewInstall(skillName, repoUrl, directoryId),
  });
}

export function useInstallSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (params: {
      skillId?: string;
      skillName: string;
      repoUrl: string;
      directoryId: string;
      overwrite: boolean;
    }) => ipc.installSkill(params),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      qc.invalidateQueries({ queryKey: TASKS_KEY });
    },
  });
}

export function useUninstallSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      skillName,
      directoryId,
    }: {
      skillName: string;
      directoryId: string;
    }) => ipc.uninstallSkill(skillName, directoryId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      qc.invalidateQueries({ queryKey: TASKS_KEY });
    },
  });
}

export function useReinstallSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (params: {
      skillId?: string;
      skillName: string;
      repoUrl: string;
      directoryId: string;
    }) => ipc.reinstallSkill(params),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: INSTALLED_KEY });
      qc.invalidateQueries({ queryKey: TASKS_KEY });
    },
  });
}
