"""merge_and_push: 聚合去重 → 写本地 skills.json + stats → push HF。

去重规则：同一 repoUrl+name 合并；sourceId 取优先级最高；tags/tools 并集；stars 最大；category 置信度优先。
"""
from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

from .schema import SkillItem, RegistryPayload, Stats
from ._common import source_rank

OUTPUT_PATH = Path("tmp/skills.json")
JSONL_PATH = Path("tmp/skills.jsonl")


def _dedup_key(s: SkillItem) -> str:
    return f"{(s.repoUrl or '').lower()}|{s.name.lower()}"


# category 置信度排序：源原生（已设置的） > 未设置。merge 时若两边都有，保留任一非「其它」。
_CATEGORY_CONFIDENCE = {"其它": 0}


def _pick_category(a: str | None, b: str | None) -> str | None:
    candidates = [c for c in (a, b) if c]
    if not candidates:
        return None
    # 偏好非「其它」
    for c in candidates:
        if c != "其它":
            return c
    return "其它"


def merge(all_items: list[SkillItem]) -> list[SkillItem]:
    bucket: dict[str, SkillItem] = {}
    for s in all_items:
        key = _dedup_key(s)
        if key not in bucket:
            bucket[key] = s
            continue
        existing = bucket[key]
        # 来源优先级
        if source_rank(s.sourceId) < source_rank(existing.sourceId):
            s_merged = s
            s_other = existing
        else:
            s_merged = existing
            s_other = s
        # 字段合并：union tags/tools，max stars，pick category
        merged_tags = list({*s_merged.tags, *s_other.tags})
        merged_tools = list({*s_merged.compatibleTools, *s_other.compatibleTools})
        stars = max(filter(lambda x: x is not None, [s_merged.stars, s_other.stars]), default=None)
        merged = s_merged.model_copy(update={
            "tags": merged_tags,
            "compatibleTools": merged_tools,
            "stars": stars,
            "category": _pick_category(s_merged.category, s_other.category),
            "description": s_merged.description or s_other.description,
            "author": s_merged.author or s_other.author,
            "updatedAt": max(filter(lambda x: x, [s_merged.updatedAt, s_other.updatedAt]), default=None),
        })
        bucket[key] = merged
    return list(bucket.values())


def _stats(skills: list[SkillItem]) -> Stats:
    by_source: dict[str, int] = {}
    by_cat: dict[str, int] = {}
    for s in skills:
        by_source[s.sourceId] = by_source.get(s.sourceId, 0) + 1
        cat = s.category or "其它"
        by_cat[cat] = by_cat.get(cat, 0) + 1
    return Stats(total=len(skills), bySource=by_source, byCategory=by_cat)


def write_local(skills: list[SkillItem]) -> tuple[Path, Path]:
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    payload = RegistryPayload(
        generatedAt=datetime.now(timezone.utc).isoformat(timespec="seconds"),
        stats=_stats(skills),
        skills=skills,
    )
    data = payload.model_dump_json(indent=2)
    OUTPUT_PATH.write_text(data, encoding="utf-8")

    # 副本 jsonl（流式加载用）
    JSONL_PATH.parent.mkdir(parents=True, exist_ok=True)
    with JSONL_PATH.open("w", encoding="utf-8") as f:
        for s in skills:
            f.write(s.model_dump_json() + "\n")

    return OUTPUT_PATH, JSONL_PATH


def push_hf(skills: list[SkillItem]) -> None:
    hf_user = os.environ.get("HF_USER")
    token = os.environ.get("HF_TOKEN")
    if not hf_user or hf_user == "<hf_user>":
        print("[push] HF_USER not configured; skipping push (local file written)")
        return
    if not token:
        print("[push] HF_TOKEN missing; skipping push")
        return

    try:
        from huggingface_hub import HfApi
    except ImportError:
        print("[push] huggingface_hub not installed; skipping push")
        return

    api = HfApi(token=token)
    repo_id = f"{hf_user}/aiskills-registry"
    api.upload_file(
        path_or_fileobj=str(OUTPUT_PATH),
        path_in_repo="skills.json",
        repo_id=repo_id,
        repo_type="dataset",
        commit_message=f"registry sync: {len(skills)} skills @ {datetime.now(timezone.utc).isoformat(timespec='minutes')}",
    )
    try:
        api.upload_file(
            path_or_fileobj=str(JSONL_PATH),
            path_in_repo="skills.jsonl",
            repo_id=repo_id,
            repo_type="dataset",
        )
    except Exception as e:
        print(f"[push] jsonl upload failed (non-fatal): {e}")
    print(f"[push] uploaded {len(skills)} skills to {repo_id}")


def main(skills: list[SkillItem]) -> None:
    merged = merge(skills)
    write_local(merged)
    push_hf(merged)


if __name__ == "__main__":
    # demo：从环境读 sample
    demo = [
        SkillItem(id="a", name="react-debugger", tags=["react"], sourceId="skills_sh",
                  detailUrl="", repoUrl="https://github.com/x/a", stars=10, category="开发编程"),
        SkillItem(id="a2", name="react-debugger", tags=["frontend"], sourceId="lobehub",
                  detailUrl="", repoUrl="https://github.com/x/a", stars=20),
    ]
    p, _ = write_local(merge(demo))
    print(p.read_text()[:500])
