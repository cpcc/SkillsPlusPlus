"""fetch_clawhub: 先域名探活，存活则 scrape，否则跳过。

clawhub.ai 历史上不稳定，不能假设一定可访问。先 GET /，2xx/3xx 才继续。
"""
from __future__ import annotations

import re

from .schema import SkillItem
from ._common import http_client
from selectolax.parser import HTMLParser

BASE = "https://clawhub.ai"
DETAIL_HREF_RE = re.compile(r"/skills/([A-Za-z0-9_\-.]+)")


def _extract_slugs(html: str) -> list[str]:
    found: set[str] = set()
    out: list[str] = []
    for m in DETAIL_HREF_RE.finditer(html):
        slug = m.group(1)
        if slug in found:
            continue
        found.add(slug)
        out.append(slug)
    return out


def _parse_meta(html: str) -> dict:
    tree = HTMLParser(html)
    meta: dict = {}
    for node in tree.css("meta"):
        name = (node.attributes.get("name") or node.attributes.get("property") or "").lower()
        content = node.attributes.get("content")
        if name and content:
            if name in ("description", "og:description"):
                meta.setdefault("description", content)
            elif name == "og:title":
                meta.setdefault("title", content)
    title_node = tree.css_first("title")
    if "title" not in meta and title_node and title_node.text():
        meta["title"] = title_node.text(strip=True)
    return meta


def fetch(max_detail: int = 100) -> list[SkillItem]:
    items: list[SkillItem] = []
    with http_client(timeout=15.0) as client:
        try:
            head = client.get(BASE + "/skills")
        except Exception as e:
            print(f"[clawhub] domain unreachable: {e}; skipping")
            return []
        if head.status_code >= 500:
            print(f"[clawhub] {BASE}/skills status={head.status_code}; skipping")
            return []

        slugs = _extract_slugs(head.text)
        if not slugs:
            print("[clawhub] no slugs on /skills; skipping")
            return []

        for slug in slugs[:max_detail]:
            url = f"{BASE}/skills/{slug}"
            try:
                r = client.get(url)
                if r.status_code >= 400:
                    continue
                meta = _parse_meta(r.text)
            except Exception as e:
                print(f"[clawhub] {url} failed: {e}")
                continue
            title = meta.get("title", slug)
            items.append(SkillItem(
                id=f"registry_clawhub_{slug}",
                name=title,
                description=meta.get("description"),
                sourceId="clawhub",
                repoUrl=None,
                detailUrl=url,
                installStrategy="copy",
            ))
    print(f"[clawhub] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    print(len(fetch(max_detail=20)))
