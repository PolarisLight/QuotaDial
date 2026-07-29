use crate::domain::session::TokenBreakdown;

#[derive(Debug, Clone, Copy)]
struct PriceEntry {
    model: &'static str,
    effective_from: i64,
    input_per_million: f64,
    cached_input_per_million: f64,
    output_per_million: f64,
}

pub struct PriceCatalog {
    entries: &'static [PriceEntry],
}

impl PriceCatalog {
    pub fn built_in() -> Self {
        Self {
            entries: BUILT_IN_PRICES,
        }
    }

    pub fn estimate(&self, model: &str, occurred_at: i64, tokens: &TokenBreakdown) -> Option<f64> {
        let entry = self
            .entries
            .iter()
            .filter(|entry| entry.model == model && entry.effective_from <= occurred_at)
            .max_by_key(|entry| entry.effective_from)?;
        let uncached_input = (tokens.input_tokens - tokens.cached_input_tokens).max(0);
        Some(
            uncached_input as f64 / 1_000_000.0 * entry.input_per_million
                + tokens.cached_input_tokens as f64 / 1_000_000.0 * entry.cached_input_per_million
                + tokens.output_tokens as f64 / 1_000_000.0 * entry.output_per_million,
        )
    }
}

// Catalog version: 2026-07-29.
// Sources:
// https://developers.openai.com/api/docs/pricing
// https://developers.openai.com/api/docs/models/gpt-5.6-sol
// https://developers.openai.com/api/docs/models/gpt-5.6-terra
// https://developers.openai.com/api/docs/models/gpt-5.6-luna
// https://developers.openai.com/api/docs/models/gpt-5.4
// https://developers.openai.com/api/docs/models/gpt-5.3-codex
// https://developers.openai.com/api/docs/models/gpt-5.2
// https://developers.openai.com/api/docs/models/gpt-5-codex
const BUILT_IN_PRICES: &[PriceEntry] = &[
    PriceEntry {
        model: "gpt-5.5",
        effective_from: 0,
        input_per_million: 5.0,
        cached_input_per_million: 0.5,
        output_per_million: 30.0,
    },
    PriceEntry {
        model: "gpt-5.6-sol",
        effective_from: 0,
        input_per_million: 5.0,
        cached_input_per_million: 0.5,
        output_per_million: 30.0,
    },
    PriceEntry {
        model: "gpt-5.6",
        effective_from: 0,
        input_per_million: 5.0,
        cached_input_per_million: 0.5,
        output_per_million: 30.0,
    },
    PriceEntry {
        model: "gpt-5.6-terra",
        effective_from: 0,
        input_per_million: 2.5,
        cached_input_per_million: 0.25,
        output_per_million: 15.0,
    },
    PriceEntry {
        model: "gpt-5.6-luna",
        effective_from: 0,
        input_per_million: 1.0,
        cached_input_per_million: 0.1,
        output_per_million: 6.0,
    },
    PriceEntry {
        model: "gpt-5.4",
        effective_from: 0,
        input_per_million: 2.5,
        cached_input_per_million: 0.25,
        output_per_million: 15.0,
    },
    PriceEntry {
        model: "gpt-5.3-codex",
        effective_from: 0,
        input_per_million: 1.75,
        cached_input_per_million: 0.175,
        output_per_million: 14.0,
    },
    PriceEntry {
        model: "gpt-5.2",
        effective_from: 0,
        input_per_million: 1.75,
        cached_input_per_million: 0.175,
        output_per_million: 14.0,
    },
    PriceEntry {
        model: "gpt-5.2-codex",
        effective_from: 0,
        input_per_million: 1.75,
        cached_input_per_million: 0.175,
        output_per_million: 14.0,
    },
    PriceEntry {
        model: "gpt-5-codex",
        effective_from: 0,
        input_per_million: 1.25,
        cached_input_per_million: 0.125,
        output_per_million: 10.0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::TokenBreakdown;

    #[test]
    fn prices_input_cached_input_and_output_separately() {
        let prices = PriceCatalog::built_in();
        let tokens = TokenBreakdown {
            input_tokens: 1_000_000,
            cached_input_tokens: 500_000,
            output_tokens: 200_000,
            reasoning_output_tokens: 50_000,
        };
        let cost = prices
            .estimate("gpt-5.6-sol", 1_785_283_200, &tokens)
            .unwrap();
        assert_eq!(cost, 8.75);
    }

    #[test]
    fn prices_gpt_5_5_at_the_standard_short_context_rate() {
        let tokens = TokenBreakdown {
            input_tokens: 1_000_000,
            cached_input_tokens: 500_000,
            output_tokens: 200_000,
            reasoning_output_tokens: 0,
        };
        let cost = PriceCatalog::built_in()
            .estimate("gpt-5.5", 1_785_283_200, &tokens)
            .unwrap();
        assert_eq!(cost, 8.75);
    }

    #[test]
    fn unknown_models_return_no_cost() {
        assert_eq!(
            PriceCatalog::built_in().estimate(
                "future-unknown-model",
                1_785_283_200,
                &TokenBreakdown::default()
            ),
            None
        );
    }
}
