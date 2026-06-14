#!/usr/bin/env bash
# Usage: ./generate-release-body.sh <tag>
# Generates a Chinese-only release body for skills++ releases.
set -euo pipefail

TAG="${1:?Usage: $0 <tag>}"
VERSION="${TAG#v}"

# ── Determine previous version tag (nearest v* tag in ancestry) ──────────
PREV_TAG="$(git describe --abbrev=0 --tags --match 'v*' "${TAG}~1" 2>/dev/null || echo "")"

# ── Collect stats ───────────────────────────────────────────────────────
if [ -n "$PREV_TAG" ]; then
  COMMITS="$(git rev-list --count "$PREV_TAG..$TAG" 2>/dev/null || echo "0")"
  STATS_LINE="$(git diff --stat "$PREV_TAG" "$TAG" 2>/dev/null | tail -1 || true)"
  FILES_CHANGED="$(echo "$STATS_LINE" | sed -E -n 's/.* ([0-9]+) files? changed.*/\1/p' || echo "0")"
  INSERTIONS="$(echo "$STATS_LINE" | sed -E -n 's/.* ([0-9]+) insertion.*/\1/p' || echo "0")"
  DELETIONS="$(echo "$STATS_LINE" | sed -E -n 's/.* ([0-9]+) deletion.*/\1/p' || echo "0")"
  [ -z "$FILES_CHANGED" ] && FILES_CHANGED="0"
  [ -z "$INSERTIONS" ] && INSERTIONS="0"
  [ -z "$DELETIONS" ] && DELETIONS="0"

  # Categorize commits
  get_commits() {
    git log "$PREV_TAG..$TAG" --oneline --format='- %s' 2>/dev/null
  }
  FEAT_COMMITS="$(git log "$PREV_TAG..$TAG" --oneline --format='- %s' 2>/dev/null | grep -iE '^- (feat|feature)[:(]' || true)"
  FIX_COMMITS="$(git log "$PREV_TAG..$TAG" --oneline --format='- %s' 2>/dev/null | grep -iE '^- (fix|bug)[:(]' || true)"
  OTHER_COMMITS="$(get_commits | grep -viE '^- (feat|feature|fix|bug)[:(]' || true)"

  # Contributors (deduplicated)
  CONTRIBUTORS="$(git log "$PREV_TAG..$TAG" --format='%an <%ae>' 2>/dev/null | sort -u | sed 's/^/- /' || true)"
  [ -z "$CONTRIBUTORS" ] && CONTRIBUTORS="- @$(git config user.name || echo 'developer')"
else
  COMMITS="0"
  FILES_CHANGED="0"
  INSERTIONS="0"
  DELETIONS="0"
  FEAT_COMMITS=""
  FIX_COMMITS=""
  OTHER_COMMITS=""
  CONTRIBUTORS="- @$(git config user.name || echo 'developer')"
fi

RELEASE_DATE="$(date +%Y-%m-%d)"
TIMESTAMP="$(date +%s)"
REPO="$(git remote get-url origin 2>/dev/null | sed -E 's|https://github.com/||; s|git@github.com:||; s|\.git$||' || echo 'owner/repo')"
COMPARE_LINK="https://github.com/${REPO}/compare/${PREV_TAG:-$(git rev-list --max-parents=0 HEAD | head -1)}...${TAG}"

# ── Build release body ──────────────────────────────────────────────────
cat << BODY
<!--
######################################################################
  skills++ v${VERSION}
  中文
  Generated: ${RELEASE_DATE}
######################################################################
-->

# skills++ v${VERSION}

---

## 概览

<!--
请简要描述此版本的主要内容、目标或亮点。后续修改时删掉此注释即可。
-->

**发布日期**：${RELEASE_DATE}
**更新规模**：${COMMITS} commits · ${FILES_CHANGED} files changed · +${INSERTIONS} / -${DELETIONS}

## ✨ 新功能

${FEAT_COMMITS}

## 🐛 修复

${FIX_COMMITS}

## 🔧 其他变更

${OTHER_COMMITS}

> 完整提交历史：[${PREV_TAG}..${TAG}](${COMPARE_LINK})

## 👥 Contributors

${CONTRIBUTORS}

---

## Assets

**Linux** (x86_64)
- skills++-v${VERSION}-Linux-x86_64.AppImage
- skills++-v${VERSION}-Linux-x86_64.deb

**macOS**
- skills++-v${VERSION}-macOS-arm64.dmg（Apple Silicon）
- skills++-v${VERSION}-macOS-x86_64.dmg（Intel）

**Windows** (x86_64)
- skills++-v${VERSION}-Windows-x86_64-setup.exe（NSIS）
- skills++-v${VERSION}-Windows-x86_64.msi

> 各文件 SHA256 校验和见下方 Assets 列表。

BODY
