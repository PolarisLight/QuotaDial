# Usage Chart Tooltips Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add compact interactive value tooltips to Token bars and remaining-quota points without repeating chart labels.

**Architecture:** `UsageQuotaChart` owns one active tooltip state and renders a single HTML overlay positioned from SVG coordinates. SVG bars and quota points expose pointer, focus, and click interactions while retaining complete accessible labels.

**Tech Stack:** React, TypeScript, SVG, Vitest, Testing Library, CSS

---

### Task 1: Interactive chart values

**Files:**
- Modify: `app/src/components/UsageQuotaChart.tsx`
- Modify: `app/src/components/UsageQuotaChart.test.tsx`
- Modify: `app/src/styles/app.css`

- [ ] **Step 1: Write failing component tests**

Add tests that use `fireEvent.mouseEnter`, `fireEvent.mouseLeave`, `fireEvent.focus`, and `fireEvent.blur` against elements identified by `aria-label`. Assert that the visible element with `role="tooltip"` contains only `70,000` for a Token bar or `60.0%` for a quota point, and disappears when interaction ends.

```tsx
fireEvent.mouseEnter(screen.getByLabelText("2026-07-29，70,000 Token"));
expect(screen.getByRole("tooltip")).toHaveTextContent("70,000");
expect(screen.getByRole("tooltip")).not.toHaveTextContent("07/29");
fireEvent.mouseLeave(screen.getByLabelText("2026-07-29，70,000 Token"));
expect(screen.queryByRole("tooltip")).toBeNull();
```

- [ ] **Step 2: Run the focused tests and verify failure**

Run:

```bash
pnpm test -- src/components/UsageQuotaChart.test.tsx
```

Expected: FAIL because the SVG targets and tooltip do not exist.

- [ ] **Step 3: Implement one active tooltip state**

Add a `ChartTooltip` state with `value`, `x`, `y`, and `kind`. Make each data-bearing bar and every observed quota point focusable and interactive. Render one tooltip overlay after the SVG:

```tsx
{tooltip && (
  <output
    className={`usage-chart-tooltip ${tooltip.kind}`}
    role="tooltip"
    style={{
      left: `${(tooltip.x / WIDTH) * 100}%`,
      top: `${(tooltip.y / HEIGHT) * 132}px`,
    }}
  >
    {tooltip.value}
  </output>
)}
```

Token values use `toLocaleString("zh-CN")`; quota values use `toFixed(1) + "%"`. Accessible labels retain the date/time and series context even though the visible tooltip contains only the value.

- [ ] **Step 4: Add visual and focus styling**

Add `.usage-chart-tooltip`, `.token-bar.interactive`, `.remaining-quota-point.interactive`, `:hover`, and `:focus-visible` rules. The tooltip uses existing surface, border, text, and shadow variables, ignores pointer events, and clamps horizontal placement with `translateX()` classes selected from the anchor position.

- [ ] **Step 5: Run complete verification**

Run:

```bash
pnpm test
pnpm build
pnpm lint
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all tests and builds pass; lint has no new warnings.

- [ ] **Step 6: Browser verification and commit**

Verify hover, keyboard focus, edge placement, and dark-theme contrast in the local preview. Commit:

```bash
git add app/src/components/UsageQuotaChart.tsx app/src/components/UsageQuotaChart.test.tsx app/src/styles/app.css
git commit -m "feat: add interactive usage chart tooltips"
```
