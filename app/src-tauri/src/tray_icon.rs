use std::f32::consts::{PI, TAU};
use tauri::image::Image;
use tiny_skia::{
    BlendMode, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
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
        lengths[index] = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
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
    let start = (center.0 - direction.0 * 3.2, center.1 - direction.1 * 3.2);
    let end = (center.0 + direction.0 * 11.5, center.1 + direction.1 * 11.5);
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

    let hub =
        PathBuilder::from_circle(center.0 * scale, center.1 * scale, 3.3 * scale).expect("hub");
    pixmap.fill_path(&hub, &solid, FillRule::Winding, Transform::identity(), None);
    if stale {
        let clear = paint(0, BlendMode::Clear);
        let notch = PathBuilder::from_circle(
            (center.0 + 2.4) * scale,
            (center.1 - 2.4) * scale,
            1.25 * scale,
        )
        .expect("stale notch");
        pixmap.fill_path(
            &notch,
            &clear,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn draw_neutral_hub(pixmap: &mut Pixmap, stale: bool, scale: f32) {
    let hub =
        PathBuilder::from_circle(22.0 * scale, 22.0 * scale, 1.8 * scale).expect("neutral hub");
    pixmap.fill_path(
        &hub,
        &paint(if stale { 40 } else { 88 }, BlendMode::SourceOver),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

pub fn render_tray_icon(state: TrayDialState, size: u32) -> Image<'static> {
    let scale = size as f32 / 44.0;
    let mut pixmap = Pixmap::new(size, size).expect("non-zero tray icon size");
    pixmap.stroke_path(
        &closed_hex_path(scale),
        &paint(88, BlendMode::SourceOver),
        &stroke(3.4 * scale),
        Transform::identity(),
        None,
    );

    let fraction = clamped_fraction(state.used_percent);
    if let Some(progress) = fraction.and_then(|value| partial_hex_path(value, scale)) {
        pixmap.stroke_path(
            &progress,
            &paint(255, BlendMode::SourceOver),
            &stroke(5.2 * scale),
            Transform::identity(),
            None,
        );
    }
    draw_ticks(&mut pixmap, scale);

    if let Some(fraction) = fraction {
        draw_hand(&mut pixmap, fraction, state.stale, scale);
    } else {
        draw_neutral_hub(&mut pixmap, state.stale, scale);
    }

    Image::new_owned(pixmap.data().to_vec(), size, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_sum(image: &Image<'_>) -> u64 {
        image
            .rgba()
            .chunks_exact(4)
            .map(|pixel| pixel[3] as u64)
            .sum()
    }

    fn render(used_percent: Option<f32>, stale: bool) -> Image<'static> {
        render_tray_icon(
            TrayDialState {
                used_percent,
                stale,
            },
            44,
        )
    }

    #[test]
    fn renders_template_safe_rgba_at_requested_size() {
        let image = render(Some(50.0), false);
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
        assert_eq!(
            render(Some(-20.0), false).rgba(),
            render(Some(0.0), false).rgba()
        );
        assert_eq!(
            render(Some(140.0), false).rgba(),
            render(Some(100.0), false).rgba()
        );
    }

    #[test]
    fn monochrome_coverage_distinguishes_zero_half_and_full() {
        let zero = render(Some(0.0), false);
        let half = render(Some(50.0), false);
        let full = render(Some(100.0), false);
        assert!(alpha_sum(&zero) < alpha_sum(&half));
        assert!(alpha_sum(&half) < alpha_sum(&full));
    }

    #[test]
    fn stale_state_changes_the_hub_without_discarding_quota() {
        let fresh = render(Some(64.0), false);
        let stale = render(Some(64.0), true);
        assert_ne!(fresh.rgba(), stale.rgba());
        assert!(alpha_sum(&stale) < alpha_sum(&fresh));
    }

    #[test]
    fn no_data_state_is_distinct_from_a_real_zero_percent() {
        let no_data = render(None, false);
        let zero = render(Some(0.0), false);
        assert_ne!(no_data.rgba(), zero.rgba());
        assert!(alpha_sum(&no_data) < alpha_sum(&zero));
    }

    #[test]
    fn stale_no_data_state_is_distinct_from_initial_no_data() {
        let initial = render(None, false);
        let stale = render(None, true);
        assert_ne!(initial.rgba(), stale.rgba());
        assert!(alpha_sum(&stale) < alpha_sum(&initial));
    }
}
