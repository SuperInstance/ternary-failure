# ternary-failure

Failure analysis with ternary classification — FMEA risk analysis, fault trees with ternary gates, reliability modeling, MTBF estimation, and ternary confidence bounds.

## Why This Exists

Reliability engineering and failure analysis often categorize risks into three levels: acceptable, needs attention, and must avoid. This crate formalizes that ternary classification into a complete toolkit: FMEA (Failure Mode and Effects Analysis) with Risk Priority Numbers, fault trees with ternary logic gates (AND, OR, NOT, K-of-N), component reliability models with exponential failure rates, and MTBF estimation with ternary confidence bounds (low/medium/high). `forbid(unsafe_code)` throughout.

## Core Concepts

- **FailureMode**: Ternary severity — `Avoid` (must prevent), `Negligible` (tolerable), `Critical` (must address).
- **Confidence**: Ternary estimate quality — `Low`, `Medium`, `High`, affecting MTBF bounds.
- **FmeaEntry**: A failure mode with severity, occurrence, and detection ratings (1–10). Risk Priority Number = S × O × D.
- **FmeaAnalysis**: Aggregate FMEA entries, sort by risk, count by classification, compute average and max RPN.
- **FaultNode / TernaryGate**: Fault tree with `TernaryAnd` (worst of inputs), `TernaryOr` (any non-negligible), `TernaryNot` (invert Avoid↔Negligible), and `KofN` (at least K active).
- **ReliabilityModel**: Exponential reliability model `R(t) = e^{-λt}` with MTBF, confidence-adjusted MTBF, and ternary confidence bounds.
- **System reliability**: `series_reliability` and `parallel_reliability` for composing component models.

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-failure = "0.1"
```

```rust
use ternary_failure::{
    FailureMode, FmeaEntry, FmeaAnalysis,
    FaultNode, TernaryGate,
    ReliabilityModel, Confidence,
    series_reliability, system_mtbf_series, classify_mtbf,
};

