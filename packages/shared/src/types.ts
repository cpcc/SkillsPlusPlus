// ===== 来源站 =====
export type SkillSource = {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
};

// ===== Skill 条目（来源聚合后标准化） =====
export type SkillItem = {
  id: string;
  name: string;
  author?: string;
  description?: string;
  tags: string[];
  sourceId: string;
  repoUrl?: string;
  detailUrl: string;
  updatedAt?: string;
  compatibleTools?: string[];
};

// ===== AI 工具目录 =====
export type AiToolDirectory = {
  id: string;
  toolName: string;
  path: string;
  isDefault: boolean;
  isDetected: boolean;
  writable: boolean;
  enabled: boolean;
  skillCount?: number;
};

// ===== 已安装 Skill =====
export type InstalledSkill = {
  id: string;
  skillId?: string;
  name: string;
  toolName: string;
  directoryId: string;
  sourceId?: string;
  repoUrl?: string;
  installedAt: string;
  status: "ok" | "missing" | "changed" | "update_available";
};

// ===== 安装任务 =====
export type InstallTask = {
  id: string;
  skillId?: string;
  skillName: string;
  toolName: string;
  directoryId: string;
  action: "install" | "reinstall" | "uninstall" | "scan";
  status: "pending" | "running" | "success" | "failed" | "cancelled";
  startedAt?: string;
  finishedAt?: string;
  errorMessage?: string;
};

// ===== 工具目录规则（Rust 侧镜像类型） =====
export type ToolRule = {
  toolName: string;
  platform: "macos" | "windows" | "linux" | "all";
  candidatePaths: string[];
  detectionHints?: string[];
  installStrategy: "copy" | "git" | "archive" | "skills_cli";
};

// ===== 应用信息（IPC 响应类型） =====
export type AppInfo = {
  version: string;
  dbPath: string;
  logPath: string;
  platform: string;
};

// ===== 错误码 =====
export const ErrorCode = {
  DIR_NOT_FOUND: "DIR_NOT_FOUND",
  DIR_NOT_WRITABLE: "DIR_NOT_WRITABLE",
  NETWORK_ERROR: "NETWORK_ERROR",
  SOURCE_FETCH_FAILED: "SOURCE_FETCH_FAILED",
  INSTALL_CONFLICT: "INSTALL_CONFLICT",
  INSTALL_FAILED: "INSTALL_FAILED",
  UNINSTALL_FAILED: "UNINSTALL_FAILED",
  DB_ERROR: "DB_ERROR",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

export type AppError = {
  code: ErrorCode;
  message: string;
  detail?: string;
};
