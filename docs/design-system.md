# skills++ 设计系统

## 1. 设计方向

**Dark Developer Tool** — 深色主题桌面工具应用，灵感来自 Raycast、Linear、Warp 等现代开发者工具，强调信息密度与操作效率。

### 设计三要素

| 维度 | 值  | 描述 |
|------|-----|------|
| 设计变化度 | 6/10 | 适度不对称，避免过度对称的板块感 |
| 动效强度 | 3/10 | 仅必要的微交互（hover/active/focus），无动画炫技 |
| 视觉密度 | 5/10 | 中密度信息布局，列表为主、网格为辅 |

---

## 2. 色彩系统

### 2.1 核心调色板

| Token | 色值 | 用途 |
|-------|------|------|
| `--color-accent` | `#818cf8` | 主强调色（Indigo/Violet） |
| `--color-accent-hover` | `#a5b4fc` | 强调色悬停态 |
| `--color-accent-muted` | `#6366f1` | 强调色按钮背景 |
| `--color-accent-subtle` | `rgba(99,102,241,0.12)` | 微强调背景（选中态、标签） |
| `--color-accent-text` | `#c7d2fe` | 强调色文字 |

### 2.2 表面层级（由深到浅）

```
surface-hover    #22222c  ← 悬停高亮
surface-overlay  #1c1c24  ← 对话框/下拉菜单
surface-raised   #16161c  ← 卡片/输入框
surface-sidebar  #101015  ← 侧边栏
surface-base     #0c0c10  ← 主背景（最深）
```

### 2.3 边框层级

| Token | 色值 | 用途 |
|-------|------|------|
| `--color-border-subtle` | `#1e1e28` | 卡片分组边框 |
| `--color-border-default` | `#282832` | 输入框/按钮边框 |
| `--color-border-strong` | `#383848` | 悬停高亮边框 |

### 2.4 文字层级

| Token | 色值 | 用途 |
|-------|------|------|
| `--color-text-primary` | `#e4e4e9` | 标题、正文 |
| `--color-text-secondary` | `#8e8e9a` | 描述、次要信息 |
| `--color-text-tertiary` | `#5c5c6a` | 占位符、元数据 |
| `--color-text-inverse` | `#0c0c10` | 浅色按钮上的文字 |

### 2.5 语义色

| Token | 色值 | 含义 |
|-------|------|------|
| `--color-success` | `#34d399` | 成功/正常 |
| `--color-warning` | `#fbbf24` | 警告/待变更 |
| `--color-danger` | `#f87171` | 危险/错误/缺失 |
| `--color-info` | `#60a5fa` | 信息/需更新 |

每个语义色配有对应的 `-subtle` 变体（12% 透明度），用于背景徽章。

---

## 3. 字体系统

### 3.1 字体家族

| Token | 字体 |
|-------|------|
| `--font-sans` | Geist Sans, system-ui, -apple-system, sans-serif |
| `--font-mono` | Geist Mono, ui-monospace, SF Mono, Menlo, monospace |

### 3.2 字号规范

| 场景 | 字号 | 字重 |
|------|------|------|
| 页面标题 | `20px` | 600 Semibold |
| 区块标题 | `13px` | 600 Semibold |
| 主体文案 | `13px` | 400–500 |
| 描述/标签 | `12px` | 400 |
| 元数据/提示 | `11px` | 400 |
| 小字标签 | `11px` | 400 |

### 3.3 字距

- 标题级：`tracking-tight`
- 区块标题（大写）：`tracking-[0.08em]`

---

## 4. 圆角规范

| Token | 值 | 用途 |
|-------|------|------|
| `--radius-sm` | `6px` | 小按钮、图标按钮 |
| `--radius-md` | `8px` | 输入框、下拉框、标准按钮 |
| `--radius-lg` | `12px` | 卡片 |
| `--radius-xl` | `16px` | 对话框 |

---

## 5. 间距系统

- 页面最大内容宽度：`max-w-[960px]`（列表页）/ `max-w-[680px]`（详情/设置页）
- 主内容区 padding：`px-8 py-6`
- 卡片内边距：`p-4`
- 元素间距（gap）：`2-4px`（紧凑）/ `8-12px`（分组）/ `20-32px`（区块分隔）

---

## 6. 布局架构

```
┌─────────────┬──────────────────────────────────┐
│             │                                  │
│  SideNav    │        Main Content              │
│  200px      │        flex-1                    │
│             │        px-8 py-6                 │
│  surface-   │        overflow-auto             │
│  sidebar    │                                  │
│             │                                  │
└─────────────┴──────────────────────────────────┘
```

---

## 7. 组件样式规范

### 7.1 按钮层级

