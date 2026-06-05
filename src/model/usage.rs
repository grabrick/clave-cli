//! Учёт расхода моделей. Часть полей/методов задействуется с Фазы 3 (/cost, футер).
#![allow(dead_code)]

use crate::prelude::*;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) struct RunUsage {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) cache_read: u64,
    pub(crate) cost_usd: f64,
}

impl RunUsage {
    pub(crate) fn tokens(self) -> u64 {
        self.input + self.output + self.cache_read
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ProviderUsage {
    pub(crate) total: RunUsage,
    pub(crate) requests: u32,
}

pub(crate) struct SessionUsage {
    pub(crate) claude: ProviderUsage,
    pub(crate) codex: ProviderUsage,
    pub(crate) started_at: Instant,
}

impl SessionUsage {
    pub(crate) fn new() -> Self {
        Self {
            claude: ProviderUsage::default(),
            codex: ProviderUsage::default(),
            started_at: Instant::now(),
        }
    }

    pub(crate) fn record(&mut self, provider: &str, run: RunUsage) {
        let slot = if provider == "claude" {
            &mut self.claude
        } else {
            &mut self.codex
        };
        slot.total.input += run.input;
        slot.total.output += run.output;
        slot.total.cache_read += run.cache_read;
        slot.total.cost_usd += run.cost_usd;
        slot.requests += 1;
    }

    pub(crate) fn total_tokens(&self) -> u64 {
        self.claude.total.tokens() + self.codex.total.tokens()
    }

    pub(crate) fn total_cost_usd(&self) -> f64 {
        self.claude.total.cost_usd + self.codex.total.cost_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_accumulates_per_provider() {
        let mut s = SessionUsage::new();
        s.record(
            "claude",
            RunUsage {
                input: 100,
                output: 50,
                cache_read: 10,
                cost_usd: 0.02,
            },
        );
        s.record(
            "claude",
            RunUsage {
                input: 200,
                output: 30,
                cache_read: 0,
                cost_usd: 0.03,
            },
        );
        s.record(
            "codex",
            RunUsage {
                input: 80,
                output: 20,
                cache_read: 0,
                cost_usd: 0.0,
            },
        );

        assert_eq!(s.claude.requests, 2);
        assert_eq!(s.codex.requests, 1);
        assert_eq!(s.claude.total.input, 300);
        assert_eq!(s.claude.total.output, 80);
        assert!((s.claude.total.cost_usd - 0.05).abs() < 1e-9);
        assert_eq!(s.total_tokens(), 390 + 100);
        assert!((s.total_cost_usd() - 0.05).abs() < 1e-9);
    }
}
