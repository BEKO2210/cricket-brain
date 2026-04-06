# Cricket-Brain Architecture

## The Münster Model

The Cricket-Brain is based on the Münster model of cricket auditory processing, first described by researchers studying the phonotactic behavior of female crickets (*Gryllus bimaculatus*).

## Neuron Roles

### AN1 — Auditory Receptor Neuron
- **Input**: Raw acoustic signal
- **Tuning**: 4500 Hz (species-specific carrier frequency)
- **Function**: Frequency-selective resonator. Only responds to signals near its eigenfrequency using a Gaussian tuning curve.

### LN2 — Inhibitory Local Interneuron (3ms delay)
- **Input**: AN1 via inhibitory synapse with 3ms delay
- **Output**: ON1 (inhibitory)
- **Function**: Provides delayed inhibition to the output neuron. Creates a temporal window that suppresses responses to non-patterned signals.

### LN3 — Excitatory Local Interneuron (2ms delay)
- **Input**: AN1 via excitatory synapse with 2ms delay
- **Output**: ON1 (excitatory)
- **Function**: Provides delayed excitation to the output neuron. When the timing of excitation aligns with gaps in inhibition, ON1 fires.

### LN5 — Inhibitory Local Interneuron (5ms delay)
- **Input**: AN1 via inhibitory synapse with 5ms delay
- **Output**: ON1 (inhibitory)
- **Function**: Provides a second, longer-delayed inhibition that creates the pulse-interval selectivity. Combined with LN2, this creates a narrow temporal window for excitation.

### ON1 — Output Neuron (4ms coincidence window)
- **Input**: LN2 (inhibitory), LN3 (excitatory), LN5 (inhibitory)
- **Function**: Coincidence detector. Fires only when current activation AND delayed activation (from 4ms ago) both exceed threshold. This ensures that only sustained, correctly-timed patterns trigger output.

## Signal Flow

```
Sound (4500 Hz pulse train)
        │
        ▼
       AN1 ─── Gaussian resonator (freq-selective)
        │
        ├──▶ LN2 (inhibitory, 3ms delay) ──▶ ON1
        │
        ├──▶ LN5 (inhibitory, 5ms delay) ──▶ ON1
        │
        └──▶ LN3 (excitatory, 2ms delay) ──▶ ON1
                                                │
                                                ▼
                                         Coincidence Check
                                         (current ∧ delayed)
                                                │
                                                ▼
                                           Output Spike
```

## Why This Works

The key insight is that the **relative timing** of excitation and inhibition at ON1 determines whether the neuron fires. The inhibitory signals from LN2 (3ms) and LN5 (5ms) create a temporal window where only the excitatory signal from LN3 (2ms) can drive ON1 above threshold.

This makes the network selectively responsive to specific temporal patterns — like Morse code pulses or cricket chirp intervals — without any training or weight adjustment.