fn main() {
    // FMEA analysis
    let mut fmea = FmeaAnalysis::new();
    fmea.add(FmeaEntry::new("motor overheating", 8, 5, 3));  // RPN = 120, Critical
    fmea.add(FmeaEntry::new("cosmetic scratch",     2, 2, 2));  // RPN = 8,   Negligible
    fmea.add(FmeaEntry::new("brake failure",       10, 3, 8));  // RPN = 240, Avoid

    for entry in fmea.sorted_by_risk() {
        println!("{:20s} RPN={:3} {:?}", entry.name, entry.rpn(), entry.ternary_risk());
    }

    // Fault tree
    let tree = FaultNode::Gate {
        name: "system".into(),
        gate: TernaryGate::TernaryOr,
        children: vec![
            FaultNode::Basic { name: "pump".into(), mode: FailureMode::Critical },
            FaultNode::Basic { name: "sensor".into(), mode: FailureMode::Negligible },
        ],
    };
    println!("System failure mode: {:?}", tree.evaluate());

    // Reliability modeling
    let pump = ReliabilityModel::new("pump", 0.001, Confidence::High);
    let valve = ReliabilityModel::new("valve", 0.002, Confidence::Medium);

    println!("Pump MTBF: {:.1} hours", pump.mtbf());
    let (lo, expected, hi) = valve.mtbf_bounds();
    println!("Valve MTBF bounds: [{:.1}, {:.1}, {:.1}]", lo, expected, hi);

    let r_system = series_reliability(&[pump.clone(), valve.clone()], 100.0);
    let mtbf_system = system_mtbf_series(&[pump, valve]);

    // Classify system MTBF
    let mode = classify_mtbf(mtbf_system, 500.0, 100.0);
    println!("System classification: {:?}", mode);
}
```

## API Overview

| Type / Function | Description |
|---|---|
| `FailureMode` | `Avoid`, `Negligible`, `Critical` — with severity scores |
| `Confidence` | `Low`, `Medium`, `High` — with weights for MTBF bounds |
| `FmeaEntry` | Failure mode with S/O/D ratings, RPN, ternary classification |
| `FmeaAnalysis` | Aggregate FMEA: `sorted_by_risk()`, `avg_rpn()`, `count_by_classification()` |
| `TernaryGate` | `TernaryAnd`, `TernaryOr`, `TernaryNot`, `KofN{k}` |
| `FaultNode` | `Basic` event or `Gate` with children; `evaluate()`, `basic_events()` |
| `ReliabilityModel` | Exponential model: `reliability_at(t)`, `mtbf()`, `adjusted_mtbf()`, `mtbf_bounds()` |
| `series_reliability` | System reliability for components in series |
| `parallel_reliability` | System reliability for k-of-n parallel components |
| `system_mtbf_series` | System MTBF for series components |
| `classify_mtbf` | Map MTBF value to `FailureMode` given thresholds |

## How It Works

**FMEA** computes Risk Priority Numbers as the product of severity × occurrence × detection ratings (each 1–10). Entries are automatically classified: RPN ≥ 200 → `Avoid`, RPN ≤ 50 → `Negligible`, otherwise → `Critical`.

**Fault trees** use ternary logic gates. `TernaryAnd` returns the worst (highest severity) of its children. `TernaryOr` returns the first non-negligible child (any active failure propagates). `TernaryNot` swaps `Avoid ↔ Negligible` and keeps `Critical`. `KofN` returns `Critical` if at least K children are non-negligible.

**ReliabilityModel** assumes a constant failure rate `λ` (exponential distribution). `R(t) = e^{-λt}`, `MTBF = 1/λ`. Confidence levels adjust the reported MTBF bounds: `High` gives tight bounds (±10%), `Medium` wider (±30%), `Low` very wide (+60%/−60%).

**System reliability** composes components: series multiplies individual reliabilities, parallel sums over k-of-n combinatorial survival probabilities.

## Use Cases

- **Hardware reliability engineering**: FMEA analysis and fault tree evaluation for mechanical/electrical systems.
- **Software incident analysis**: Classify incident types by severity/occurrence/detection and build fault trees for root cause analysis.
- **Safety-critical system design**: Model system reliability with confidence bounds and classify MTBF against safety thresholds.
- **Risk assessment**: Ternary risk classification for any domain where risks naturally fall into must-avoid / critical / tolerable categories.

## Known Limitations

- **FMEA classification thresholds are hardcoded**: `FmeaEntry::new()` classifies risk using fixed RPN thresholds: ≥200 → `Avoid`, ≤50 → `Negligible`, else `Critical`. These industry-derived thresholds are not configurable and may not match your domain's risk appetite. A critical system might need `Avoid` at RPN 100, not 200.

- **Fault tree `TernaryOr` is short-circuit**: `TernaryOr` returns the first non-negligible child without evaluating the rest. This means it can miss the worst-case failure mode if children are ordered with a `Critical` before an `Avoid` event. The result depends on child ordering.

- **Exponential reliability model assumes constant failure rate**: `ReliabilityModel` uses `R(t) = e^{−λt}`, which assumes failures are memoryless (constant hazard rate). Real systems have bathtub-curve failure rates (infant mortality → useful life → wear-out). The model is inaccurate during burn-in and end-of-life phases.

- **Confidence bounds are symmetric multipliers, not statistical**: `mtbf_bounds()` applies fixed multipliers based on `Confidence` level (e.g., ±30% for Medium). These are not confidence intervals from a chi-squared distribution or any statistical model — they're rough engineering heuristics.

- **`series_reliability()` assumes independent failures**: The product `R_system = ∏ Rᵢ` assumes component failures are statistically independent. In practice, shared environmental stresses (temperature, vibration) create correlated failures that make actual system reliability lower than predicted.

- **No repair modeling**: The reliability models represent only the failure process. There is no mean time to repair (MTTR), availability calculation, or Markov model for repairable systems.

## Ecosystem

Part of the **SuperInstance** ternary computing suite:

- `ternary-lattice` — lattice structures for ternary values
- `ternary-codes` — error-correcting codes for ternary data
- `ternary-gradient` — gradient-free optimization on ternary landscapes
- `ternary-language` — ternary NLP and grammar processing
- `ternary-trees` — ternary decision trees and forests
- `ternary-transform` — wavelet, Fourier, and kernel transforms
- `ternary-planning` — planning and scheduling with ternary priorities
- `ternary-rl` — reinforcement learning with ternary actions
- `ternary-som` — self-organizing maps for ternary data
- `ternary-failure` — this crate

## License

MIT
