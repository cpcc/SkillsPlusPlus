from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from registry import classify as classify_module
from registry.schema import SkillItem


def make_skill(
    skill_id: str,
    name: str,
    *,
    description: str | None = None,
    tags: list[str] | None = None,
) -> SkillItem:
    return SkillItem(
        id=skill_id,
        name=name,
        description=description,
        tags=tags or [],
        sourceId="registry",
        detailUrl="https://example.com/skill",
    )


class ClassifyTests(unittest.TestCase):
    def test_rule_classification_works_without_llm_key(self) -> None:
        with patch.dict(os.environ, {}, clear=False):
            items = [
                make_skill("dev", "React Debug Helper", tags=["react"]),
                make_skill("unknown", "Aurora Atlas"),
            ]

            result = classify_module.classify(items)

        self.assertEqual(result[0].category, "开发编程")
        self.assertEqual(result[1].category, "其它")

    def test_blank_llm_api_key_is_treated_as_disabled(self) -> None:
        with patch.dict(os.environ, {"LLM_API_KEY": "   \n\t  "}, clear=False):
            with patch.object(
                classify_module,
                "_llm_classify_batch",
                side_effect=AssertionError("blank key should skip LLM classification"),
            ):
                result = classify_module.classify([make_skill("unknown", "Aurora Atlas")])

        self.assertEqual(result[0].category, "其它")

    def test_llm_mapping_applies_when_key_exists(self) -> None:
        with patch.dict(os.environ, {"LLM_API_KEY": "test-key"}, clear=False):
            with patch.object(
                classify_module,
                "_llm_classify_batch",
                return_value={0: "科研"},
            ) as batch_mock:
                result = classify_module.classify([make_skill("unknown", "Aurora Atlas")])

        batch_mock.assert_called_once()
        self.assertEqual(result[0].category, "科研")


if __name__ == "__main__":
    unittest.main()