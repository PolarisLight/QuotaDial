use std::f32::consts::{PI, TAU};
use tauri::image::Image;
use tiny_skia::{
    BlendMode, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke,
    Transform,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayDialState {
    pub used_percent: Option<f32>,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconStyle {
    Template,
    Color,
}

#[derive(Clone, Copy)]
struct Palette {
    outline: (u8, u8, u8, u8),
    active: (u8, u8, u8, u8),
    hand: (u8, u8, u8, u8),
    neutral: (u8, u8, u8, u8),
}

const TEMPLATE_PALETTE: Palette = Palette {
    outline: (0, 0, 0, 88),
    active: (0, 0, 0, 255),
    hand: (0, 0, 0, 255),
    neutral: (0, 0, 0, 88),
};

const COLOR_PALETTE: Palette = Palette {
    outline: (91, 105, 130, 230),
    active: (58, 202, 145, 255),
    hand: (246, 249, 252, 255),
    neutral: (142, 157, 181, 220),
};

const HEX: [(f32, f32); 6] = [
    (22.0, 5.5),
    (36.5, 14.0),
    (36.5, 30.0),
    (22.0, 38.5),
    (7.5, 30.0),
    (7.5, 14.0),
];

const WINDOWS_HEX: [(f32, f32); 6] = [
    (16.0, 1.0),
    (30.5, 8.8),
    (30.5, 23.2),
    (16.0, 31.0),
    (1.5, 23.2),
    (1.5, 8.8),
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

fn paint(color: (u8, u8, u8, u8), blend_mode: BlendMode) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
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

fn windows_hex_path(scale: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.move_to(WINDOWS_HEX[0].0 * scale, WINDOWS_HEX[0].1 * scale);
    for &(x, y) in &WINDOWS_HEX[1..] {
        builder.line_to(x * scale, y * scale);
    }
    builder.close();
    builder.finish().expect("Windows hex path")
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
    let clear = paint((0, 0, 0, 0), BlendMode::Clear);
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

fn draw_hand(pixmap: &mut Pixmap, fraction: f32, stale: bool, scale: f32, palette: Palette) {
    let angle = -PI / 2.0 + fraction * TAU;
    let direction = (angle.cos(), angle.sin());
    let center = (22.0, 22.0);
    let start = (center.0 - direction.0 * 3.2, center.1 - direction.1 * 3.2);
    let end = (center.0 + direction.0 * 11.5, center.1 + direction.1 * 11.5);
    let mut builder = PathBuilder::new();
    builder.move_to(start.0 * scale, start.1 * scale);
    builder.line_to(end.0 * scale, end.1 * scale);
    let solid = paint(palette.hand, BlendMode::SourceOver);
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
        let clear = paint((0, 0, 0, 0), BlendMode::Clear);
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

fn draw_neutral_hub(pixmap: &mut Pixmap, stale: bool, scale: f32, palette: Palette) {
    let hub =
        PathBuilder::from_circle(22.0 * scale, 22.0 * scale, 1.8 * scale).expect("neutral hub");
    let mut color = palette.neutral;
    if stale {
        color.3 = color.3.saturating_div(2);
    }
    pixmap.fill_path(
        &hub,
        &paint(color, BlendMode::SourceOver),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

pub fn render_tray_icon(state: TrayDialState, size: u32, style: TrayIconStyle) -> Image<'static> {
    let palette = match style {
        TrayIconStyle::Template => TEMPLATE_PALETTE,
        TrayIconStyle::Color => COLOR_PALETTE,
    };
    let scale = size as f32 / 44.0;
    let mut pixmap = Pixmap::new(size, size).expect("non-zero tray icon size");
    pixmap.stroke_path(
        &closed_hex_path(scale),
        &paint(palette.outline, BlendMode::SourceOver),
        &stroke(3.4 * scale),
        Transform::identity(),
        None,
    );

    let fraction = clamped_fraction(state.used_percent);
    if let Some(progress) = fraction.and_then(|value| partial_hex_path(value, scale)) {
        pixmap.stroke_path(
            &progress,
            &paint(palette.active, BlendMode::SourceOver),
            &stroke(5.2 * scale),
            Transform::identity(),
            None,
        );
    }
    draw_ticks(&mut pixmap, scale);

    if let Some(fraction) = fraction {
        draw_hand(&mut pixmap, fraction, state.stale, scale, palette);
    } else {
        draw_neutral_hub(&mut pixmap, state.stale, scale, palette);
    }

    Image::new_owned(pixmap.data().to_vec(), size, size)
}

fn fill_rounded_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: (u8, u8, u8, u8),
) {
    let solid = paint(color, BlendMode::SourceOver);
    let horizontal = Rect::from_xywh(x + radius, y, width - radius * 2.0, height)
        .expect("horizontal rounded rectangle body");
    let vertical = Rect::from_xywh(x, y + radius, width, height - radius * 2.0)
        .expect("vertical rounded rectangle body");
    for rect in [horizontal, vertical] {
        pixmap.fill_path(
            &PathBuilder::from_rect(rect),
            &solid,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    for (center_x, center_y) in [
        (x + radius, y + radius),
        (x + width - radius, y + radius),
        (x + radius, y + height - radius),
        (x + width - radius, y + height - radius),
    ] {
        let circle = PathBuilder::from_circle(center_x, center_y, radius).expect("rounded corner");
        pixmap.fill_path(
            &circle,
            &solid,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn draw_segment_digit(
    pixmap: &mut Pixmap,
    digit: u8,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    thickness: f32,
    color: (u8, u8, u8, u8),
) {
    const SEGMENTS: [u8; 10] = [
        0b011_1111, 0b000_0110, 0b101_1011, 0b100_1111, 0b110_0110, 0b110_1101, 0b111_1101,
        0b000_0111, 0b111_1111, 0b110_1111,
    ];
    let mask = SEGMENTS[digit as usize];
    let middle_y = y + height / 2.0 - thickness / 2.0;
    let vertical_height = height / 2.0 - thickness * 1.25;
    let horizontal_width = width - thickness * 2.0;
    let segments = [
        (x + thickness, y, horizontal_width, thickness),
        (
            x + width - thickness,
            y + thickness * 0.75,
            thickness,
            vertical_height,
        ),
        (
            x + width - thickness,
            y + height / 2.0 + thickness * 0.5,
            thickness,
            vertical_height,
        ),
        (
            x + thickness,
            y + height - thickness,
            horizontal_width,
            thickness,
        ),
        (
            x,
            y + height / 2.0 + thickness * 0.5,
            thickness,
            vertical_height,
        ),
        (x, y + thickness * 0.75, thickness, vertical_height),
        (x + thickness, middle_y, horizontal_width, thickness),
    ];
    for (index, (segment_x, segment_y, segment_width, segment_height)) in
        segments.into_iter().enumerate()
    {
        if mask & (1 << index) != 0 {
            fill_rounded_rect(
                pixmap,
                segment_x,
                segment_y,
                segment_width,
                segment_height,
                thickness * 0.42,
                color,
            );
        }
    }
}

pub fn render_windows_percentage_icon(
    remaining_percent: Option<f32>,
    stale: bool,
    size: u32,
) -> Image<'static> {
    let scale = size as f32 / 32.0;
    let mut pixmap = Pixmap::new(size, size).expect("non-zero tray icon size");
    let hex = windows_hex_path(scale);
    pixmap.fill_path(
        &hex,
        &paint((20, 35, 30, 252), BlendMode::SourceOver),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    pixmap.stroke_path(
        &hex,
        &paint(
            (112, 213, 165, if stale { 185 } else { 255 }),
            BlendMode::SourceOver,
        ),
        &stroke(2.8 * scale),
        Transform::identity(),
        None,
    );

    let normalized = remaining_percent
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0));
    if let Some(remaining) = normalized {
        let digit_color = if remaining <= 15.0 {
            (255, 137, 128, if stale { 175 } else { 255 })
        } else if remaining <= 40.0 {
            (255, 204, 112, if stale { 175 } else { 255 })
        } else {
            (246, 245, 237, if stale { 185 } else { 255 })
        };
        let label = format!("{remaining:.0}");
        let digit_count = label.len() as f32;
        let (digit_width, digit_height, gap, thickness) = if label.len() == 3 {
            (5.8, 16.0, 1.2, 1.55)
        } else {
            (9.2, 19.0, 2.0, 2.25)
        };
        let total_width = digit_width * digit_count + gap * (digit_count - 1.0);
        let start_x = (32.0 - total_width) / 2.0;
        let start_y = (32.0 - digit_height) / 2.0;
        for (index, character) in label.chars().enumerate() {
            if let Some(digit) = character.to_digit(10) {
                draw_segment_digit(
                    &mut pixmap,
                    digit as u8,
                    (start_x + index as f32 * (digit_width + gap)) * scale,
                    start_y * scale,
                    digit_width * scale,
                    digit_height * scale,
                    thickness * scale,
                    digit_color,
                );
            }
        }
    } else {
        for x in [8.5, 18.5] {
            fill_rounded_rect(
                &mut pixmap,
                x * scale,
                15.0 * scale,
                5.0 * scale,
                2.4 * scale,
                0.8 * scale,
                (246, 245, 237, 190),
            );
        }
    }

    if stale {
        let indicator =
            PathBuilder::from_circle(26.5 * scale, 6.8 * scale, 1.8 * scale).expect("stale dot");
        pixmap.fill_path(
            &indicator,
            &paint((255, 204, 112, 235), BlendMode::SourceOver),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
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
            TrayIconStyle::Template,
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
    fn renders_a_colored_icon_for_non_template_trays() {
        let image = render_tray_icon(
            TrayDialState {
                used_percent: Some(50.0),
                stale: false,
            },
            32,
            TrayIconStyle::Color,
        );
        assert_eq!(image.width(), 32);
        assert_eq!(image.height(), 32);
        assert!(image
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0 && (pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)));
    }

    #[test]
    fn renders_remaining_percentage_as_a_windows_tray_badge() {
        let low = render_windows_percentage_icon(Some(12.0), false, 32);
        let high = render_windows_percentage_icon(Some(97.0), false, 32);
        assert_eq!(high.width(), 32);
        assert_eq!(high.height(), 32);
        assert_ne!(low.rgba(), high.rgba());
        assert!(high
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel[1] > pixel[0] && pixel[3] > 0));
    }

    #[test]
    fn windows_badge_keeps_the_brand_hex_and_palette() {
        let image = render_windows_percentage_icon(Some(82.0), false, 32);
        let pixels = image.rgba().chunks_exact(4).collect::<Vec<_>>();
        assert_eq!(pixels[0][3], 0);
        assert!(pixels.iter().filter(|pixel| pixel[3] > 0).count() > 600);
        assert!(pixels
            .iter()
            .any(|pixel| { pixel[0] == 20 && pixel[1] == 35 && pixel[2] == 30 && pixel[3] > 0 }));
        assert!(pixels
            .iter()
            .any(|pixel| { pixel[1] > 190 && pixel[1] > pixel[0] && pixel[3] > 0 }));
        assert!(pixels
            .iter()
            .any(|pixel| { pixel[0] > 230 && pixel[1] > 230 && pixel[2] > 220 && pixel[3] > 0 }));
    }

    #[test]
    fn windows_percentage_badge_marks_stale_and_missing_values() {
        let fresh = render_windows_percentage_icon(Some(82.0), false, 32);
        let stale = render_windows_percentage_icon(Some(82.0), true, 32);
        let missing = render_windows_percentage_icon(None, false, 32);
        assert_ne!(fresh.rgba(), stale.rgba());
        assert_ne!(fresh.rgba(), missing.rgba());
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
