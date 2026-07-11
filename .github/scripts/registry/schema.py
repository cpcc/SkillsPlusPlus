"""SkillItem pydantic 模型——与 packages/shared/src/types.ts + Rust models 对齐。

聚合 JSON 顶层结构：
{
  "version": 1,
  "generatedAt": "ISO-8601",
  "stats": { "total": N, "bySource": {...}, "byCategory": {...} },
  "skills": [SkillItem, ...]
}
"""
from __future__ import annotations

from typing import Optional
from pydantic import BaseModel, Field


INSTALL_STRATEGIES = {"git", "copy", "archive", "skills_cli"}

# 17 类，与 apps/desktop/src/routes/discover/FilterBar.tsx 的 CATEGORIES 完全一致。
CATEGORIES = [
    "自媒体", "金融", "法律", "互联网", "科研", "教育",
    "健康医疗", "通用工具", "办公效率", "内容创作", "开发编程",
    "数据分析", "知识管理", "商业运营", "IT 运维与安全", "生活服务", "其它",
]


class SkillItem(BaseModel):
    id: str
    name: str
    author: Optional[str] = None
    description: Optional[str] = None
    tags: list[str] = Field(default_factory=list)
    category: Optional[str] = None
    sourceId: str
    repoUrl: Optional[str] = None
    detailUrl: str
    updatedAt: Optional[str] = None
    compatibleTools: list[str] = Field(default_factory=list)
    stars: Optional[int] = None
    installStrategy: Optional[str] = None
    archiveUrl: Optional[str] = None


class Stats(BaseModel):
    total: int
    bySource: dict[str, int] = Field(default_factory=dict)
    byCategory: dict[str, int] = Field(default_factory=dict)


class RegistryPayload(BaseModel):
    version: int = 1
    generatedAt: str
    stats: Stats
    skills: list[SkillItem]
