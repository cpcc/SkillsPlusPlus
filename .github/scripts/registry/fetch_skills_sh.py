"""fetch_skills_sh: 调 skills.sh /api/search 抓 skill 列表。

该 API 仅返回 id/skillId/name/installs/source(=owner/repo)，没有 description/author/tags。
后续由 enrich_github.py 补齐，由 classify.py 推断分类。

关键词来自 Rust adapter 的 TOPICS 列表（claude-skill/codex-skill/...）。
"""
from __future__ import annotations

from .schema import SkillItem
from ._common import http_client, github_repo_url, github_archive_url

# 与 Rust adapters/github.rs 的 TOPICS 对齐
KEYWORDS = [
    "claude-skill", "codex-skill", "copilot-skill", "gemini-skill",
    "opencode-skill", "ai-skill",
]


def fetch(limit_per_query: int = 50) -> list[SkillItem]:
    items: list[SkillItem] = []
    seen: set[str] = set()
    with http_client(timeout=20.0) as client:
        for kw in KEYWORDS:
            try:
                r = client.get(
                    "https://skills.sh/api/search",
                    params={"q": kw, "limit": limit_per_query},
                    headers={"Accept": "application/json"},
                )
                r.raise_for_status()
                payload = r.json()
            except Exception as e:
                print(f"[skills_sh] query={kw} failed: {e}")
                continue

            for hit in payload.get("skills", []):
                hit_id = hit.get("id")
                if not hit_id or hit_id in seen:
                    continue
                seen.add(hit_id)

                source = hit.get("source", "")  # owner/repo
                skill_id = hit.get("skillId") or hit.get("skill_id") or hit.get("name", hit_id)
                installs = hit.get("installs") or 0
                owner_repo = source.split("/", 1)
                if len(owner_repo) != 2:
                    continue
                owner, repo = owner_repo

                items.append(SkillItem(
                    id=f"registry_skills_sh_{hit_id}",
                    name=skill_id,
                    sourceId="skills_sh",
                    repoUrl=github_repo_url(owner, repo),
                    detailUrl=f"https://skills.sh/{hit_id}",
                    stars=int(installs),
                    installStrategy="git",
                    archiveUrl=github_archive_url(owner, repo),
                ))
    print(f"[skills_sh] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    for s in fetch(10):
        print(s.id, s.name, s.stars)
