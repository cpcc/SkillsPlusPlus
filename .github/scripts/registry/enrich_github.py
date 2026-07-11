"""enrich_github: 对每条 skill 调 GitHub repo API 补 description/topics/language/stars/homepage。

带磁盘 ETag 缓存（`tmp/gh_meta_cache.json`），避免对同一 repo 重复请求。
只对 repoUrl 是 github.com 的 skill 生效。
"""
from __future__ import annotations

import json
import os
import time
from pathlib import Path
from typing import Optional

from .schema import SkillItem
from ._common import http_client, github_token, extract_owner_repo

CACHE_PATH = Path("tmp/gh_meta_cache.json")


def _load_cache() -> dict[str, dict]:
    if CACHE_PATH.exists():
        try:
            return json.loads(CACHE_PATH.read_text("utf-8"))
        except Exception:
            return {}
    return {}


def _save_cache(cache: dict[str, dict]) -> None:
    CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
    CACHE_PATH.write_text(json.dumps(cache, ensure_ascii=False), "utf-8")


def _fetch_repo_meta(client, owner: str, repo: str, etag: Optional[str], token: Optional[str]):
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if etag:
        headers["If-None-Match"] = etag
    r = client.get(f"https://api.github.com/repos/{owner}/{repo}", headers=headers)
    if r.status_code == 304:
        return None, etag  # 未修改
    if r.status_code >= 400:
        raise RuntimeError(f"{r.status_code} {r.text[:200]}")
    new_etag = r.headers.get("ETag") or etag
    return r.json(), new_etag


def enrich(items: list[SkillItem]) -> list[SkillItem]:
    token = github_token()
    cache = _load_cache()

    with http_client(timeout=15.0) as client:
        for s in items:
            if not s.repoUrl:
                continue
            owner_repo = extract_owner_repo(s.repoUrl)
            if not owner_repo:
                continue
            key = "/".join(owner_repo)
            cached = cache.get(key)
            etag = cached.get("__etag") if cached else None

            try:
                meta, new_etag = _fetch_repo_meta(client, owner_repo[0], owner_repo[1], etag, token)
            except Exception as e:
                print(f"[enrich] {key} failed: {e}")
                time.sleep(2)
                continue

            if meta is None and cached:
                meta = cached

            if not isinstance(meta, dict):
                continue

            # 写回缓存
            cache[key] = {**meta, "__etag": new_etag}
            if len(cache) % 25 == 0:
                _save_cache(cache)

            # 填字段：不覆盖已存在的非空值（源原生数据优先）
            if not s.description and meta.get("description"):
                s.description = meta["description"]
            if not s.author and meta.get("owner", {}).get("login"):
                s.author = meta["owner"]["login"]
            topics = meta.get("topics") or []
            if topics:
                merged = list({*s.tags, *topics})
                s.tags = merged[:10]
            if s.stars is None and meta.get("stargazers_count") is not None:
                s.stars = meta["stargazers_count"]
            if not s.updatedAt and meta.get("updated_at"):
                s.updatedAt = meta["updated_at"]
            if not s.archiveUrl:
                s.archiveUrl = (f"https://codeload.github.com/{key}/tar.gz/refs/heads/main")

            time.sleep(0.2)  # GitHub secondary rate limit ~900 req/min

    _save_cache(cache)
    print(f"[enrich] enriched {len(items)} skills (cache size={len(cache)})")
    return items


if __name__ == "__main__":
    demo = [SkillItem(id="x", name="test", sourceId="skills_sh",
                      detailUrl="", repoUrl="https://github.com/octocat/Hello-World")]
    for s in enrich(demo):
        print(s.model_dump())
