# Future Integration: ternary-failure

## Current State
Provides failure mode classification (Avoid/Negligible/Critical), FMEA-style risk analysis, fault trees with ternary gates, reliability modeling, and MTBF estimation with ternary confidence levels.

## Integration Opportunities

### With ternary-cell (Failure Prevention)
Cell population crashes are failure events. `FailureMode::Critical` maps to a cell grid collapse (too many cells pruned in `gc` phase). FMEA analysis on the tick cycle identifies which phases are most failure-prone. Fault tree analysis traces a cell failure back to its root cause: was it the `acquire` phase (bad input), the `predict` phase (wrong model), or the `gc` phase (over-aggressive pruning)?

### With ternary-failure → avoidance-cascade
Avoidance cascades ARE a failure mode. The death spiral from `avoidance-cascade` is a `FailureMode::Critical` event. `ternary-failure` provides the FMEA framework for analyzing it; `avoidance-cascade` provides the specific cascade model. Together: general failure analysis + specific cascade detection.

### With ternary-chaos
Chaotic systems have high failure rates. Lyapunov exponents from `ternary-chaos` feed into reliability modeling: positive Lyapunov = decreasing MTBF. Together, they predict when a room will fail and how to prevent it.

## Potential in Mature Systems
In room-as-codespace, every room has a failure mode analysis. FMEA identifies what can go wrong: Codespace timeout, ensign crash, resource exhaustion. Fault trees trace failures to root causes. MTBF estimation predicts maintenance windows. Ternary confidence levels track how sure we are about each prediction.

## Cross-Pollination Ideas
- FMEA as a room design tool — design rooms that are robust against identified failure modes
- Fault trees as debugging tools — when a room fails, trace the failure tree automatically
- MTBF as a Codespace lifecycle parameter — preemptively restart rooms approaching their MTBF

## Dependencies for Next Steps
- Integration with ternary-cell for tick-phase FMEA
- Integration with avoidance-cascade for cascade-specific failure analysis
- ternary-room needs failure mode tracking per room
