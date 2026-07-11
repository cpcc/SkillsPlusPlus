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
