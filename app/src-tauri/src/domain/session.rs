#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
}

impl TokenBreakdown {
    pub fn total(&self) -> i64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub title: String,
    pub project_path: Option<String>,
    pub last_active_at: i64,
    pub primary_model: Option<String>,
    pub tokens: TokenBreakdown,
    pub monthly_tokens: TokenBreakdown,
    pub equivalent_cost_usd: Option<f64>,
    pub monthly_equivalent_cost_usd: Option<f64>,
    pub priced_tokens: i64,
    pub unpriced_tokens: i64,
    pub monthly_priced_tokens: i64,
    pub monthly_unpriced_tokens: i64,
    pub child_session_count: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDiagnostics {
    pub scanned_files: i64,
    pub skipped_lines: i64,
    pub last_imported_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyUsageSummary {
    pub period_start: i64,
    pub period_end: i64,
    pub tokens: TokenBreakdown,
    pub equivalent_cost_usd: Option<f64>,
    pub priced_tokens: i64,
    pub unpriced_tokens: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionView {
    pub sessions: Vec<SessionSummary>,
    pub monthly_summary: MonthlyUsageSummary,
    pub diagnostics: SessionDiagnostics,
}
