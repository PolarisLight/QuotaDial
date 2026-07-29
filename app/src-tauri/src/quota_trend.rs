use crate::storage::repository::RateObservation;
use chrono::{Local, TimeZone};
use std::collections::BTreeMap;

const MIN_SAMPLE_COUNT: usize = 3;
const MIN_SAMPLE_SPAN_SECONDS: i64 = 1_800;
const MAX_HISTORY_POINTS: usize = 64;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaHistoryPoint {
    pub observed_at: i64,
    pub remaining_percent: f64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuotaPaceStatus {
    Slow,
    Normal,
    Fast,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaPace {
    pub percent_per_day: f64,
    pub ideal_percent_per_day: f64,
    pub status: QuotaPaceStatus,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaTrend {
    pub history: Vec<QuotaHistoryPoint>,
    pub pace: Option<QuotaPace>,
}

pub fn build_trend(points: &[RateObservation], window_duration_mins: i64) -> QuotaTrend {
    let mut ordered = points.to_vec();
    ordered.sort_by_key(|point| point.observed_at);
    let segment_start = ordered
        .windows(2)
        .rposition(|pair| pair[1].used_percent < pair[0].used_percent)
        .map_or(0, |index| index + 1);
    let segment = &ordered[segment_start..];

    let history = downsample_history(segment);
    let pace = pace(segment, window_duration_mins);
    QuotaTrend { history, pace }
}

fn downsample_history(points: &[RateObservation]) -> Vec<QuotaHistoryPoint> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let last = points.last().expect("first point implies last point");
    let mut daily_latest = BTreeMap::new();
    for point in points {
        if let Some(observed) = Local.timestamp_opt(point.observed_at, 0).single() {
            daily_latest.insert(observed.date_naive(), point);
        }
    }

    let mut selected = Vec::with_capacity(daily_latest.len() + 2);
    selected.push(first);
    selected.extend(daily_latest.into_values());
    selected.push(last);
    selected.sort_by_key(|point| point.observed_at);
    selected.dedup_by_key(|point| point.observed_at);

    if selected.len() > MAX_HISTORY_POINTS {
        selected = (0..MAX_HISTORY_POINTS)
            .map(|slot| {
                let index = slot * (selected.len() - 1) / (MAX_HISTORY_POINTS - 1);
                selected[index]
            })
            .collect();
        selected.dedup_by_key(|point| point.observed_at);
    }

    selected
        .into_iter()
        .map(|point| QuotaHistoryPoint {
            observed_at: point.observed_at,
            remaining_percent: (100.0 - point.used_percent).clamp(0.0, 100.0),
        })
        .collect()
}

fn pace(points: &[RateObservation], window_duration_mins: i64) -> Option<QuotaPace> {
    let span_seconds = points.last()?.observed_at - points.first()?.observed_at;
    if points.len() < MIN_SAMPLE_COUNT || span_seconds < MIN_SAMPLE_SPAN_SECONDS {
        return None;
    }
    let percent_per_day = median(&pairwise_daily_slopes(points))?.max(0.0);
    let ideal_percent_per_day = ideal_pace(window_duration_mins)?;
    Some(QuotaPace {
        percent_per_day,
        ideal_percent_per_day,
        status: classify_pace(percent_per_day, window_duration_mins),
        sample_count: points.len(),
    })
}

fn pairwise_daily_slopes(points: &[RateObservation]) -> Vec<f64> {
    let mut slopes = Vec::new();
    for (index, left) in points.iter().enumerate() {
        for right in points.iter().skip(index + 1) {
            let days = (right.observed_at - left.observed_at) as f64 / 86_400.0;
            if days > 0.0 {
                slopes.push((right.used_percent - left.used_percent) / days);
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

fn ideal_pace(window_duration_mins: i64) -> Option<f64> {
    (window_duration_mins > 0)
        .then(|| 100.0 / (window_duration_mins as f64 / 1_440.0))
}

fn classify_pace(percent_per_day: f64, window_duration_mins: i64) -> QuotaPaceStatus {
    let Some(ideal) = ideal_pace(window_duration_mins) else {
        return QuotaPaceStatus::Normal;
    };
    if percent_per_day < ideal * 0.8 {
        QuotaPaceStatus::Slow
    } else if percent_per_day > ideal * 1.2 {
        QuotaPaceStatus::Fast
    } else {
        QuotaPaceStatus::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(points: &[(i64, f64)]) -> Vec<RateObservation> {
        points
            .iter()
            .map(|(observed_at, used_percent)| RateObservation {
                observed_at: *observed_at,
                limit_id: "codex".into(),
                window_kind: "primary".into(),
                used_percent: *used_percent,
                window_duration_mins: 10_080,
                resets_at: 2_000_000_000,
            })
            .collect()
    }

    #[test]
    fn converts_used_percent_to_downward_remaining_history() {
        let trend = build_trend(
            &samples(&[(0, 10.0), (86_400, 25.0), (172_800, 40.0)]),
            10_080,
        );
        assert_eq!(
            trend
                .history
                .iter()
                .map(|point| point.remaining_percent)
                .collect::<Vec<_>>(),
            vec![90.0, 75.0, 60.0]
        );
    }

    #[test]
    fn classifies_seven_day_pace_with_a_twenty_percent_band() {
        assert_eq!(classify_pace(10.0, 10_080), QuotaPaceStatus::Slow);
        assert_eq!(
            classify_pace(14.2857, 10_080),
            QuotaPaceStatus::Normal
        );
        assert_eq!(classify_pace(18.0, 10_080), QuotaPaceStatus::Fast);
    }

    #[test]
    fn keeps_first_last_and_daily_latest_points_under_the_limit() {
        let dense = samples(
            &(0..80)
                .map(|day| (1_700_000_000 + day * 86_400, day as f64))
                .collect::<Vec<_>>(),
        );
        let trend = build_trend(&dense, 10_080);
        assert!(trend.history.len() <= 64);
        assert_eq!(
            trend.history.first().unwrap().observed_at,
            dense.first().unwrap().observed_at
        );
        assert_eq!(
            trend.history.last().unwrap().observed_at,
            dense.last().unwrap().observed_at
        );
    }

    #[test]
    fn waits_for_three_samples_and_thirty_minutes_before_classifying() {
        let trend = build_trend(&samples(&[(0, 10.0), (3_600, 12.0)]), 10_080);
        assert_eq!(trend.pace, None);
    }
}
