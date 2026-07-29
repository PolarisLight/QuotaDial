# Hex Quota Dial Brand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the liquid-window brand with a hexagonal quota dial, render live monochrome quota state in the macOS menu bar, and regenerate every platform icon from one vector master.

**Architecture:** A static SVG master defines the colored application icon and feeds Tauri's platform icon generator. React mirrors the same static 64% brand state for the sidebar and favicon. Rust owns a focused `tray_icon` renderer that draws a template-safe RGBA dial from the live consumed percentage, so the menu bar remains readable without color and does not depend on pre-generated percentage assets.

**Tech Stack:** React 19, TypeScript, SVG, Vitest, Rust, Tauri 2, tiny-skia, Cargo tests, Tauri icon generator.

---

## File Structure

- Create `app/brand/hex-quota-dial.svg`: canonical 1024px colored application icon.
- Modify `app/brand/tray-template.svg`: 44px monochrome reference master for design review and documentation.
- Delete `app/brand/monitor-window.svg`: retired liquid-window master.
- Modify `app/public/favicon.svg`: static 64% dial derived from the master.
- Modify `app/src/components/BrandMark.tsx`: sidebar SVG using the new dial anatomy.
- Create `app/src/components/BrandMark.test.tsx`: structural and accessibility checks for the React mark.
- Modify `app/src/styles/app.css`: replace liquid-window classes with track, progress, tick, hand, and hub styles.
- Create `app/src-tauri/src/tray_icon.rs`: pure geometry and raster rendering for live template icons.
- Modify `app/src-tauri/src/lib.rs`: expose the new tray icon module.
- Modify `app/src-tauri/src/tray.rs`: remove temporary diagnostics, use the live renderer initially and on every quota snapshot.
- Modify `app/src-tauri/Cargo.toml`: add `tiny-skia` for deterministic antialiased rasterization.
- Modify `app/package.json`: add a repeatable platform icon generation command.
- Regenerate `app/src-tauri/icons/*`: macOS, Windows, iOS, Android, and PNG outputs from the canonical master.
- Delete `app/src-tauri/icons/trayTemplate.png`: no longer used because the tray image is generated from live state.

### Task 1: Build and test the monochrome tray dial renderer

**Files:**
- Create: `app/src-tauri/src/tray_icon.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/Cargo.toml`
- Test: `app/src-tauri/src/tray_icon.rs`

- [ ] **Step 1: Add failing geometry and raster tests**

Add `pub mod tray_icon;` next to the existing `pub mod tray;` declaration in `app/src-tauri/src/lib.rs`.

Create `app/src-tauri/src/tray_icon.rs` with the tests first:

