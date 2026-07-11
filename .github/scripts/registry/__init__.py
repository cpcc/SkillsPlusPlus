"""CI 聚合脚本：抓取多来源 skill → 补 GitHub 元数据 → 分类 → 聚合 → push HF。

由 `.github/workflows/registry-sync.yml` 调用。各 fetcher 单独可执行，
`__main__.py` 是编排入口。
"""
