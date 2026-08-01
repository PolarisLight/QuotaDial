# QuotaDial README Redesign

Date: 2026-08-01

## Goal

Turn the repository homepage into a concise bilingual product introduction that explains what QuotaDial measures, proves the interface with a real screenshot, and gives visitors a trustworthy path to the current macOS preview release.

## Audience and story

- Audience: Codex users who want account-level quota visibility and developers evaluating or contributing to QuotaDial.
- One-sentence value: QuotaDial shows account quota, remaining-capacity trends, local session Token usage, and equivalent API cost in one native desktop dashboard.
- Primary proof: a readable screenshot of the actual QuotaDial dashboard.
- First successful action: download the latest Apple Silicon DMG from GitHub Releases.
- Limitation: v0.1.0 is an unsigned Apple Silicon preview; Claude and Windows support are planned, not shipped.

## Files

- `README.md`: English default GitHub homepage.
- `README.zh-CN.md`: independent Chinese version with matching structure.
- `assets/readme/hero.svg`: editable, deterministic title visual.
- `assets/readme/dashboard.webp`: optimized real dashboard screenshot.

Both README files link to each other at the top. The existing `app/README.md` remains the app-development note and is not used as the repository homepage.

## Visual direction

- Palette: `#15382E` background, `#F5F3EA` foreground, `#72D6A7` primary, `#8FA7FF` chart accent, `#91A59E` muted.
- Typography: Apple/system sans-serif with monospace only for metadata.
- Shape: 24-unit radius, thin keylines, restrained spacing, no decorative shadow stack.
- Motif: QuotaDial's hexagonal dial fused with a descending remaining-quota curve.
- Composition: calm technical editorial layout, not a badge wall or generic AI illustration.

The 1200-unit SVG hero contains the product name, a concrete description, the quota dial, and a short remaining-capacity trace. It supplies its own dark background, accessible title/description, and legible type at GitHub width. The real screenshot follows immediately instead of being shrunk inside the hero.

## Content order

1. Language switch, pure SVG hero, compact release/platform badges.
2. Real dashboard screenshot and a short product explanation.
3. Four concrete capabilities: account quota, trends and forecast, session detail, equivalent cost.
4. Data-boundary explanation separating account-wide and local-only information.
5. Local privacy statement.
6. Download and macOS installation caveat.
7. Development commands and project stack.
8. Short roadmap stating Claude and Windows as planned.

The README will not claim adoption, Apple notarization, released Claude support, or a Windows build. Commands and critical installation information remain Markdown text rather than SVG content.

## Validation

- Run the installed skill's `audit_readme.py` against both README files.
- Render the SVG and inspect it at approximately 900 px and 360 px widths.
- Verify image paths, language links, release links, accessibility text, and dark/light GitHub contrast.
- Check the final diff and keep unrelated files untouched.