```rust
use tauri::image::Image;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayDialState {
    pub used_percent: Option<f32>,
    pub stale: bool,
}

pub fn render_tray_icon(_state: TrayDialState, _size: u32) -> Image<'static> {
    unimplemented!("implemented after the tests fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_sum(image: &Image<'_>) -> u64 {
        image.rgba().chunks_exact(4).map(|pixel| pixel[3] as u64).sum()
    }

    #[test]
    fn renders_template_safe_rgba_at_requested_size() {
        let image = render_tray_icon(
            TrayDialState {
                used_percent: Some(50.0),
                stale: false,
            },
            44,
        );
        assert_eq!(image.width(), 44);
        assert_eq!(image.height(), 44);
        assert_eq!(image.rgba().len(), 44 * 44 * 4);
        assert!(image
            .rgba()
            .chunks_exact(4)
            .all(|pixel| pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0));
    }

    #[test]
    fn clamps_invalid_percentages() {
        let below = render_tray_icon(
            TrayDialState {
                used_percent: Some(-20.0),
                stale: false,
            },
            44,
        );
        let zero = render_tray_icon(
            TrayDialState {
                used_percent: Some(0.0),
                stale: false,
            },
            44,
        );
        let above = render_tray_icon(
            TrayDialState {
                used_percent: Some(140.0),
                stale: false,
            },
            44,
        );
        let full = render_tray_icon(
            TrayDialState {
                used_percent: Some(100.0),
                stale: false,
            },
            44,
        );
        assert_eq!(below.rgba(), zero.rgba());
        assert_eq!(above.rgba(), full.rgba());
    }

    #[test]
    fn monochrome_coverage_distinguishes_zero_half_and_full() {
        let zero = render_tray_icon(
            TrayDialState {
                used_percent: Some(0.0),
                stale: false,
            },
            44,
        );
        let half = render_tray_icon(
            TrayDialState {
                used_percent: Some(50.0),
                stale: false,
            },
            44,
        );
        let full = render_tray_icon(
            TrayDialState {
                used_percent: Some(100.0),
                stale: false,
            },
            44,
        );
        assert!(alpha_sum(&zero) < alpha_sum(&half));
        assert!(alpha_sum(&half) < alpha_sum(&full));
    }

    #[test]
    fn stale_state_changes_the_hub_without_discarding_quota() {
        let fresh = render_tray_icon(
            TrayDialState {
                used_percent: Some(64.0),
                stale: false,
            },
            44,
        );
        let stale = render_tray_icon(
            TrayDialState {
                used_percent: Some(64.0),
                stale: true,
            },
            44,
        );
        assert_ne!(fresh.rgba(), stale.rgba());
        assert!(alpha_sum(&stale) < alpha_sum(&fresh));
    }

    #[test]
    fn no_data_state_is_distinct_from_a_real_zero_percent() {
        let no_data = render_tray_icon(
            TrayDialState {
                used_percent: None,
                stale: false,
            },
            44,
        );
        let zero = render_tray_icon(
            TrayDialState {
                used_percent: Some(0.0),
                stale: false,
            },
            44,
        );
        assert_ne!(no_data.rgba(), zero.rgba());
        assert!(alpha_sum(&no_data) < alpha_sum(&zero));
    }

    #[test]
    fn stale_no_data_state_is_distinct_from_initial_no_data() {
        let initial = render_tray_icon(
            TrayDialState {
                used_percent: None,
                stale: false,
            },
            44,
        );
        let stale = render_tray_icon(
            TrayDialState {
                used_percent: None,
                stale: true,
            },
            44,
        );
        assert_ne!(initial.rgba(), stale.rgba());
        assert!(alpha_sum(&stale) < alpha_sum(&initial));
    }
}
```

- [ ] **Step 2: Run the focused Rust test and verify failure**

Run:

```bash
cd app/src-tauri
cargo test tray_icon::tests
```

Expected: FAIL because `render_tray_icon` reaches `unimplemented!`.

- [ ] **Step 3: Add tiny-skia and implement the renderer**

Add to `[dependencies]` in `app/src-tauri/Cargo.toml`:

```toml
tiny-skia = "0.11"
```

Replace the placeholder implementation in `tray_icon.rs` with a renderer organized around these exact functions:

