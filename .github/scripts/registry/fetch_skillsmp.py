"""fetch_skillsmp: 首选源。

skillsmp.com 的公开 REST API（先探测 /docs/api，若不存在降级到根路径抓首页元数据）。
文档：https://skillsmp.com/docs/api
"""
from __future__ import annotations

from .schema import SkillItem
from ._common import http_client, github_archive_url, extract_owner_repo

# 候选 API 端点（文档未明确，依次尝试）
CANDIDATE_ENDPOINTS = [
    "https://skillsmp.com/api/skills",
    "https://skillsmp.com/api/v1/skills",
    "https://skillsmp.com/api/skills?per_page=200",
]


def fetch() -> list[SkillItem]:
    items: list[SkillItem] = []
    with http_client(timeout=20.0) as client:
        payload = None
        for url in CANDIDATE_ENDPOINTS:
            try:
                r = client.get(url, headers={"Accept": "application/json"})
                if r.status_code >= 400:
                    continue
                ct = r.headers.get("content-type", "")
                if "json" not in ct:
                    continue
                payload = r.json()
                print(f"[skillsmp] using endpoint: {url}")
                break
            except Exception as e:
                print(f"[skillsmp] {url} failed: {e}")
                continue

        if not payload:
            print("[skillsmp] no reachable API endpoint; skipping")
            return []

        # 兼容 {skills: [...]} / [...] / {data: [...]} 三种 shape
        if isinstance(payload, list):
            rows = payload
        elif isinstance(payload, dict):
            rows = payload.get("skills") or payload.get("data") or payload.get("items") or []
        else:
            rows = []
            print(f"[skillsmp] unexpected payload type: {type(payload)}")

        for row in rows:
            if not isinstance(row, dict):
                continue
            name = row.get("name") or row.get("slug") or row.get("id")
            if not name:
                continue
            slug = row.get("slug") or str(row.get("id") or name)
            repo_url = row.get("repoUrl") or row.get("repo_url") or row.get("githubUrl")
            owner_repo = extract_owner_repo(repo_url) if repo_url else None
            archive = github_archive_url(*owner_repo) if owner_repo else None
            category = row.get("category") or row.get("categories") or None
            if isinstance(category, list):
                category = category[0] if category else None
            items.append(SkillItem(
                id=f"registry_skillsmp_{slug}",
                name=name,
                author=row.get("author") or (owner_repo[0] if owner_repo else None),
                description=row.get("description") or row.get("desc"),
                tags=row.get("tags", []) or [],
                category=category,
                sourceId="skillsmp",
                repoUrl=repo_url,
                detailUrl=row.get("detailUrl") or f"https://skillsmp.com/skills/{slug}",
                updatedAt=row.get("updatedAt") or row.get("updated_at"),
                compatibleTools=row.get("compatibleTools", []) or [],
                stars=row.get("stars") or row.get("installs"),
                installStrategy=row.get("installStrategy") or ("archive" if archive else None),
                archiveUrl=archive,
            ))
    print(f"[skillsmp] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    print(len(fetch()))
