"""fetch_lobehub: 拉 https://chat-plugins.lobehub.com/index.json。"""
from __future__ import annotations

from .schema import SkillItem
from ._common import http_client


def fetch() -> list[SkillItem]:
    items: list[SkillItem] = []
    try:
        with http_client(timeout=20.0) as client:
            r = client.get("https://chat-plugins.lobehub.com/index.json")
            r.raise_for_status()
            payload = r.json()
    except Exception as e:
        print(f"[lobehub] failed: {e}")
        return []

    for p in payload.get("plugins", []):
        ident = p.get("identifier")
        if not ident:
            continue
        meta = p.get("meta", {}) or {}
        items.append(SkillItem(
            id=f"registry_lobehub_{ident}",
            name=meta.get("title", ident),
            author=p.get("author"),
            description=meta.get("description"),
            tags=meta.get("tags", []) or [],
            sourceId="lobehub",
            repoUrl=p.get("homepage"),
            detailUrl=p.get("homepage") or f"https://lobehub.com/plugins/{ident}",
            updatedAt=p.get("createdAt"),
            compatibleTools=["通用"],
            installStrategy="archive",
        ))
    print(f"[lobehub] fetched {len(items)} skills")
    return items


if __name__ == "__main__":
    print(len(fetch()))