```rust
use std::f32::consts::{PI, TAU};
use tauri::image::Image;
use tiny_skia::{
    BlendMode, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayDialState {
    pub used_percent: Option<f32>,
    pub stale: bool,
}

const HEX: [(f32, f32); 6] = [
    (22.0, 5.5),
    (36.5, 14.0),
    (36.5, 30.0),
    (22.0, 38.5),
    (7.5, 30.0),
    (7.5, 14.0),
];

fn clamped_fraction(percent: Option<f32>) -> Option<f32> {
    percent.map(|value| {
        if value.is_finite() {
            value.clamp(0.0, 100.0) / 100.0
        } else {
            0.0
        }
    })
}

fn paint(alpha: u8, blend_mode: BlendMode) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, alpha);
    paint.anti_alias = true;
    paint.blend_mode = blend_mode;
    paint
}

fn stroke(width: f32) -> Stroke {
    Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    }
}

fn closed_hex_path(scale: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.move_to(HEX[0].0 * scale, HEX[0].1 * scale);
    for &(x, y) in &HEX[1..] {
        builder.line_to(x * scale, y * scale);
    }
    builder.close();
    builder.finish().expect("hex path")
}

fn partial_hex_path(fraction: f32, scale: f32) -> Option<Path> {
    if fraction <= 0.0 {
        return None;
    }
    let mut lengths = [0.0_f32; 6];
    for index in 0..6 {
        let from = HEX[index];
        let to = HEX[(index + 1) % 6];
        lengths[index] =
            ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
    }
    let perimeter: f32 = lengths.iter().sum();
    let mut remaining = perimeter * fraction;
    let mut builder = PathBuilder::new();
    builder.move_to(HEX[0].0 * scale, HEX[0].1 * scale);
    for index in 0..6 {
        if remaining <= 0.0 {
            break;
        }
        let from = HEX[index];
        let to = HEX[(index + 1) % 6];
        let length = lengths[index];
        let t = (remaining / length).clamp(0.0, 1.0);
        builder.line_to(
            (from.0 + (to.0 - from.0) * t) * scale,
            (from.1 + (to.1 - from.1) * t) * scale,
        );
        remaining -= length;
    }
    builder.finish()
}

fn draw_ticks(pixmap: &mut Pixmap, scale: f32) {
    let clear = paint(0, BlendMode::Clear);
    let tick_stroke = stroke(2.0 * scale);
    for &(x, y) in &HEX {
        let dx = 22.0 - x;
        let dy = 22.0 - y;
        let length = (dx * dx + dy * dy).sqrt();
        let ux = dx / length;
        let uy = dy / length;
        let mut builder = PathBuilder::new();
        builder.move_to((x + ux * 0.5) * scale, (y + uy * 0.5) * scale);
        builder.line_to((x + ux * 5.0) * scale, (y + uy * 5.0) * scale);
        if let Some(path) = builder.finish() {
            pixmap.stroke_path(&path, &clear, &tick_stroke, Transform::identity(), None);
        }
    }
}

fn draw_hand(pixmap: &mut Pixmap, fraction: f32, stale: bool, scale: f32) {
    let angle = -PI / 2.0 + fraction * TAU;
    let direction = (angle.cos(), angle.sin());
    let center = (22.0, 22.0);
    let start = (
        center.0 - direction.0 * 3.2,
        center.1 - direction.1 * 3.2,
    );
    let end = (
        center.0 + direction.0 * 11.5,
        center.1 + direction.1 * 11.5,
    );
    let mut builder = PathBuilder::new();
    builder.move_to(start.0 * scale, start.1 * scale);
    builder.line_to(end.0 * scale, end.1 * scale);
    let solid = paint(255, BlendMode::SourceOver);
    if let Some(path) = builder.finish() {
        pixmap.stroke_path(
            &path,
            &solid,
            &stroke(3.5 * scale),
            Transform::identity(),
            None,
        );
    }
    pixmap.fill_path(
        &tiny_skia::PathBuilder::from_circle(
            center.0 * scale,
            center.1 * scale,
            3.3 * scale,
        )
        .expect("hub"),
        &solid,
        tiny_skia::FillRule::Winding,
        Transform::identity(),
        None,
    );
    if stale {
        let clear = paint(0, BlendMode::Clear);
        let notch = tiny_skia::PathBuilder::from_circle(
            (center.0 + 2.4) * scale,
            (center.1 - 2.4) * scale,
            1.25 * scale,
        )
        .expect("stale notch");
        pixmap.fill_path(
            &notch,
            &clear,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

pub fn render_tray_icon(state: TrayDialState, size: u32) -> Image<'static> {
    let render_scale = size as f32 / 44.0;
    let mut pixmap = Pixmap::new(size, size).expect("non-zero tray icon size");
    let track = closed_hex_path(render_scale);
    pixmap.stroke_path(
        &track,
        &paint(88, BlendMode::SourceOver),
        &stroke(3.4 * render_scale),
        Transform::identity(),
        None,
    );
    if let Some(fraction) = clamped_fraction(state.used_percent) {
        if let Some(progress) = partial_hex_path(fraction, render_scale) {
            pixmap.stroke_path(
                &progress,
                &paint(255, BlendMode::SourceOver),
                &stroke(5.2 * render_scale),
                Transform::identity(),
                None,
            );
        }
        draw_hand(&mut pixmap, fraction, state.stale, render_scale);
    } else {
        let neutral_hub = tiny_skia::PathBuilder::from_circle(
            22.0 * render_scale,
            22.0 * render_scale,
            1.8 * render_scale,
        )
        .expect("neutral hub");
        pixmap.fill_path(
            &neutral_hub,
            &paint(
                if state.stale { 40 } else { 88 },
                BlendMode::SourceOver,
            ),
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    draw_ticks(&mut pixmap, render_scale);
    Image::new_owned(pixmap.data().to_vec(), size, size)
}
```

- [ ] **Step 4: Run renderer tests and the complete Rust suite**

Run:

```bash
cd app/src-tauri
cargo test tray_icon::tests
cargo test
```

Expected: all renderer tests pass, followed by the complete Rust suite passing.

