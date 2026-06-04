#![forbid(unsafe_code)]

//! Failure analysis with ternary classification.
//!
//! Provides failure mode classification (Avoid/Negligible/Critical),
//! FMEA-style risk analysis, fault trees with ternary gates, reliability
//! modeling, and MTBF estimation with ternary confidence.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Ternary Classification
// ---------------------------------------------------------------------------

/// Ternary failure mode severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureMode {
    /// Should be avoided at all costs.
    Avoid,
    /// Impact is negligible; can be tolerated.
    Negligible,
    /// Critical; must be addressed.
    Critical,
}

impl FailureMode {
    pub fn severity_score(&self) -> f64 {
        match self {
            FailureMode::Avoid => 1.0,
            FailureMode::Negligible => 0.0,
            FailureMode::Critical => 0.667,
        }
    }

    pub fn from_severity(score: f64) -> Self {
        if score < 0.33 {
            FailureMode::Negligible
        } else if score < 0.75 {
            FailureMode::Critical
        } else {
            FailureMode::Avoid
        }
    }
}

/// Ternary confidence level for estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn weight(&self) -> f64 {
        match self {
            Confidence::Low => 0.33,
            Confidence::Medium => 0.67,
            Confidence::High => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// FMEA Analysis
// ---------------------------------------------------------------------------

/// A single failure mode entry for FMEA analysis.
#[derive(Debug, Clone)]
pub struct FmeaEntry {
    pub name: String,
    pub severity: u8,       // 1-10
    pub occurrence: u8,     // 1-10
    pub detection: u8,      // 1-10
    pub classification: FailureMode,
}

impl FmeaEntry {
    pub fn new(name: &str, severity: u8, occurrence: u8, detection: u8) -> Self {
        let severity = severity.clamp(1, 10);
        let occurrence = occurrence.clamp(1, 10);
        let detection = detection.clamp(1, 10);
        let rpn = severity as u32 * occurrence as u32 * detection as u32;
        let classification = if rpn >= 200 {
            FailureMode::Avoid
        } else if rpn <= 50 {
            FailureMode::Negligible
        } else {
            FailureMode::Critical
        };
        Self {
            name: name.to_string(),
            severity,
            occurrence,
            detection,
            classification,
        }
    }

    /// Risk Priority Number.
    pub fn rpn(&self) -> u32 {
        self.severity as u32 * self.occurrence as u32 * self.detection as u32
    }

    /// Ternary risk assessment.
    pub fn ternary_risk(&self) -> FailureMode {
        self.classification
    }
}

/// FMEA analysis engine.
#[derive(Debug, Clone)]
pub struct FmeaAnalysis {
    pub entries: Vec<FmeaEntry>,
}

impl FmeaAnalysis {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, entry: FmeaEntry) {
        self.entries.push(entry);
    }

    /// Sort entries by RPN descending.
    pub fn sorted_by_risk(&self) -> Vec<&FmeaEntry> {
        let mut v: Vec<_> = self.entries.iter().collect();
        v.sort_by(|a, b| b.rpn().cmp(&a.rpn()));
        v
    }

    /// Count entries by classification.
    pub fn count_by_classification(&self) -> HashMap<FailureMode, usize> {
        let mut counts = HashMap::new();
        for e in &self.entries {
            *counts.entry(e.classification).or_insert(0) += 1;
        }
        counts
    }

    /// Average RPN across all entries.
    pub fn avg_rpn(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.entries.iter().map(|e| e.rpn() as f64).sum::<f64>() / self.entries.len() as f64
    }

    /// Maximum RPN.
    pub fn max_rpn(&self) -> u32 {
        self.entries.iter().map(|e| e.rpn()).max().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Fault Tree with Ternary Gates
// ---------------------------------------------------------------------------

/// A ternary gate in a fault tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TernaryGate {
    /// AND gate: output is worst of inputs (max severity).
    TernaryAnd,
    /// OR gate: output is best of inputs (min severity) — any input triggers.
    TernaryOr,
    /// NOT gate: inverts Avoid↔Negligible, keeps Critical.
    TernaryNot,
    /// K-of-N gate: at least K inputs must be Avoid/Critical for output to be Critical.
    KofN { k: usize },
}

/// A node in the fault tree.
#[derive(Debug, Clone)]
pub enum FaultNode {
    /// A basic event with a known failure mode.
    Basic { name: String, mode: FailureMode },
    /// A gate with children.
    Gate {
        name: String,
        gate: TernaryGate,
        children: Vec<FaultNode>,
    },
}

impl FaultNode {
    /// Evaluate the fault tree, returning the resulting failure mode.
    pub fn evaluate(&self) -> FailureMode {
        match self {
            FaultNode::Basic { mode, .. } => *mode,
            FaultNode::Gate { gate, children, .. } => {
                let child_modes: Vec<FailureMode> = children.iter().map(|c| c.evaluate()).collect();
                match gate {
                    TernaryGate::TernaryAnd => {
                        // AND: worst of all (max severity)
                        let mut worst = FailureMode::Negligible;
                        for m in &child_modes {
                            if m.severity_score() > worst.severity_score() {
                                worst = *m;
                            }
                        }
                        worst
                    }
                    TernaryGate::TernaryOr => {
                        // OR: any non-negligible triggers (min severity that's non-negligible)
                        let mut best = FailureMode::Negligible;
                        for m in &child_modes {
                            if m.severity_score() > best.severity_score() {
                                best = *m;
                            }
                        }
                        best
                    }
                    TernaryGate::TernaryNot => {
                        // NOT: invert
                        if let Some(first) = child_modes.first() {
                            match first {
                                FailureMode::Avoid => FailureMode::Negligible,
                                FailureMode::Negligible => FailureMode::Avoid,
                                FailureMode::Critical => FailureMode::Critical,
                            }
                        } else {
                            FailureMode::Negligible
                        }
                    }
                    TernaryGate::KofN { k } => {
                        let active = child_modes.iter()
                            .filter(|m| **m != FailureMode::Negligible)
                            .count();
                        if active >= *k {
                            FailureMode::Critical
                        } else {
                            FailureMode::Negligible
                        }
                    }
                }
            }
        }
    }

    /// Collect all basic events.
    pub fn basic_events(&self) -> Vec<(&str, FailureMode)> {
        let mut result = Vec::new();
        self.collect_basics(&mut result);
        result
    }

    fn collect_basics<'a>(&'a self, result: &mut Vec<(&'a str, FailureMode)>) {
        match self {
            FaultNode::Basic { name, mode } => result.push((name.as_str(), *mode)),
            FaultNode::Gate { children, .. } => {
                for c in children {
                    c.collect_basics(result);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reliability Modeling
// ---------------------------------------------------------------------------

/// Component reliability model.
#[derive(Debug, Clone)]
pub struct ReliabilityModel {
    pub name: String,
    /// Failure rate (failures per unit time).
    pub failure_rate: f64,
    /// Confidence in the estimate.
    pub confidence: Confidence,
}

impl ReliabilityModel {
    pub fn new(name: &str, failure_rate: f64, confidence: Confidence) -> Self {
        Self {
            name: name.to_string(),
            failure_rate,
            confidence,
        }
    }

    /// Reliability at time t: R(t) = e^(-λt)
    pub fn reliability_at(&self, t: f64) -> f64 {
        (-self.failure_rate * t).exp()
    }

    /// Mean Time Between Failures = 1/λ
    pub fn mtbf(&self) -> f64 {
        if self.failure_rate > 0.0 {
            1.0 / self.failure_rate
        } else {
            f64::INFINITY
        }
    }

    /// MTBF adjusted by confidence.
    pub fn adjusted_mtbf(&self) -> f64 {
        let base = self.mtbf();
        // Lower confidence → wider uncertainty → we report conservative (shorter) MTBF
        base * self.confidence.weight()
    }

    /// MTBF with ternary confidence bounds: (low, expected, high).
    pub fn mtbf_bounds(&self) -> (f64, f64, f64) {
        let base = self.mtbf();
        match self.confidence {
            Confidence::High => (base * 0.9, base, base * 1.1),
            Confidence::Medium => (base * 0.7, base, base * 1.3),
            Confidence::Low => (base * 0.4, base, base * 1.6),
        }
    }
}

/// System reliability for components in series.
pub fn series_reliability(models: &[ReliabilityModel], t: f64) -> f64 {
    models.iter().map(|m| m.reliability_at(t)).fold(1.0, |acc, r| acc * r)
}

/// System reliability for components in parallel (k-of-n).
pub fn parallel_reliability(models: &[ReliabilityModel], t: f64, k: usize) -> f64 {
    let n = models.len();
    if k == 0 || n == 0 {
        return 1.0;
    }
    if k > n {
        return 0.0;
    }
    let rs: Vec<f64> = models.iter().map(|m| m.reliability_at(t)).collect();
    // Sum over all combinations of size >= k
    let mut total = 0.0;
    for mask in 1u32..=(1u32 << n) - 1 {
        let ones = mask.count_ones() as usize;
        if ones >= k {
            let mut prob = 1.0;
            for i in 0..n {
                if mask & (1 << i) != 0 {
                    prob *= rs[i];
                } else {
                    prob *= 1.0 - rs[i];
                }
            }
            total += prob;
        }
    }
    total
}

/// System MTBF for series components.
pub fn system_mtbf_series(models: &[ReliabilityModel]) -> f64 {
    let total_rate: f64 = models.iter().map(|m| m.failure_rate).sum();
    if total_rate > 0.0 {
        1.0 / total_rate
    } else {
        f64::INFINITY
    }
}

/// Classify a system MTBF into a ternary failure mode.
pub fn classify_mtbf(mtbf: f64, threshold_negligible: f64, threshold_avoid: f64) -> FailureMode {
    if mtbf >= threshold_negligible {
        FailureMode::Negligible
    } else if mtbf >= threshold_avoid {
        FailureMode::Critical
    } else {
        FailureMode::Avoid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_mode_severity() {
        assert!((FailureMode::Avoid.severity_score() - 1.0).abs() < 0.001);
        assert!((FailureMode::Negligible.severity_score() - 0.0).abs() < 0.001);
        assert!((FailureMode::Critical.severity_score() - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_failure_mode_from_severity() {
        assert_eq!(FailureMode::from_severity(0.1), FailureMode::Negligible);
        assert_eq!(FailureMode::from_severity(0.5), FailureMode::Critical);
        assert_eq!(FailureMode::from_severity(0.9), FailureMode::Avoid);
    }

    #[test]
    fn test_confidence_weight() {
        assert!((Confidence::Low.weight() - 0.33).abs() < 0.01);
        assert!((Confidence::Medium.weight() - 0.67).abs() < 0.01);
        assert!((Confidence::High.weight() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_fmea_entry_rpn() {
        let entry = FmeaEntry::new("motor", 8, 5, 3);
        assert_eq!(entry.rpn(), 120);
    }

    #[test]
    fn test_fmea_entry_classification() {
        let high = FmeaEntry::new("high_risk", 10, 10, 5);
        assert_eq!(high.classification, FailureMode::Avoid); // RPN=500
        let low = FmeaEntry::new("low_risk", 2, 2, 3);
        assert_eq!(low.classification, FailureMode::Negligible); // RPN=12
        let mid = FmeaEntry::new("mid_risk", 5, 5, 5);
        assert_eq!(mid.classification, FailureMode::Critical); // RPN=125
    }

    #[test]
    fn test_fmea_analysis() {
        let mut analysis = FmeaAnalysis::new();
        analysis.add(FmeaEntry::new("a", 10, 10, 10));
        analysis.add(FmeaEntry::new("b", 2, 2, 2));
        analysis.add(FmeaEntry::new("c", 5, 5, 5));
        assert_eq!(analysis.entries.len(), 3);
        let sorted = analysis.sorted_by_risk();
        assert_eq!(sorted[0].name, "a");
        assert_eq!(sorted[2].name, "b");
    }

    #[test]
    fn test_fmea_avg_rpn() {
        let mut analysis = FmeaAnalysis::new();
        analysis.add(FmeaEntry::new("x", 5, 5, 5)); // 125
        assert!((analysis.avg_rpn() - 125.0).abs() < 0.001);
    }

    #[test]
    fn test_fmea_count_by_classification() {
        let mut analysis = FmeaAnalysis::new();
        analysis.add(FmeaEntry::new("a", 10, 10, 5));
        analysis.add(FmeaEntry::new("b", 2, 2, 2));
        let counts = analysis.count_by_classification();
        assert_eq!(counts[&FailureMode::Avoid], 1);
        assert_eq!(counts[&FailureMode::Negligible], 1);
    }

    #[test]
    fn test_fault_tree_basic() {
        let node = FaultNode::Basic {
            name: "pump".to_string(),
            mode: FailureMode::Critical,
        };
        assert_eq!(node.evaluate(), FailureMode::Critical);
    }

    #[test]
    fn test_fault_tree_and_gate() {
        let tree = FaultNode::Gate {
            name: "system".to_string(),
            gate: TernaryGate::TernaryAnd,
            children: vec![
                FaultNode::Basic { name: "a".to_string(), mode: FailureMode::Negligible },
                FaultNode::Basic { name: "b".to_string(), mode: FailureMode::Critical },
            ],
        };
        assert_eq!(tree.evaluate(), FailureMode::Critical); // max
    }

    #[test]
    fn test_fault_tree_or_gate() {
        let tree = FaultNode::Gate {
            name: "redundant".to_string(),
            gate: TernaryGate::TernaryOr,
            children: vec![
                FaultNode::Basic { name: "a".to_string(), mode: FailureMode::Critical },
                FaultNode::Basic { name: "b".to_string(), mode: FailureMode::Negligible },
            ],
        };
        assert_eq!(tree.evaluate(), FailureMode::Critical);
    }

    #[test]
    fn test_fault_tree_not_gate() {
        let tree = FaultNode::Gate {
            name: "inverted".to_string(),
            gate: TernaryGate::TernaryNot,
            children: vec![
                FaultNode::Basic { name: "x".to_string(), mode: FailureMode::Avoid },
            ],
        };
        assert_eq!(tree.evaluate(), FailureMode::Negligible);
    }

    #[test]
    fn test_fault_tree_k_of_n() {
        let tree = FaultNode::Gate {
            name: "voting".to_string(),
            gate: TernaryGate::KofN { k: 2 },
            children: vec![
                FaultNode::Basic { name: "a".to_string(), mode: FailureMode::Critical },
                FaultNode::Basic { name: "b".to_string(), mode: FailureMode::Critical },
                FaultNode::Basic { name: "c".to_string(), mode: FailureMode::Negligible },
            ],
        };
        assert_eq!(tree.evaluate(), FailureMode::Critical); // 2 active >= k=2
    }

    #[test]
    fn test_fault_tree_k_of_n_insufficient() {
        let tree = FaultNode::Gate {
            name: "voting".to_string(),
            gate: TernaryGate::KofN { k: 3 },
            children: vec![
                FaultNode::Basic { name: "a".to_string(), mode: FailureMode::Critical },
                FaultNode::Basic { name: "b".to_string(), mode: FailureMode::Negligible },
                FaultNode::Basic { name: "c".to_string(), mode: FailureMode::Negligible },
            ],
        };
        assert_eq!(tree.evaluate(), FailureMode::Negligible); // only 1 active < k=3
    }

    #[test]
    fn test_fault_tree_basic_events() {
        let tree = FaultNode::Gate {
            name: "root".to_string(),
            gate: TernaryGate::TernaryAnd,
            children: vec![
                FaultNode::Basic { name: "a".to_string(), mode: FailureMode::Critical },
                FaultNode::Basic { name: "b".to_string(), mode: FailureMode::Avoid },
            ],
        };
        let events = tree.basic_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_reliability_model() {
        let model = ReliabilityModel::new("pump", 0.001, Confidence::High);
        assert!((model.reliability_at(0.0) - 1.0).abs() < 0.001);
        assert!(model.reliability_at(1000.0) < 0.5);
    }

    #[test]
    fn test_mtbf() {
        let model = ReliabilityModel::new("valve", 0.01, Confidence::High);
        assert!((model.mtbf() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_adjusted_mtbf() {
        let model = ReliabilityModel::new("sensor", 0.01, Confidence::Medium);
        assert!((model.adjusted_mtbf() - 67.0).abs() < 1.0);
    }

    #[test]
    fn test_mtbf_bounds() {
        let model = ReliabilityModel::new("motor", 0.01, Confidence::High);
        let (low, expected, high) = model.mtbf_bounds();
        assert!(low < expected);
        assert!(expected < high);
        assert!((expected - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_series_reliability() {
        let models = vec![
            ReliabilityModel::new("a", 0.001, Confidence::High),
            ReliabilityModel::new("b", 0.002, Confidence::High),
        ];
        let r = series_reliability(&models, 100.0);
        assert!(r > 0.0 && r < 1.0);
    }

    #[test]
    fn test_parallel_reliability() {
        let models = vec![
            ReliabilityModel::new("a", 0.001, Confidence::High),
            ReliabilityModel::new("b", 0.001, Confidence::High),
        ];
        // 1-of-2 parallel: system works if at least 1 works
        let r = parallel_reliability(&models, 100.0, 1);
        let r_single = models[0].reliability_at(100.0);
        assert!(r > r_single); // parallel is more reliable than single
    }

    #[test]
    fn test_system_mtbf_series() {
        let models = vec![
            ReliabilityModel::new("a", 0.01, Confidence::High),
            ReliabilityModel::new("b", 0.01, Confidence::High),
        ];
        let mtbf = system_mtbf_series(&models);
        assert!((mtbf - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_classify_mtbf() {
        assert_eq!(classify_mtbf(1000.0, 500.0, 100.0), FailureMode::Negligible);
        assert_eq!(classify_mtbf(300.0, 500.0, 100.0), FailureMode::Critical);
        assert_eq!(classify_mtbf(50.0, 500.0, 100.0), FailureMode::Avoid);
    }
}
