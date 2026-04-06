# Scaling Roadmap

## Platform Tiers

### Tier 1: Arduino Uno (ATmega328P)
- **RAM**: 2 KB
- **Flash**: 32 KB
- **Clock**: 16 MHz
- **Max neurons**: ~8 (with fixed-size arrays)
- **Use case**: Single-pattern recognition (e.g., one Morse character)
- **Implementation**: `no_std`, fixed arrays `[f32; 16]`, no heap allocation

### Tier 2: ESP32
- **RAM**: 520 KB
- **Flash**: 4 MB
- **Clock**: 240 MHz (dual-core)
- **Max neurons**: ~2,000
- **Use case**: Multi-pattern recognition, real-time audio processing
- **Implementation**: `no_std` with `alloc`, VecDeque on heap

### Tier 3: Raspberry Pi / Desktop
- **RAM**: 1+ GB
- **Clock**: 1+ GHz (multi-core)
- **Max neurons**: 40,000+
- **Use case**: Full-scale pattern recognition, benchmark comparison
- **Implementation**: `std`, full library features

## Memory Formula

Per neuron:
```
bytes = size_of(Neuron) + (delay_taps + 1) * 4
     ≈ 48 + delay * 4
```

Per synapse:
```
bytes = size_of(DelaySynapse) + delay * 4
     ≈ 56 + delay * 4
```

Total estimate for `N` neurons with `K` synapses, average delay `D`:
```
total ≈ N * (48 + D*4) + K * (56 + D*4)
```

## Scaling Examples

| Config | Neurons | Synapses | Avg Delay | Est. Memory |
|--------|---------|----------|-----------|-------------|
| Arduino | 5 | 6 | 3 | ~600 B |
| ESP32 | 2,000 | 6,000 | 5 | ~530 KB |
| Desktop | 40,960 | 122,880 | 6 | ~12 MB |

## Performance Characteristics

The Cricket-Brain scales **linearly** with neuron count:
- Each `step()` iterates all neurons once: O(N)
- Each synapse transmits once: O(K)
- No matrix multiplication: O(1) per neuron operation
- No backpropagation: O(0) training cost

This is fundamentally different from transformer architectures where inference scales as O(N² · D) for attention computation.

## Roadmap

- **v0.1** — Morse code recognition (current)
- **v0.2** — Multi-frequency token recognition (parallel resonator banks)
- **v0.3** — 40k resonator network for sequence prediction
- **v1.0** — Arduino port with real-time audio input via ADC
