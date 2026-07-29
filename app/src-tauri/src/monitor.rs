use crate::{
    app_server::{
        process::spawn_codex_app_server,
        protocol::{RpcNotification, RpcPeer},
    },
    domain::{
        account::{AccountUsageResult, RateLimitBucket, RateLimitsResult, RateWindow},
        dashboard::{AccountUsageView, DashboardSnapshot, QuotaView},
    },
    error::AppError,
    forecast,
    sessions::service::SessionService,
    storage::repository::AccountRepository,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, Mutex};

const REFRESH_INTERVAL_SECONDS: u64 = 60;
const STALE_AFTER_SECONDS: i64 = 120;

#[async_trait]
pub trait AccountSource: Send + Sync {
    async fn read_rate_limits(&self) -> Result<(RateLimitsResult, serde_json::Value), AppError>;
    async fn read_account_usage(&self)
        -> Result<(AccountUsageResult, serde_json::Value), AppError>;
    fn subscribe(&self) -> broadcast::Receiver<RpcNotification>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}

struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }
}

struct AppServerSession {
    peer: RpcPeer,
    _child: tokio::process::Child,
}

pub struct CodexAccountSource {
    session: Mutex<Option<AppServerSession>>,
    notifications: broadcast::Sender<RpcNotification>,
}

impl CodexAccountSource {
    pub fn new() -> Self {
        let (notifications, _) = broadcast::channel(64);
        Self {
            session: Mutex::new(None),
            notifications,
        }
    }

    async fn peer(&self) -> Result<RpcPeer, AppError> {
        let mut session = self.session.lock().await;
        if let Some(current) = session.as_ref() {
            return Ok(current.peer.clone());
        }

        let (peer, child) = spawn_codex_app_server().await?;
        let mut incoming = peer.subscribe();
        let outgoing = self.notifications.clone();
        tokio::spawn(async move {
            while let Ok(notification) = incoming.recv().await {
                let _ = outgoing.send(notification);
            }
        });
        *session = Some(AppServerSession {
            peer: peer.clone(),
            _child: child,
        });
        Ok(peer)
    }

    async fn request(&self, method: &str) -> Result<serde_json::Value, AppError> {
        const RETRY_DELAYS_SECONDS: [u64; 5] = [1, 2, 5, 10, 30];
        let mut last_error = None;

        for attempt in 0..=RETRY_DELAYS_SECONDS.len() {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(
                    RETRY_DELAYS_SECONDS[attempt - 1],
                ))
                .await;
            }

            match self.peer().await {
                Ok(peer) => match peer.request(method, None).await {
                    Ok(value) => return Ok(value),
                    Err(error) => {
                        last_error = Some(error);
                        *self.session.lock().await = None;
                    }
                },
                Err(error) => {
                    last_error = Some(error);
                    *self.session.lock().await = None;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::Unavailable("unable to connect to Codex app-server".into())
        }))
    }
}

impl Default for CodexAccountSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccountSource for CodexAccountSource {
    async fn read_rate_limits(&self) -> Result<(RateLimitsResult, serde_json::Value), AppError> {
        let raw = self.request("account/rateLimits/read").await?;
        let parsed = serde_json::from_value(raw.clone())?;
        Ok((parsed, raw))
    }

    async fn read_account_usage(
        &self,
    ) -> Result<(AccountUsageResult, serde_json::Value), AppError> {
        let raw = self.request("account/usage/read").await?;
        let parsed = serde_json::from_value(raw.clone())?;
        Ok((parsed, raw))
    }

    fn subscribe(&self) -> broadcast::Receiver<RpcNotification> {
        self.notifications.subscribe()
    }
}

pub struct AccountMonitor {
    source: Arc<dyn AccountSource>,
    repository: Arc<AccountRepository>,
    snapshot: watch::Sender<DashboardSnapshot>,
    refresh_lock: Mutex<()>,
    clock: Arc<dyn Clock>,
    sessions: Arc<SessionService>,
}

