"""classify: 给 skill 分配 17 类之一。

优先级：
1. 源原生 category（skillsmp / skillhub 已经有）
2. 规则映射：name + tags + description 关键词命中 TOPIC_TO_CATEGORY
3. LLM 兜底（可选）：环境变量 LLM_API_KEY 配置时调用 Anthropic Claude Haiku
4. 默认「其它」
"""
from __future__ import annotations

import json
import os
import re
from typing import Optional

from .schema import SkillItem, CATEGORIES

# 关键词 → 类别。匹配时小写比较；命中即归类。
# 列表顺序即优先级（先命中先返回）。
RULES: list[tuple[list[str], str]] = [
    (["react", "vue", "nextjs", "nuxt", "svelte", "angular", "rust", "python", "typescript", "javascript",
      "go-", "golang", "java ", "c++", "csharp", "kotlin", "swift", "code", "coding", "debug", "refactor",
      "git ", "docker", "kubernetes", "lint", "compiler", "framework", "sdk", "api-client", "cli",
      "leetcode", "program"], "开发编程"),
    (["sql", "database", "etl", "pandas", "numpy", "jupyter", "notebook", "visualization",
      "chart", "tableau", "powerbi", "data-pipeline", "excel-公式", "spreadsheet"], "数据分析"),
    (["finance", "trading", "stock", " ETF", "portfolio", "kline", "k线", "quant",
      "forex", "bond", "财报", "投资", "股票", "基金", "理财"], "金融"),
    (["legal", "lawyer", "contract", "法规", "法律", "合同", "诉讼"], "法律"),
    (["internet", "seo", "marketing", "广告", "运营", "增长"], "互联网"),
    (["research", "paper", "academic", "论文", "学术", "科研", "文献", "arxiv", "scholar"], "科研"),
    (["education", "tutorial", "course", "lesson", "teach", "student", "学习", "教程", "课程", "教学"], "教育"),
    (["health", "medical", "doctor", "patient", "病历", "医学", "健康", "药品"], "健康医疗"),
    (["office", "document", "word", "powerpoint", "ppt", "meeting", "notes", "calendar",
      "邮件", "会议", "文档", "效率"], "办公效率"),
    (["content", "writing", "blog", "copywriting", "story", "writer", "自媒体",
      "公众号", "小红书", "twitter", "文案", "写作", "创作"], "内容创作"),
    (["knowledge", "notes", "wiki", "obsidian", "notion", "笔记", "知识库", "记忆"], "知识管理"),
    (["business", "startup", "crm", "sales", "ops", "hr", "招聘", "客户",
      "团队", "组织", "经营"], "商业运营"),
    (["security", "devops", "monitor", "incident", "logs", "运维", "安全",
      "漏洞", "漏洞扫描", "防火墙"], "IT 运维与安全"),
    (["video", "youtube", "tiktok", "douyin", "bilibili", "播客", "podcast",
      "直播", "短视频", "剪辑"], "自媒体"),
    (["food", "travel", "shopping", "weather", "map", "calendar-event",
      "生活", "美食", "旅游", "购物", "天气", "地图"], "生活服务"),
    (["utility", "tool", "tools", "helper", "通用", "general", "toolkit"], "通用工具"),
]


def _get_llm_api_key() -> str:
    return os.environ.get("LLM_API_KEY", "").strip()


def _match_category(skill: SkillItem) -> Optional[str]:
    if skill.category and skill.category in CATEGORIES:
        return skill.category
    haystack_parts = [skill.name.lower()]
    if skill.description:
        haystack_parts.append(skill.description.lower())
    for tag in skill.tags:
        haystack_parts.append(tag.lower())
    haystack = " | ".join(haystack_parts)
    for keywords, cat in RULES:
        for kw in keywords:
            if kw.lower() in haystack:
                return cat
    return None


def _llm_classify_batch(skills: list[SkillItem]) -> dict[int, str]:
    """调用 Claude Haiku 批量分类。返回 {index_in_skills: category}。

    无 LLM_API_KEY 时返回空 dict（即不使用 LLM）。
    """
    api_key = _get_llm_api_key()
    model = os.environ.get("LLM_MODEL", "claude-haiku-4-5-20251001")
    if not api_key:
        return {}

    try:
        import anthropic
    except ImportError:
        print("[classify] anthropic SDK not installed; skipping LLM")
        return {}

    client = anthropic.Anthropic(api_key=api_key)
    sys_prompt = (
        "你是 AI skill 分类器。把给定的 skill 列表分类到以下 17 类之一：\n"
        + "、".join(CATEGORIES) + "\n"
        "严格输出 JSON：{\"results\": [{\"id\": \"<skill id>\", \"category\": \"<类名>\"}, ...]}。"
        "不要输出其它内容。无法判断时填「其它」。"
    )
    user_payload = json.dumps([
        {"id": s.id, "name": s.name,
         "description": (s.description or "")[:200],
         "tags": s.tags[:8]}
        for s in skills
    ], ensure_ascii=False)

    try:
        resp = client.messages.create(
            model=model,
            max_tokens=4096,
            system=sys_prompt,
            messages=[{"role": "user", "content": user_payload}],
        )
        text = "".join(block.text for block in resp.content if hasattr(block, "text"))
        parsed = json.loads(re.sub(r"^.*?{", "{", text, count=1, flags=re.DOTALL))
        out: dict[int, str] = {}
        id_to_idx = {s.id: i for i, s in enumerate(skills)}
        for r in parsed.get("results", []):
            rid = r.get("id")
            cat = r.get("category")
            if rid in id_to_idx and cat in CATEGORIES:
                out[id_to_idx[rid]] = cat
        return out
    except Exception as e:
        print(f"[classify] LLM batch failed: {e}")
        return {}


def classify(items: list[SkillItem]) -> list[SkillItem]:
    # 1) 先用规则 + 源原生分一批
    needs_llm: list[tuple[int, SkillItem]] = []
    for i, s in enumerate(items):
        cat = _match_category(s)
        if cat:
            s.category = cat
        else:
            needs_llm.append((i, s))

    # 2) LLM 兜底（可选），分批
    BATCH = 25
    if needs_llm and _get_llm_api_key():
        for start in range(0, len(needs_llm), BATCH):
            chunk = needs_llm[start:start + BATCH]
            mapping = _llm_classify_batch([s for _, s in chunk])
            for j, (idx, s) in enumerate(chunk):
                if j in mapping:
                    s.category = mapping[j]
                else:
                    s.category = "其它"
        # 未进入 LLM 批次的（如果切批时丢）也兜底
        for _, s in needs_llm:
            if not s.category:
                s.category = "其它"
    else:
        for _, s in needs_llm:
            s.category = "其它"

    # 防御性：所有 skill 都必须有非空 category
    for s in items:
        if not s.category:
            s.category = "其它"

    by_cat: dict[str, int] = {}
    for s in items:
        by_cat[s.category or "其它"] = by_cat.get(s.category or "其它", 0) + 1
    print(f"[classify] done. distribution: {by_cat}")
    return items


if __name__ == "__main__":
    demo = [SkillItem(id="a", name="react-debugger", tags=["react"], sourceId="x", detailUrl=""),
            SkillItem(id="b", name="foo-bar", sourceId="x", detailUrl="")]
    for s in classify(demo):
        print(s.id, s.category)
