# QuotaDial README Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish a polished English/Chinese repository homepage built from QuotaDial's real identity, interface, release, and data boundaries.

**Architecture:** Use two root Markdown files with identical information architecture, one deterministic SVG hero, and one optimized real dashboard screenshot. Keep commands, release caveats, privacy, and roadmap claims in searchable Markdown.

**Tech Stack:** GitHub Markdown, hand-authored SVG, WebP, `beautify-github-readme` audit script.

---

### Task 1: Create the visual proof assets

**Files:**
- Create: `assets/readme/hero.svg`
- Create: `assets/readme/dashboard.webp`

- [ ] **Step 1: Create a 1200×390 SVG hero**

Use the approved palette and draw the QuotaDial wordmark, concrete account-quota description, hexagonal dial, and descending remaining-quota trace. Include `<title>`, `<desc>`, a full background, and system fonts; do not use scripts, remote resources, or `foreignObject`.

- [ ] **Step 2: Produce the real dashboard proof image**

Convert the user-provided 1848×994 dashboard capture to WebP at repository width. Preserve the interface without mock data edits.

- [ ] **Step 3: Render and inspect the hero**

Run `sips -s format png assets/readme/hero.svg --out /tmp/quotadial-readme-hero.png` and inspect the render for clipping, contrast, and visual identity.

### Task 2: Write the bilingual repository homepage

**Files:**
- Create: `README.md`
- Create: `README.zh-CN.md`

- [ ] **Step 1: Write the English README**

Order content as language switch, hero, release/platform badges, screenshot, capabilities, account/local data boundary, privacy, download, development, and roadmap. Link the v0.1.0 release and state Apple Silicon plus unsigned/notarized limitations.

- [ ] **Step 2: Write the Chinese README**

Mirror the English information architecture in natural Chinese. Keep technical claims, links, commands, and limitations consistent rather than translating slogans literally.

- [ ] **Step 3: Keep development notes in place**

Do not modify `app/README.md`; it remains the app-level development note.

### Task 3: Audit and hand off

**Files:**
- Verify: `README.md`
- Verify: `README.zh-CN.md`
- Verify: `assets/readme/hero.svg`
- Verify: `assets/readme/dashboard.webp`

- [ ] **Step 1: Run README audits**

Run the installed `scripts/audit_readme.py` against both root README files and fix every actionable failure.

- [ ] **Step 2: Verify local paths and repository claims**

Check image files, language links, GitHub Release URLs, `git diff --check`, and the absence of unsupported Claude/Windows claims.

- [ ] **Step 3: Commit the completed README redesign**

Stage only the two README files, two visual assets, and this plan; commit as `docs: redesign bilingual project readme`.

