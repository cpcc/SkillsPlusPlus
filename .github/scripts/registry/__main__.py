"""编排入口：fetch_all → enrich → classify → merge_and_push。

直接 `python -m registry` 调用（cwd 为仓库根）。
"""
from __future__ import annotations

import sys
from pathlib import Path

# 让 `.github/scripts/` 在 sys.path 中，无论 cwd。
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from registry import (  # noqa: E402
    fetch_skills_sh, fetch_github_topic, fetch_lobehub,
    fetch_skillsmp, fetch_skillhub, fetch_clawhub,
    enrich_github, classify, merge_and_push,
)


def fetch_all() -> list:
    items: list = []
    for fn in (fetch_skillsmp.fetch, fetch_skills_sh.fetch, fetch_github_topic.fetch,
               fetch_lobehub.fetch, fetch_skillhub.fetch, fetch_clawhub.fetch):
        try:
            items.extend(fn())
        except Exception as e:
            print(f"[main] {fn.__module__} failed: {e}")
    print(f"[main] raw total: {len(items)}")
    return items


def main() -> int:
    raw = fetch_all()
    enriched = enrich_github.enrich(raw)
    classified = classify.classify(enriched)
    merge_and_push.main(classified)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