- [ ] **Step 5: Commit the renderer**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock app/src-tauri/src/lib.rs app/src-tauri/src/tray_icon.rs
git commit -m "feat: render live quota tray icon"
```

### Task 2: Wire live dial state into the Tauri tray

**Files:**
- Modify: `app/src-tauri/src/tray.rs`
- Delete: `app/src-tauri/icons/trayTemplate.png`
- Test: `app/src-tauri/src/tray.rs`

- [ ] **Step 1: Add failing tray state mapping tests**

Add to the existing `tray.rs` test module before defining `tray_dial_state`:

```rust
#[test]
fn maps_snapshot_to_live_dial_state() {
    let snapshot = snapshot_with_quota_and_sessions();
    assert_eq!(
        tray_dial_state(&snapshot),
        crate::tray_icon::TrayDialState {
            used_percent: Some(25.0),
            stale: false,
        }
    );
}

#[test]
fn maps_missing_quota_to_neutral_dial_state() {
    let snapshot = DashboardSnapshot {
        is_stale: true,
        ..DashboardSnapshot::default()
    };
    assert_eq!(
        tray_dial_state(&snapshot),
        crate::tray_icon::TrayDialState {
            used_percent: None,
            stale: true,
        }
    );
}
```

- [ ] **Step 2: Run the focused tray tests**

Run:

```bash
cd app/src-tauri
cargo test tray::tests
```

Expected: FAIL to compile because `tray_dial_state` is not defined yet.

- [ ] **Step 3: Replace the static PNG with rendered state**

Change the imports in `tray.rs` to include:

```rust
use crate::tray_icon::{render_tray_icon, TrayDialState};
```

Add the pure mapper near `tray_title`:

```rust
pub fn tray_dial_state(snapshot: &DashboardSnapshot) -> TrayDialState {
    TrayDialState {
        used_percent: snapshot
            .primary_quota
            .as_ref()
            .map(|quota| quota.used_percent as f32),
        stale: snapshot.is_stale,
    }
}
```

Replace:

```rust
.icon(Image::from_bytes(include_bytes!("../icons/trayTemplate.png"))?)
```

with:

```rust
.icon(render_tray_icon(
    TrayDialState {
        used_percent: None,
        stale: false,
    },
    44,
))
```

Inside the snapshot loop, immediately after calculating `title`, add:

```rust
let _ = tray_updates.set_icon(Some(render_tray_icon(
    tray_dial_state(&snapshot),
    44,
)));
```

Remove `image::Image` from the Tauri import list because the tray no longer decodes a PNG.

Remove the temporary diagnostics block:

```rust
let tray_diagnostics = tray.clone();
tauri::async_runtime::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    log::info!("codex monitor tray rect: {:?}", tray_diagnostics.rect());
});
```

Delete `app/src-tauri/icons/trayTemplate.png`.

- [ ] **Step 4: Verify the tray integration**

Run:

```bash
cd app/src-tauri
cargo test tray::tests
cargo test
```

Expected: all tests pass and no source file references `trayTemplate.png` or logs `tray rect`.

Confirm with:

```bash
rg -n "trayTemplate|tray rect" app/src-tauri
```

Expected: no matches.

- [ ] **Step 5: Commit the tray integration**

```bash
git add app/src-tauri/src/tray.rs app/src-tauri/icons/trayTemplate.png
git commit -m "feat: update tray icon from quota state"
```

### Task 3: Replace the frontend and vector brand assets

**Files:**
- Create: `app/brand/hex-quota-dial.svg`
- Modify: `app/brand/tray-template.svg`
- Delete: `app/brand/monitor-window.svg`
- Modify: `app/public/favicon.svg`
- Modify: `app/src/components/BrandMark.tsx`
- Create: `app/src/components/BrandMark.test.tsx`
- Modify: `app/src/styles/app.css`

- [ ] **Step 1: Add the failing React brand test**

Create `app/src/components/BrandMark.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BrandMark } from "./BrandMark";