impl AccountMonitor {
    pub fn new(
        source: Arc<dyn AccountSource>,
        repository: Arc<AccountRepository>,
        sessions: Arc<SessionService>,
    ) -> Self {
        Self::new_with_clock_and_sessions(source, repository, Arc::new(SystemClock), sessions)
    }

    pub fn new_with_clock(
        source: Arc<dyn AccountSource>,
        repository: Arc<AccountRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let sessions = Arc::new(SessionService::new(
            repository.clone(),
            std::path::PathBuf::from("__codex_monitor_tests_no_sessions__"),
        ));
        Self::new_with_clock_and_sessions(source, repository, clock, sessions)
    }

    pub fn new_with_clock_and_sessions(
        source: Arc<dyn AccountSource>,
        repository: Arc<AccountRepository>,
        clock: Arc<dyn Clock>,
        sessions: Arc<SessionService>,
    ) -> Self {
        let (snapshot, _) = watch::channel(DashboardSnapshot::default());
        Self {
            source,
            repository,
            snapshot,
            refresh_lock: Mutex::new(()),
            clock,
            sessions,
        }
    }

    pub fn snapshot(&self) -> DashboardSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<DashboardSnapshot> {
        self.snapshot.subscribe()
    }

    pub fn apply_local_sessions(&self, local_sessions: crate::domain::session::LocalSessionView) {
        let mut snapshot = self.snapshot();
        snapshot.local_sessions = local_sessions;
        self.snapshot.send_replace(snapshot);
    }

    pub async fn refresh(&self) -> Result<DashboardSnapshot, AppError> {
        let _guard = self.refresh_lock.lock().await;
        let now = self.clock.now();
        let (rate_limits, raw_rate_limits) = match self.source.read_rate_limits().await {
            Ok(value) => value,
            Err(error) => {
                let mut snapshot = self.snapshot();
                snapshot.connection_error = Some(error.to_string());
                snapshot.is_stale =
                    snapshot.observed_at == 0 || now - snapshot.observed_at >= STALE_AFTER_SECONDS;
                self.snapshot.send_replace(snapshot);
                return Err(error);
            }
        };

        self.repository
            .insert_rate_limits(now, &rate_limits, &raw_rate_limits)?;
        let (primary_quota, other_quotas) = quota_views(&rate_limits);
        let forecast = primary_quota.as_ref().and_then(|quota| {
            self.repository
                .current_segment(&quota.limit_id, &quota.window_kind)
                .ok()
                .and_then(|points| forecast::forecast(&points, now, quota.resets_at))
        });

        let (account_usage, account_usage_error) = match self.source.read_account_usage().await {
            Ok((usage, raw_usage)) => {
                self.repository
                    .insert_account_usage(now, &usage, &raw_usage)?;
                (Some(AccountUsageView::from(usage)), None)
            }
            Err(error) => (None, Some(error.to_string())),
        };

        let snapshot = DashboardSnapshot {
            observed_at: now,
            is_stale: false,
            connection_error: None,
            account_usage_error,
            primary_quota,
            other_quotas,
            account_usage,
            forecast,
            local_sessions: self.sessions.snapshot(),
        };
        self.snapshot.send_replace(snapshot.clone());
        Ok(snapshot)
    }

    pub async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        let mut notifications = self.source.subscribe();
        let start =
            tokio::time::Instant::now() + std::time::Duration::from_secs(REFRESH_INTERVAL_SECONDS);
        let mut interval = tokio::time::interval_at(
            start,
            std::time::Duration::from_secs(REFRESH_INTERVAL_SECONDS),
        );
        let _ = self.refresh().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let _ = self.refresh().await;
                }
                notification = notifications.recv() => {
                    match notification {
                        Ok(item) if item.method == "account/rateLimits/updated" => {
                            let _ = self.refresh().await;
                        }
                        Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    }
}

