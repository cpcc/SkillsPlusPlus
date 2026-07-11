"""fetch_skillhub: HTML scrape skillhub.cn。

无公开 API；URL 规律明确：详情页 `https://skillhub.cn/skills/<slug>`。
抓取策略：从首页或 `/skills` 索引页解析列表，逐条抓详情页元数据。
带缓存避免重复抓。失败容忍。
"""
from __future__ import annotations

import re
import time
from typing import Optional

from .schema import SkillItem
from ._common import http_client
from selectolax.parser import HTMLParser

INDEX_URLS = [
    "https://skillhub.cn/skills",
    "https://skillhub.cn/",
]
DETAIL_HREF_RE = re.compile(r"/skills/([A-Za-z0-9_\-.]+)")


def _extract_slugs(html: str) -> list[str]:
    seen: list[str] = []
    found: set[str] = set()
    for m in DETAIL_HREF_RE.finditer(html):
        slug = m.group(1)
        if slug in found:
            continue
        found.add(slug)
        seen.append(slug)
    return seen


def _parse_meta(html: str) -> dict:
    """从详情页提取 description / og 标签。宽容解析。"""
    tree = HTMLParser(html)
    meta: dict = {}
    for node in tree.css("meta"):
        name = (node.attributes.get("name") or node.attributes.get("property") or "").lower()
        content = node.attributes.get("content")
        if not name or not content:
            continue
        if name in ("description", "og:description"):
            meta.setdefault("description", content)
        elif name in ("og:title",):
            meta.setdefault("title", content)
    # <title> 兜底
    title_node = tree.css_first("title")
    if "title" not in meta and title_node and title_node.text():
        meta["title"] = title_node.text(strip=True)
    return meta


def fetch(max_detail: int = 200) -> list[SkillItem]:
    items: list[SkillItem] = []
    slugs: list[str] = []
    with http_client(timeout=20.0) as client:
        for url in INDEX_URLS:
            try:
                r = client.get(url)
                if r.status_code >= 400:
                    continue
                slugs = _extract_slugs(r.text)
                if slugs:
                    print(f"[skillhub] index {url}: {len(slugs)} slugs")
                    break
            except Exception as e:
                print(f"[skillhub] index {url} failed: {e}")

        if not slugs:
            print("[skillhub] no slugs discovered; skipping")
            return []

        for slug in slugs[:max_detail]:
            detail_url = f"https://skillhub.cn/skills/{slug}"
            try:
                r = client.get(detail_url)
                if r.status_code >= 400:
                    continue
                meta = _parse_meta(r.text)
            except Exception as e:
                print(f"[skillhub] {detail_url} failed: {e}")
                continue
            title = meta.get("title", slug)
            items.append(SkillItem(
                id=f"registry_skillhub_{slug}",
                name=title,
                description=meta.get("description"),
                sourceId="skillhub",
                repoUrl=None,
                detailUrl=detail_url,
                installStrategy="copy",
            ))
            time.sleep(0.3)  # 礼貌限速
    print(f"[skillhub] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    print(len(fetch(max_detail=20)))
