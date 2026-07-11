"""各 fetcher 的通用工具：HTTP 客户端、GitHub archive URL、来源优先级。"""
from __future__ import annotations

import os
from typing import Optional

import httpx

# 来源优先级（高→低）。merge_and_push 在合并同一 repoUrl+name 时按此取 sourceId。
SOURCE_PRIORITY = ["skillsmp", "skills_sh", "lobehub", "skillhub", "clawhub"]


def source_rank(source_id: str) -> int:
    try:
        return SOURCE_PRIORITY.index(source_id)
    except ValueError:
        return len(SOURCE_PRIORITY)


def http_client(*, timeout: float = 15.0) -> httpx.Client:
    return httpx.Client(
        timeout=timeout,
        headers={"User-Agent": "skills-plus-plus-registry/0.1"},
        follow_redirects=True,
    )


def github_archive_url(owner: str, repo: str) -> str:
    """与 Rust github_archive_url 对齐：main 分支 tar.gz。"""
    return f"https://codeload.github.com/{owner}/{repo}/tar.gz/refs/heads/main"


def github_token() -> Optional[str]:
    return os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")


def github_repo_url(owner: str, repo: str) -> str:
    return f"https://github.com/{owner}/{repo}"


def extract_owner_repo(repo_url: str) -> Optional[tuple[str, str]]:
    """从 https://github.com/<owner>/<repo> 抽取 (owner, repo)。"""
    for prefix in ("https://github.com/", "http://github.com/"):
        if repo_url.startswith(prefix):
            path = repo_url[len(prefix):].rstrip("/")
            if path.endswith(".git"):
                path = path[:-4]
            parts = path.split("/", 2)
            if len(parts) >= 2 and parts[0] and parts[1]:
                return parts[0], parts[1]
    return None
