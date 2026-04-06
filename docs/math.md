# Mathematical Foundation

## Gaussian Frequency Tuning

Each neuron has a characteristic eigenfrequency `f₀` and responds to input frequencies `f_in` according to a Gaussian tuning curve:

```
match = exp( -(Δf / f₀ / w)² )
```

where:
- `Δf = |f_in - f₀|` — absolute frequency deviation
- `f₀` — neuron's eigenfrequency (e.g., 4500 Hz)
- `w` — bandwidth parameter (default: 0.1 = 10% of eigenfrequency; configurable per-neuron via `Neuron.bandwidth`. `ResonatorBank` adapts this automatically based on token spacing)

At 0% deviation: `match = exp(0) = 1.0`
At 10% deviation: `match = exp(-1) ≈ 0.368`
At 20% deviation: `match = exp(-4) ≈ 0.018`

The threshold for resonance is `match > 0.3`, giving an effective bandwidth of approximately ±11% of eigenfrequency.

## Amplitude Update

When the neuron resonates (`match > 0.3`):

```
A(t+1) = min( A(t) + match · 0.3, 1.0 )
```

The amplitude grows proportionally to match strength, capped at 1.0.

## Amplitude Decay

When the neuron does not resonate (`match ≤ 0.3`):

```
A(t+1) = A(t) · 0.95
```

Exponential decay with time constant τ = -1/ln(0.95) ≈ 19.5 timesteps.

## Phase Update

When resonating, the phase locks to the input signal:

```
φ(t+1) = φ(t) + (φ_in - φ(t)) · 0.1
```

This is an exponential moving average with α = 0.1, providing smooth phase-locking.

When not resonating, the phase drifts toward zero:

```
φ(t+1) = φ(t) · 0.98
```

## Delay-Line Coincidence Detection

The output neuron uses a coincidence detection rule:

```
fire = (A(t) > θ) ∧ (A(t - τ) > θ · 0.8)
```

where:
- `A(t)` — current amplitude
- `A(t - τ)` — amplitude from τ timesteps ago (read from history buffer)
- `θ` — firing threshold (default: 0.7)
- `τ` — delay tap length (default: 4ms for ON1)

The neuron fires only when **both** conditions are met:
1. Current amplitude exceeds the threshold
2. The amplitude τ timesteps ago also exceeded 80% of the threshold

This ensures that only **sustained** activation patterns trigger firing, preventing false positives from transient spikes.

## Synaptic Delay

Each synapse implements a pure delay line:

```
output(t) = input(t - d)
```

where `d` is the delay in timesteps. For inhibitory synapses:

```
output(t) = -input(t - d)
```

The delay is implemented as a FIFO ring buffer of length `d`.