| 类型 | 样式 |
|------|------|
| **Primary** | `bg-accent-muted text-white hover:bg-accent` |
| **Secondary** | `border border-border-default bg-surface-raised text-text-secondary hover:bg-surface-hover hover:text-text-primary` |
| **Icon** | `text-text-tertiary hover:bg-surface-hover hover:text-text-secondary` |
| **Danger** | `text-danger hover:bg-danger-subtle` |

### 7.2 输入框

- 背景：`bg-surface-raised`
- 边框：`border border-border-default`
- 聚焦：`focus:border-accent`
- 占位符：`placeholder:text-text-tertiary`

### 7.3 卡片（Skill / Directory）

- 背景：`bg-surface-raised`
- 边框：`border border-border-subtle`
- 悬停：`hover:border-border-strong`
- 点击按压缩放：`active:scale-[0.99]`

### 7.4 对话框

- 遮罩：`bg-black/50 backdrop-blur-sm`
- 面板：`bg-surface-overlay border border-border-default`
- 阴影：`shadow-2xl shadow-black/30`

---

## 8. 交互状态

### 8.1 状态覆盖

所有交互组件必须覆盖以下状态：
- **默认** — 可操作提示（悬停光标）
- **悬停** — 颜色/边框变化 + 可选平移
- **按下** — `scale-[0.98]` 微缩放
- **聚焦** — `outline: 2px solid accent` + `offset: 2px`
- **禁用** — `opacity-40` + `pointer-events-none`
- **加载中** — 骨架屏（animate-pulse）+ 旋转图标

### 8.2 骨架屏

用于加载状态，匹配最终内容布局形状：
```css
animate-pulse rounded-[--radius-lg] border border-border-subtle bg-surface-raised
/* 内部占位块 */
rounded bg-border-subtle
```

### 8.3 空状态

格式：图标容器（`h-12 w-12`） + 描述文案 + 引导操作
```css
flex flex-col items-center gap-3 text-center
```

---

## 9. 设计避坑清单

> 以下是被主动避免的 AI 生成设计通病

| 避坑项 | 规则 |
|--------|------|
| 紫色渐变 | 不使用任何渐变作为默认装饰 |
| 等大三列卡片 | 使用网格但变化内容密度 |
| 玻璃态效果 | 不使用 backdrop-blur 装饰卡片 |
| 纯白色背景 | 始终使用表面层级系统 |
| 无差别阴影 | 仅对话框使用阴影，卡片用边框区分 |
| overscroll 效果 | 不添加滚动视差或大型动画 |

---

## 10. 文件索引

| 文件 | 职责 |
|------|------|
| `src/index.css` | 设计令牌定义（@theme + 全局样式） |
| `src/App.css` | 页面级过渡动画 |
| `src/components/layout/AppShell.tsx` | 主布局容器 |
| `src/components/layout/SideNav.tsx` | 侧边导航 |
| `src/components/ui/toast.tsx` | 吐司通知 |
| `src/components/ErrorBoundary.tsx` | 错误边界 |
| `src/components/install/InstallDialog.tsx` | 安装确认对话框 |
| `src/components/install/InstallLogPanel.tsx` | 安装日志面板 |
| `src/routes/discover/` | 发现页 + 卡片 + 筛选栏 |
| `src/routes/skill/` | Skill 详情页 |
| `src/routes/installed/` | 已安装管理页 |
| `src/routes/tools/` | 工具与目录页 |
| `src/routes/settings/` | 设置页 |


## 相关skill
Skill (folder)	Install name	Description
taste-skill	design-taste-frontend	🆕 v2 (experimental) - substantial rewrite of the default skill. Reads the brief, infers the design language, tunes three dials (VARIANCE / MOTION / DENSITY). Brief inference, design-system map, hard em-dash ban, canonical GSAP code skeletons, redesign-audit protocol, strict pre-flight check. Actively iterating toward v2.0.0 stable.
taste-skill-v1	design-taste-frontend-v1	The original v1 of taste-skill, preserved for projects depending on its exact behavior. Use only if the v2 default breaks something specific in your workflow.
gpt-tasteskill	gpt-taste	Stricter variant for GPT/Codex: higher layout variance, stronger GSAP direction, aggressive anti-slop.
image-to-code-skill	image-to-code	Image-first pipeline: generate site references, analyze them, then implement the frontend to match.
redesign-skill	redesign-existing-projects	Existing projects: audit the UI first, then fix layout, spacing, hierarchy, styling.
soft-skill	high-end-visual-design	Polished, calm, expensive UI with softer contrast, whitespace, premium fonts, spring motion.
output-skill	full-output-enforcement	When the model ships half-finished work: full output, no placeholder comments.
minimalist-skill	minimalist-ui	Editorial product UI (Notion/Linear vibes), restrained palette, crisp structure.
brutalist-skill	industrial-brutalist-ui	Hard mechanical language: Swiss type, sharp contrast, experimental layout.
stitch-skill	stitch-design-taste	Google Stitch-compatible rules, including optional DESIGN.md export format.