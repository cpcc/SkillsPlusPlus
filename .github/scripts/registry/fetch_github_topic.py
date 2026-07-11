"""fetch_github_topic: 调 GitHub Search API 按 topic 拉仓库。

与 fetch_skills_sh 互补：skills.sh 只覆盖已索引的仓库，GitHub topic 直接命中原始仓库。
"""
from __future__ import annotations

import time

from .schema import SkillItem
from ._common import http_client, github_token, github_repo_url, github_archive_url

TOPICS = [
    "claude-skill", "codex-skill", "copilot-skill", "gemini-skill",
    "opencode-skill", "ai-skill",
]


def _infer_tools(topics: list[str]) -> list[str]:
    t = " ".join(topics).lower()
    tools = []
    if "claude" in t: tools.append("Claude")
    if "codex" in t: tools.append("Codex")
    if "copilot" in t: tools.append("GitHub Copilot")
    if "gemini" in t: tools.append("Gemini CLI")
    if "cursor" in t: tools.append("Cursor")
    if "opencode" in t: tools.append("OpenCode")
    return tools or ["通用"]


def fetch(per_page: int = 50) -> list[SkillItem]:
    token = github_token()
    if not token:
        print("[github_topic] GITHUB_TOKEN missing, skipping")
        return []

    items: list[SkillItem] = []
    seen: set[int] = set()
    with http_client(timeout=20.0) as client:
        for topic in TOPICS:
            try:
                r = client.get(
                    "https://api.github.com/search/repositories",
                    params={
                        "q": f"topic:{topic}",
                        "sort": "stars",
                        "order": "desc",
                        "per_page": per_page,
                    },
                    headers={
                        "Accept": "application/vnd.github+json",
                        "X-GitHub-Api-Version": "2022-11-28",
                        "Authorization": f"Bearer {token}",
                    },
                )
                if r.status_code == 403:
                    print(f"[github_topic] rate-limited on topic={topic}; sleeping 30s")
                    time.sleep(30)
                    continue
                r.raise_for_status()
            except Exception as e:
                print(f"[github_topic] topic={topic} failed: {e}")
                continue

            for repo in r.json().get("items", []):
                rid = repo.get("id")
                if rid is None or rid in seen:
                    continue
                seen.add(rid)
                owner = repo.get("owner", {}).get("login", "")
                name = repo.get("name", "")
                if not owner or not name:
                    continue
                topics = repo.get("topics", []) or []
                items.append(SkillItem(
                    id=f"registry_github_{rid}",
                    name=name,
                    author=owner,
                    description=repo.get("description"),
                    tags=[t for t in topics if not t.endswith("-skill")] or ["skill"],
                    sourceId="skills_sh",  # GitHub 是 skills.sh 的上游，共用 sourceId
                    repoUrl=repo.get("html_url") or github_repo_url(owner, name),
                    detailUrl=repo.get("html_url") or github_repo_url(owner, name),
                    updatedAt=repo.get("updated_at"),
                    compatibleTools=_infer_tools(topics),
                    stars=repo.get("stargazers_count"),
                    installStrategy="git",
                    archiveUrl=github_archive_url(owner, name),
                ))
            time.sleep(0.5)  # 礼貌限速
    print(f"[github_topic] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    for s in fetch(10):
        print(s.id, s.name, s.stars)
