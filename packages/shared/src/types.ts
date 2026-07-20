// ===== 安装策略 =====
export type InstallStrategy = "git" | "copy" | "archive" | "skills_cli";

// ===== 来源站 =====
export type SkillSource = {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
};

export type RefreshWarning = {
  sourceId: string;
  message: string;
};

export type RefreshSourcesResult = {
  skills: SkillItem[];
  warnings: RefreshWarning[];
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
  stars?: number;
  /** adapter 声明的默认安装策略 */
  installStrategy?: InstallStrategy;
  /** 用户切换到 copy/archive/skills_cli 时使用的归档下载地址 */
  archiveUrl?: string;
  /** CI 聚合阶段生成的分类（对齐 FilterBar 17 类），registry 源必有；其它源可能为空 */
  category?: string;
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
  directoryPath: string;
  sourceId?: string;
  repoUrl?: string;
  installedAt: string;
  status: "ok" | "missing" | "changed" | "update_available";
  installStrategy: InstallStrategy;
  contentHash?: string;
  canonicalPath?: string;
  author?: string;
  description?: string;
};

// ===== 工具目录规则（Rust 侧镜像类型） =====
export type ToolRule = {
  toolName: string;
  platform: "macos" | "windows" | "linux" | "all";
  candidatePaths: string[];
  detectionHints?: string[];
  installStrategy: InstallStrategy;
};

// ===== 应用信息（IPC 响应类型） =====
export type AppInfo = {
  version: string;
  dbPath: string;
  logPath: string;
  platform: string;
};

// ===== 应用更新检查（GitHub Releases latest） =====
export type UpdateInfo = {
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  releaseNotes: string;
  publishedAt: string;
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

// ===== 安装预览 =====
export type InstallPreview = {
  skillName: string;
  repoUrl: string;
  targetPath: string;
  strategy: InstallStrategy;
  /** 仅 skills_cli 时有值：canonical 存储绝对路径 */
  canonicalPath?: string;
  /** 仅 skills_cli 时有值：symlink 绝对路径 */
  symlinkPath?: string;
  conflict?: {
    existingPath: string;
    kind: "existing_dir" | "existing_file";
  };
};

// ===== Canonical store（npx skills 互通） =====
/** ~/.agents/.skill-lock.json 中的单个条目，字段与 vercel-labs skills 一致 */
export type LockEntry = {
  source: string;
  sourceType: string;
  sourceUrl: string;
  skillPath: string;
  skillFolderHash: string;
  installedAt: string;
  updatedAt: string;
};

export type SkillLockfile = {
  version: number;
  skills: Record<string, LockEntry>;
};

/** ~/.agents/skills/<name>/ 扫描结果 */
export type CanonicalSkill = {
  name: string;
  path: string;
  description?: string;
  hasSkillMd: boolean;
};

// ===== 目录文件树（抽屉） =====
export type FileNodeKind = "file" | "dir";

export type FileTreeNode = {
  name: string;
  /** 永远用 '/' 分隔，方便前端直接当 key 用。 */
  relativePath: string;
  /** 直接给前端用，不在前端拼路径。 */
  absolutePath: string;
  kind: FileNodeKind;
  size: number;
  /** dir 级：该目录是否含 SKILL.md（任意大小写）。 */
  hasSkillMd: boolean;
  /** dir 级：hasSkillMd 或顶层含 .md/.yaml/.yml 文件。 */
  isSkill: boolean;
  children?: FileTreeNode[] | null;
  /** 因深度/计数限制被截断。 */
  truncated: boolean;
  /** read_dir 失败时填。 */
  error?: string | null;
};

// ===== 镜像配置 =====
export type MirrorConfig = {
  /** 是否启用镜像 fallback */
  enabled: boolean;
  /** GitHub 镜像候选前缀列表（按优先级），空字符串代表直连 */
  githubMirrors: string[];
};

export type MirrorHealth = {
  /** 镜像前缀（空字符串表示直连） */
  prefix: string;
  /** 是否可达（任一测试 URL 成功） */
  reachable: boolean;
  /** 首个成功 URL 的响应时间（毫秒），失败时为 null */
  latencyMs: number | null;
  /** 错误信息（仅 unreachable 时有值） */
  error?: string;
};

// ===== 跨设备同步 =====

/** 同步快照中的一条安装记录 */
export type SyncInstalledSkill = {
  name: string;
  toolName: string;
  /** 相对于 home 的目录路径（/ 分隔符） */
  directoryRelativePath: string;
  sourceId?: string;
  repoUrl?: string;
  installStrategy: InstallStrategy;
  contentHash?: string;
  /** 相对于 home 的 canonical 路径 */
  canonicalRelativePath?: string;
  installedAt: string;
  author?: string;
  description?: string;
};

/** 同步快照中的一条自定义目录 */
export type SyncDirectory = {
  id: string;
  toolName: string;
  /** 相对于 home 的目录路径（/ 分隔符） */
  relativePath: string;
  isDefault: boolean;
};

/** 来源站开关 */
export type SyncSourcePref = {
  id: string;
  enabled: boolean;
};

/** 同步快照（导出/导入的 JSON 根结构） */
export type SyncSnapshot = {
  version: number;
  exportedAt: string;
  deviceName: string;
  platform: string;
  installedSkills: SyncInstalledSkill[];
  customDirectories: SyncDirectory[];
  sourcePreferences: SyncSourcePref[];
  appSettings: Record<string, string>;
  lockfile: Record<string, LockEntry>;
};

/** 导入操作的汇总结果 */
export type ImportResult = {
  importedSkills: number;
  skippedSkills: number;
  importedDirectories: number;
  updatedSources: number;
  updatedSettings: number;
  mergedLockfileEntries: number;
};

// ===== Phase 2: WebDAV 云同步 =====

/** WebDAV 同步配置 */
export type SyncConfig = {
  /** WebDAV 服务器 URL */
  webdavUrl: string;
  /** WebDAV 用户名 */
  webdavUsername: string;
  /** WebDAV 密码 */
  webdavPassword: string;
  /** 远端存储路径 */
  webdavRemotePath: string;
  /** 是否启用自动同步 */
  autoSync: boolean;
  /** 自动同步间隔（分钟） */
  autoSyncInterval: number;
};

/** 同步状态 */
export type SyncStatus = {
  /** 上次同步时间（ISO 8601），null 表示从未同步 */
  lastSyncAt: string | null;
  /** 上次同步的设备名 */
  lastSyncDevice: string | null;
  /** 上次同步结果：success / conflict / error */
  lastSyncResult: string | null;
  /** 上次同步的错误信息 */
  lastSyncError: string | null;
};

/** 同步冲突 */
export type SyncConflict = {
  /** 冲突类型：remote_deleted */
  kind: string;
  /** skill 名称 */
  skillName: string;
  /** 工具名 */
  toolName: string;
  /** 目录相对路径 */
  directoryRelativePath: string;
};

/** sync_now 操作的汇总结果 */
export type SyncResult = {
  /** 从远端拉取的新安装记录数 */
  pulledSkills: number;
  /** 推送到远端的安装记录数 */
  pushedSkills: number;
  /** 更新的设置数 */
  updatedSettings: number;
  /** 合并的 lockfile 条目数 */
  mergedLockfileEntries: number;
  /** 检测到的冲突列表 */
  conflicts: SyncConflict[];
};
