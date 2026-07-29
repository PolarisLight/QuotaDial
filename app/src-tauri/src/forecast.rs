use crate::storage::repository::RateObservation;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ForecastStatus {
    DepletesBeforeReset,
    SurvivesWindow,
    NoMeasurableBurn,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ForecastConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExhaustionForecast {
    pub status: ForecastStatus,
    pub rate_percent_per_hour: f64,
    pub exhausts_at: Option<i64>,
    pub confidence: ForecastConfidence,
    pub sample_count: usize,
    pub span_seconds: i64,
}

pub fn forecast(
    points: &[RateObservation],
    now: i64,
    resets_at: i64,
) -> Option<ExhaustionForecast> {
    if points.iter().any(|point| point.resets_at != resets_at) || resets_at <= now {
        return None;
    }

    let mut ordered = points.to_vec();
    ordered.sort_by_key(|point| point.observed_at);
    let first = ordered.first()?;
    if ordered.iter().any(|point| {
        point.limit_id != first.limit_id
            || point.window_kind != first.window_kind
            || point.window_duration_mins != first.window_duration_mins
    }) {
        return None;
    }

    let segment_start = ordered
        .windows(2)
        .rposition(|pair| pair[1].used_percent < pair[0].used_percent)
        .map_or(0, |index| index + 1);
    let segment = &ordered[segment_start..];
    let span_seconds = segment.last()?.observed_at - segment.first()?.observed_at;
    if segment.len() < 3 || span_seconds < 1_800 {
        return None;
    }

    let slopes = pairwise_slopes(segment);
    let rate = median(&slopes)?;
    let confidence = confidence(&slopes, rate, segment.len(), span_seconds);
    if rate <= 0.05 {
        return Some(ExhaustionForecast {
            status: ForecastStatus::NoMeasurableBurn,
            rate_percent_per_hour: rate.max(0.0),
            exhausts_at: None,
            confidence,
            sample_count: segment.len(),
            span_seconds,
        });
    }

    let latest = segment.last()?;
    let hours_remaining = (100.0 - latest.used_percent).max(0.0) / rate;
    let projected = now.saturating_add((hours_remaining * 3_600.0).round() as i64);
    let (status, exhausts_at) = if projected < resets_at {
        (ForecastStatus::DepletesBeforeReset, Some(projected))
    } else {
        (ForecastStatus::SurvivesWindow, None)
    };

    Some(ExhaustionForecast {
        status,
        rate_percent_per_hour: rate,
        exhausts_at,
        confidence,
        sample_count: segment.len(),
        span_seconds,
    })
}

fn pairwise_slopes(points: &[RateObservation]) -> Vec<f64> {
    let mut slopes = Vec::new();
    for (index, left) in points.iter().enumerate() {
        for right in points.iter().skip(index + 1) {
            let hours = (right.observed_at - left.observed_at) as f64 / 3_600.0;
            if hours > 0.0 {
                slopes.push((right.used_percent - left.used_percent) / hours);
            }
        }
    }
    slopes
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    } else {
        Some(sorted[middle])
    }
}

fn confidence(
    slopes: &[f64],
    rate: f64,
    sample_count: usize,
    span_seconds: i64,
) -> ForecastConfidence {
    let deviations = slopes
        .iter()
        .map(|slope| (slope - rate).abs())
        .collect::<Vec<_>>();
    let relative_deviation = median(&deviations)
        .map(|deviation| deviation / rate.abs().max(0.05))
        .unwrap_or(f64::INFINITY);

    if sample_count >= 8 && span_seconds >= 14_400 && relative_deviation <= 0.25 {
        ForecastConfidence::High
    } else if sample_count >= 4 && span_seconds >= 3_600 && relative_deviation <= 0.60 {
        ForecastConfidence::Medium
    } else {
        ForecastConfidence::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(resets_at: i64, points: &[(i64, f64)]) -> Vec<RateObservation> {
        points
            .iter()
            .map(|(observed_at, used_percent)| RateObservation {
                observed_at: *observed_at,
                limit_id: "codex".into(),
                window_kind: "primary".into(),
                used_percent: *used_percent,
                window_duration_mins: 10_080,
                resets_at,
            })
            .collect()
    }

    #[test]
    fn predicts_exhaustion_before_reset_from_stable_samples() {
        let points = samples(
            200_000,
            &[(0, 10.0), (3_600, 14.0), (7_200, 18.0), (10_800, 22.0)],
        );
        let result = forecast(&points, 10_800, 200_000).unwrap();
        assert_eq!(result.status, ForecastStatus::DepletesBeforeReset);
        assert!((result.rate_percent_per_hour - 4.0).abs() < 0.01);
        assert_eq!(result.exhausts_at, Some(81_000));
    }

    #[test]
    fn reports_not_expected_to_deplete_before_reset() {
        let points = samples(20_000, &[(0, 10.0), (7_200, 11.0), (14_400, 12.0)]);
        let result = forecast(&points, 14_400, 20_000).unwrap();
        assert_eq!(result.status, ForecastStatus::SurvivesWindow);
        assert_eq!(result.exhausts_at, None);
    }

    #[test]
    fn rejects_samples_across_reset_boundaries() {
        let mut points = samples(20_000, &[(0, 10.0), (3_600, 12.0), (7_200, 14.0)]);
        points[0].resets_at = 19_000;
        assert_eq!(forecast(&points, 7_200, 20_000), None);
    }

    #[test]
    fn requires_three_samples_spanning_thirty_minutes() {
        let too_few = samples(20_000, &[(0, 10.0), (3_600, 12.0)]);
        let too_short = samples(20_000, &[(0, 10.0), (600, 11.0), (1_200, 12.0)]);
        assert_eq!(forecast(&too_few, 3_600, 20_000), None);
        assert_eq!(forecast(&too_short, 1_200, 20_000), None);
    }

    #[test]
    fn a_falling_used_percent_starts_a_new_segment() {
        let points = samples(
            30_000,
            &[
                (0, 40.0),
                (3_600, 50.0),
                (7_200, 2.0),
                (10_800, 4.0),
                (14_400, 6.0),
            ],
        );
        let result = forecast(&points, 14_400, 30_000).unwrap();
        assert_eq!(result.sample_count, 3);
        assert!((result.rate_percent_per_hour - 2.0).abs() < 0.01);
    }

    #[test]
    fn reports_no_measurable_burn_for_flat_usage() {
        let points = samples(30_000, &[(0, 10.0), (3_600, 10.01), (7_200, 10.02)]);
        let result = forecast(&points, 7_200, 30_000).unwrap();
        assert_eq!(result.status, ForecastStatus::NoMeasurableBurn);
        assert_eq!(result.exhausts_at, None);
    }
}