describe("BrandMark", () => {
  it("renders the quota dial anatomy with an accessible name", () => {
    const { container } = render(<BrandMark />);
    expect(screen.getByRole("img", { name: "Codex Monitor 额度表盘" })).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__track")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__used")).toBeInTheDocument();
    expect(container.querySelectorAll(".brand-mark__tick")).toHaveLength(6);
    expect(container.querySelector(".brand-mark__hand")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__hub")).toBeInTheDocument();
    expect(container.querySelector(".brand-mark__level")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the focused test and verify failure**

Run:

```bash
cd app
pnpm test -- src/components/BrandMark.test.tsx
```

Expected: FAIL because the current component still exposes the liquid-window classes and accessible name.

- [ ] **Step 3: Replace `BrandMark.tsx`**

Use this static 64% brand state:

```tsx
type BrandMarkProps = {
  className?: string;
};

const ticks = [
  "M22 5.5v4.8",
  "M36.5 14l-4.2 2.4",
  "M36.5 30l-4.2-2.4",
  "M22 38.5v-4.8",
  "M7.5 30l4.2-2.4",
  "M7.5 14l4.2 2.4",
];

export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg
      aria-label="Codex Monitor 额度表盘"
      className={className}
      role="img"
      viewBox="0 0 44 44"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        className="brand-mark__track"
        d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        pathLength="100"
      />
      <path
        className="brand-mark__used"
        d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        pathLength="100"
      />
      {ticks.map((path) => (
        <path className="brand-mark__tick" d={path} key={path} />
      ))}
      <path className="brand-mark__hand" d="M24.5 20 13.1 29.3" />
      <circle className="brand-mark__hub" cx="22" cy="22" r="2.25" />
      <circle className="brand-mark__hub-core" cx="22" cy="22" r=".95" />
    </svg>
  );
}
```

- [ ] **Step 4: Replace the sidebar brand CSS**

Keep the existing `.brand-mark` container and replace the four liquid-window rules with:

```css
.brand-mark__track,
.brand-mark__used {
  fill: none;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 4.4;
}

.brand-mark__track {
  stroke: #f6f5ed;
}

.brand-mark__used {
  stroke: #70d5a5;
  stroke-dasharray: 64 36;
}

.brand-mark__tick {
  fill: none;
  stroke: #1b3029;
  stroke-linecap: round;
  stroke-width: 1.35;
}

.brand-mark__hand {
  fill: none;
  stroke: #f6f5ed;
  stroke-linecap: round;
  stroke-width: 2.25;
}

.brand-mark__hub {
  fill: #f6f5ed;
}

.brand-mark__hub-core {
  fill: #1b3029;
}
```

- [ ] **Step 5: Create the vector masters and favicon**

Create `app/brand/hex-quota-dial.svg` as the canonical 1024px source:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 44 44">
  <rect id="tile" x="1.5" y="1.5" width="41" height="41" rx="10" fill="#19332c"/>
  <g id="quota-dial">
    <path id="total-track" d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
          pathLength="100" fill="none" stroke="#f6f5ed" stroke-width="4.4"
          stroke-linecap="round" stroke-linejoin="round"/>
    <path id="used-track" d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
          pathLength="100" fill="none" stroke="#70d5a5" stroke-width="4.4"
          stroke-dasharray="64 36" stroke-linecap="round" stroke-linejoin="round"/>
    <g id="ticks" fill="none" stroke="#19332c" stroke-width="1.35"
       stroke-linecap="round">
      <path d="M22 5.5v4.8"/><path d="M36.5 14l-4.2 2.4"/>
      <path d="M36.5 30l-4.2-2.4"/><path d="M22 38.5v-4.8"/>
      <path d="M7.5 30l4.2-2.4"/><path d="M7.5 14l4.2 2.4"/>
    </g>
    <path id="hand" d="M24.5 20 13.1 29.3" fill="none" stroke="#f6f5ed"
          stroke-width="2.25" stroke-linecap="round"/>
    <circle id="hub" cx="22" cy="22" r="2.25" fill="#f6f5ed"/>
    <circle id="hub-core" cx="22" cy="22" r=".95" fill="#19332c"/>
  </g>
</svg>
```

Update `app/brand/tray-template.svg` to the 44px monochrome state:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 44 44">
  <path d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        fill="none" stroke="#000" stroke-opacity=".35"
        stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        pathLength="100" fill="none" stroke="#000"
        stroke-width="5.2" stroke-dasharray="64 36"
        stroke-linecap="round" stroke-linejoin="round"/>
  <g fill="none" stroke="#000" stroke-width="2" stroke-linecap="round">
    <path d="M22 5.5v4.8"/><path d="M36.5 14l-4.2 2.4"/>
    <path d="M36.5 30l-4.2-2.4"/><path d="M22 38.5v-4.8"/>
    <path d="M7.5 30l4.2-2.4"/><path d="M7.5 14l4.2 2.4"/>
  </g>
  <path d="M24.5 20 13.1 29.3" fill="none" stroke="#000"
        stroke-width="3.5" stroke-linecap="round"/>
  <circle cx="22" cy="22" r="3.3" fill="#000"/>
</svg>
```

Update `app/public/favicon.svg` with the 44px colored `BrandMark` geometry on the existing green rounded tile. Delete `app/brand/monitor-window.svg`.

- [ ] **Step 6: Run frontend tests and build**

Run:

```bash
cd app
pnpm test -- src/components/BrandMark.test.tsx
pnpm test
pnpm build
```

Expected: the focused test passes, the full frontend suite passes, and Vite production build completes without TypeScript or CSS errors.

- [ ] **Step 7: Commit the frontend and masters**

```bash
git add app/brand app/public/favicon.svg app/src/components/BrandMark.tsx app/src/components/BrandMark.test.tsx app/src/styles/app.css
git commit -m "feat: adopt hex quota dial brand"
```

### Task 4: Regenerate and validate platform assets

**Files:**
- Modify: `app/package.json`
- Regenerate: `app/src-tauri/icons/*`

- [ ] **Step 1: Add the repeatable icon generation script**

Add to `scripts` in `app/package.json`:

```json
"icons:generate": "tauri icon brand/hex-quota-dial.svg --output src-tauri/icons"
```

- [ ] **Step 2: Generate all platform assets**

Run:

```bash
cd app
pnpm icons:generate
```

Expected: Tauri regenerates PNG, ICNS, ICO, Windows Store, iOS, and Android assets under `app/src-tauri/icons`.

- [ ] **Step 3: Verify generated dimensions and stale-asset removal**

Run:

```bash
cd app
file src-tauri/icons/icon.icns src-tauri/icons/icon.ico src-tauri/icons/icon.png
rg -n "monitor-window|brand-mark__level|brand-mark__glint|trayTemplate" brand public src src-tauri --glob '!src-tauri/target/**'
```

Expected: `file` recognizes all three icon formats and `rg` returns no stale brand references.

- [ ] **Step 4: Run all automated verification**

Run:

```bash
cd app
pnpm lint
pnpm test
pnpm build
cd src-tauri
cargo test
```

Expected: lint, all 19 existing frontend tests plus the new brand test, production build, and all Rust tests pass.

- [ ] **Step 5: Build and inspect the macOS application**

Run:

```bash
cd app
pnpm tauri build --debug
```

Launch:

```bash
open "src-tauri/target/debug/bundle/macos/Codex Monitor.app"
```

Inspect:

1. Dock icon at normal and small Dock sizes.
2. Sidebar mark at 34px in light and dark themes.
3. Menu bar template icon at no data, approximately 20%, 50%, and 80%.
4. Menu bar icon against light, dark, and high-contrast menu bars.
5. Pointer has a smooth round tip, short counterweight, and small hub with no triangular spike.
6. Tray title, tooltip, menu actions, and refresh behavior remain unchanged.

- [ ] **Step 6: Commit generated assets and verification script**

```bash
git add app/package.json app/src-tauri/icons
git commit -m "build: regenerate quota dial app icons"
```

### Task 5: Final regression and documentation check

**Files:**
- Verify: `docs/superpowers/specs/2026-07-30-hex-quota-dial-brand-design.md`
- Verify: all files changed in Tasks 1-4

- [ ] **Step 1: Compare implementation against the accepted spec**

Confirm:

- start position is twelve o'clock and progress grows clockwise;
- pointer and progress endpoint use the same clamped percentage;
- monochrome icons use thin total track plus thick consumed track;
- 0% and 100% differ in track weight even though the pointer shares a direction;
- no-data, stale, and fresh states have distinct geometry;
- static brand state is not presented as live account data;
- the old liquid window, audio-wave, and interwoven flower shapes are absent.

- [ ] **Step 2: Run the final clean verification**

Run:

```bash
cd app
pnpm lint
pnpm test
pnpm build
cd src-tauri
cargo test
git status --short
```

Expected: every command passes. `git status --short` shows only intentionally uncommitted user changes, if any.

- [ ] **Step 3: Record final visual evidence**

Capture:

- one screenshot of the full dashboard with the new sidebar mark;
- one screenshot of the opened menu bar menu at a real quota value;
- one close crop of the Dock icon and menu bar template icon.

Save temporary evidence outside tracked source files unless the user asks to keep it in the repository.

- [ ] **Step 4: Commit any verification-only correction**

Only if visual inspection required a correction, stage the corrected brand files reported by
`git diff --name-only` explicitly, then commit them. For example, if the optical correction
touches the canonical SVG and sidebar mark:

```bash
git add app/brand/hex-quota-dial.svg app/src/components/BrandMark.tsx app/src/styles/app.css
git commit -m "fix: refine quota dial optical balance"
```

If no correction was required, do not create an empty commit.