fn quota_views(rate_limits: &RateLimitsResult) -> (Option<QuotaView>, Vec<QuotaView>) {
    let mut views = Vec::new();
    for (limit_id, bucket) in rate_limits.buckets() {
        if let Some(window) = bucket.primary.as_ref() {
            views.push(quota_view(&limit_id, &bucket, "primary", window));
        }
        if let Some(window) = bucket.secondary.as_ref() {
            views.push(quota_view(&limit_id, &bucket, "secondary", window));
        }
    }

    let primary_index = views
        .iter()
        .position(|quota| quota.limit_id == "codex" && quota.window_kind == "primary")
        .or_else(|| {
            views
                .iter()
                .position(|quota| quota.window_kind == "primary")
        });
    let primary = primary_index.map(|index| views.remove(index));
    (primary, views)
}

fn quota_view(
    limit_id: &str,
    bucket: &RateLimitBucket,
    window_kind: &str,
    window: &RateWindow,
) -> QuotaView {
    let label = bucket
        .limit_name
        .clone()
        .unwrap_or_else(|| match window.window_duration_mins {
            10_080 => "7 日额度".into(),
            300 => "5 小时额度".into(),
            _ => limit_id.to_owned(),
        });
    QuotaView {
        limit_id: limit_id.to_owned(),
        label,
        window_kind: window_kind.to_owned(),
        used_percent: window.used_percent,
        remaining_percent: (100.0 - window.used_percent).clamp(0.0, 100.0),
        window_duration_mins: window.window_duration_mins,
        resets_at: window.resets_at,
        plan_type: bucket.plan_type.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_server::protocol::RpcNotification,
        domain::account::{AccountUsageResult, RateLimitsResult},
        error::AppError,
        storage::repository::AccountRepository,
    };
    use async_trait::async_trait;
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };
    use tokio::sync::{broadcast, watch};

    struct FakeClock(AtomicI64);

    impl FakeClock {
        fn set(&self, value: i64) {
            self.0.store(value, Ordering::SeqCst);
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> i64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    struct FakeSource {
        rate_reads: AtomicUsize,
        usage_reads: AtomicUsize,
        fail_rate: AtomicBool,
        fail_usage: AtomicBool,
        notifications: broadcast::Sender<RpcNotification>,
    }

    impl FakeSource {
        fn new() -> Self {
            let (notifications, _) = broadcast::channel(8);
            Self {
                rate_reads: AtomicUsize::new(0),
                usage_reads: AtomicUsize::new(0),
                fail_rate: AtomicBool::new(false),
                fail_usage: AtomicBool::new(false),
                notifications,
            }
        }

        fn notify_rate_limit_update(&self) {
            let _ = self.notifications.send(RpcNotification {
                method: "account/rateLimits/updated".into(),
                params: serde_json::json!({}),
            });
        }
    }

    #[async_trait]
    impl AccountSource for FakeSource {
        async fn read_rate_limits(
            &self,
        ) -> Result<(RateLimitsResult, serde_json::Value), AppError> {
            self.rate_reads.fetch_add(1, Ordering::SeqCst);
            if self.fail_rate.load(Ordering::SeqCst) {
                return Err(AppError::Disconnected);
            }
            let raw = serde_json::json!({
                "rateLimitsByLimitId": {
                    "codex": {
                        "limitId": "codex",
                        "limitName": null,
                        "primary": {
                            "usedPercent": 18,
                            "windowDurationMins": 10080,
                            "resetsAt": 200000
                        },
                        "secondary": null,
                        "rateLimitReachedType": null
                    }
                }
            });
            Ok((serde_json::from_value(raw.clone()).unwrap(), raw))
        }

        async fn read_account_usage(
            &self,
        ) -> Result<(AccountUsageResult, serde_json::Value), AppError> {
            self.usage_reads.fetch_add(1, Ordering::SeqCst);
            if self.fail_usage.load(Ordering::SeqCst) {
                return Err(AppError::Unavailable("usage unavailable".into()));
            }
            let raw = serde_json::json!({
                "summary": { "lifetimeTokens": 9000, "peakDailyTokens": 4000 },
                "dailyUsageBuckets": [
                    { "startDate": "2026-07-29", "tokens": 3000 }
                ]
            });
            Ok((serde_json::from_value(raw.clone()).unwrap(), raw))
        }

        fn subscribe(&self) -> broadcast::Receiver<RpcNotification> {
            self.notifications.subscribe()
        }
    }

    fn monitor(source: Arc<FakeSource>, clock: Arc<FakeClock>) -> Arc<AccountMonitor> {
        Arc::new(AccountMonitor::new_with_clock(
            source,
            Arc::new(AccountRepository::open_in_memory().unwrap()),
            clock,
        ))
    }

    async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
        for _ in 0..100 {
            if counter.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("counter did not reach {expected}");
    }

    #[tokio::test]
    async fn startup_refresh_reads_quota_and_account_usage() {
        let source = Arc::new(FakeSource::new());
        let clock = Arc::new(FakeClock(AtomicI64::new(1_000)));
        let monitor = monitor(source.clone(), clock);

        let snapshot = monitor.refresh().await.unwrap();

        assert_eq!(source.rate_reads.load(Ordering::SeqCst), 1);
        assert_eq!(source.usage_reads.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot.primary_quota.unwrap().remaining_percent, 82.0);
        assert_eq!(
            snapshot.account_usage.unwrap().daily_usage_buckets[0].tokens,
            3_000
        );
    }

    #[tokio::test]
    async fn rate_limit_notification_triggers_immediate_refresh() {
        let source = Arc::new(FakeSource::new());
        let clock = Arc::new(FakeClock(AtomicI64::new(1_000)));
        let monitor = monitor(source.clone(), clock);
        let (shutdown, receiver) = watch::channel(false);
        let task = tokio::spawn(monitor.run(receiver));
        wait_for_count(&source.rate_reads, 1).await;

        source.notify_rate_limit_update();
        wait_for_count(&source.rate_reads, 2).await;

        shutdown.send(true).unwrap();
        task.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn periodic_loop_refreshes_after_sixty_seconds() {
        let source = Arc::new(FakeSource::new());
        let clock = Arc::new(FakeClock(AtomicI64::new(1_000)));
        let monitor = monitor(source.clone(), clock);
        let (shutdown, receiver) = watch::channel(false);
        let task = tokio::spawn(monitor.run(receiver));
        wait_for_count(&source.rate_reads, 1).await;

        tokio::time::advance(Duration::from_secs(60)).await;
        wait_for_count(&source.rate_reads, 2).await;

        shutdown.send(true).unwrap();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn failed_usage_read_keeps_quota_data() {
        let source = Arc::new(FakeSource::new());
        source.fail_usage.store(true, Ordering::SeqCst);
        let clock = Arc::new(FakeClock(AtomicI64::new(1_000)));
        let monitor = monitor(source, clock);

        let snapshot = monitor.refresh().await.unwrap();

        assert!(snapshot.primary_quota.is_some());
        assert!(snapshot.account_usage.is_none());
        assert!(snapshot.account_usage_error.is_some());
        assert!(!snapshot.is_stale);
    }

    #[tokio::test]
    async fn disconnected_source_retains_values_and_becomes_stale() {
        let source = Arc::new(FakeSource::new());
        let clock = Arc::new(FakeClock(AtomicI64::new(1_000)));
        let monitor = monitor(source.clone(), clock.clone());
        monitor.refresh().await.unwrap();

        source.fail_rate.store(true, Ordering::SeqCst);
        clock.set(1_121);
        let error = monitor.refresh().await.unwrap_err();
        let snapshot = monitor.snapshot();

        assert!(matches!(error, AppError::Disconnected));
        assert_eq!(snapshot.primary_quota.unwrap().remaining_percent, 82.0);
        assert!(snapshot.is_stale);
        assert!(snapshot.connection_error.is_some());
    }
}
