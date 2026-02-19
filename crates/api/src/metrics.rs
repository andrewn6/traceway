//! Prometheus metrics for llm-fs cloud deployment.
//!
//! This module provides instrumentation for monitoring the health and performance
//! of the llm-fs service in production.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Metrics registry for the application
#[derive(Debug, Default)]
pub struct Metrics {
    // Counters
    pub span_writes_total: AtomicU64,
    pub span_reads_total: AtomicU64,
    pub trace_writes_total: AtomicU64,
    pub trace_reads_total: AtomicU64,
    pub sse_connections_total: AtomicU64,
    pub api_requests_total: AtomicU64,
    pub api_errors_total: AtomicU64,

    // Gauges (current values)
    pub sse_connections_active: AtomicU64,
    pub span_count: AtomicU64,
    pub trace_count: AtomicU64,

    // Histogram buckets for latency tracking
    pub span_write_latency_sum_us: AtomicU64,
    pub span_write_latency_count: AtomicU64,
    pub api_latency_sum_us: AtomicU64,
    pub api_latency_count: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Record a span write operation
    pub fn record_span_write(&self, duration: std::time::Duration) {
        self.span_writes_total.fetch_add(1, Ordering::Relaxed);
        self.span_write_latency_sum_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.span_write_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a span read operation
    pub fn record_span_read(&self) {
        self.span_reads_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a trace write operation
    pub fn record_trace_write(&self) {
        self.trace_writes_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an API request
    pub fn record_api_request(&self, duration: std::time::Duration, is_error: bool) {
        self.api_requests_total.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.api_errors_total.fetch_add(1, Ordering::Relaxed);
        }
        self.api_latency_sum_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.api_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment SSE connection count
    pub fn sse_connect(&self) {
        self.sse_connections_total.fetch_add(1, Ordering::Relaxed);
        self.sse_connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement SSE connection count
    pub fn sse_disconnect(&self) {
        self.sse_connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Update span/trace counts
    pub fn update_counts(&self, spans: u64, traces: u64) {
        self.span_count.store(spans, Ordering::Relaxed);
        self.trace_count.store(traces, Ordering::Relaxed);
    }

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Counters
        output.push_str("# HELP llmfs_span_writes_total Total number of span write operations\n");
        output.push_str("# TYPE llmfs_span_writes_total counter\n");
        output.push_str(&format!(
            "llmfs_span_writes_total {}\n",
            self.span_writes_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_span_reads_total Total number of span read operations\n");
        output.push_str("# TYPE llmfs_span_reads_total counter\n");
        output.push_str(&format!(
            "llmfs_span_reads_total {}\n",
            self.span_reads_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_trace_writes_total Total number of trace write operations\n");
        output.push_str("# TYPE llmfs_trace_writes_total counter\n");
        output.push_str(&format!(
            "llmfs_trace_writes_total {}\n",
            self.trace_writes_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_api_requests_total Total number of API requests\n");
        output.push_str("# TYPE llmfs_api_requests_total counter\n");
        output.push_str(&format!(
            "llmfs_api_requests_total {}\n",
            self.api_requests_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_api_errors_total Total number of API errors\n");
        output.push_str("# TYPE llmfs_api_errors_total counter\n");
        output.push_str(&format!(
            "llmfs_api_errors_total {}\n",
            self.api_errors_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_sse_connections_total Total SSE connections (cumulative)\n");
        output.push_str("# TYPE llmfs_sse_connections_total counter\n");
        output.push_str(&format!(
            "llmfs_sse_connections_total {}\n",
            self.sse_connections_total.load(Ordering::Relaxed)
        ));

        // Gauges
        output.push_str("# HELP llmfs_sse_connections_active Current active SSE connections\n");
        output.push_str("# TYPE llmfs_sse_connections_active gauge\n");
        output.push_str(&format!(
            "llmfs_sse_connections_active {}\n",
            self.sse_connections_active.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_span_count Current number of spans in storage\n");
        output.push_str("# TYPE llmfs_span_count gauge\n");
        output.push_str(&format!(
            "llmfs_span_count {}\n",
            self.span_count.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP llmfs_trace_count Current number of traces in storage\n");
        output.push_str("# TYPE llmfs_trace_count gauge\n");
        output.push_str(&format!(
            "llmfs_trace_count {}\n",
            self.trace_count.load(Ordering::Relaxed)
        ));

        // Latency summaries
        let span_write_count = self.span_write_latency_count.load(Ordering::Relaxed);
        let span_write_sum = self.span_write_latency_sum_us.load(Ordering::Relaxed);
        let span_write_avg = if span_write_count > 0 {
            span_write_sum as f64 / span_write_count as f64 / 1000.0 // Convert to ms
        } else {
            0.0
        };

        output.push_str(
            "# HELP llmfs_span_write_latency_ms Average span write latency in milliseconds\n",
        );
        output.push_str("# TYPE llmfs_span_write_latency_ms gauge\n");
        output.push_str(&format!(
            "llmfs_span_write_latency_ms {:.3}\n",
            span_write_avg
        ));

        let api_count = self.api_latency_count.load(Ordering::Relaxed);
        let api_sum = self.api_latency_sum_us.load(Ordering::Relaxed);
        let api_avg = if api_count > 0 {
            api_sum as f64 / api_count as f64 / 1000.0
        } else {
            0.0
        };

        output.push_str("# HELP llmfs_api_latency_ms Average API latency in milliseconds\n");
        output.push_str("# TYPE llmfs_api_latency_ms gauge\n");
        output.push_str(&format!("llmfs_api_latency_ms {:.3}\n", api_avg));

        // Error rate
        let total_requests = self.api_requests_total.load(Ordering::Relaxed);
        let total_errors = self.api_errors_total.load(Ordering::Relaxed);
        let error_rate = if total_requests > 0 {
            total_errors as f64 / total_requests as f64
        } else {
            0.0
        };

        output.push_str("# HELP llmfs_error_rate Current API error rate (0-1)\n");
        output.push_str("# TYPE llmfs_error_rate gauge\n");
        output.push_str(&format!("llmfs_error_rate {:.6}\n", error_rate));

        output
    }
}

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}
