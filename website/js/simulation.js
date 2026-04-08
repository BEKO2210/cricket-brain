'use strict';

/* ==============================================================
   Cricket-Brain Live Simulation Engine
   JavaScript port of the Rust neuromorphic engine
   ============================================================== */

class Neuron {
  constructor(id, name, eigenfreq, delayMs) {
    this.id = id;
    this.name = name;
    this.eigenfreq = eigenfreq;
    this.phase = 0;
    this.amplitude = 0;
    this.threshold = 0.7;
    this.delayTaps = delayMs;
    this.capacity = delayMs + 1;
    this.history = new Float32Array(this.capacity).fill(0);
    this.histHead = 0;
    this.histLen = this.capacity;
  }

  resonate(inputFreq, inputPhase) {
    const df = Math.abs(inputFreq - this.eigenfreq);
    const norm = df / this.eigenfreq / 0.1;
    const match = Math.exp(-(norm * norm));

    if (match > 0.3) {
      this.amplitude = Math.min(this.amplitude + match * 0.3, 1.0);
      this.phase += (inputPhase - this.phase) * 0.1;
    } else {
      this.amplitude *= 0.95;
      this.phase *= 0.98;
    }

    this.history[this.histHead] = this.amplitude;
    this.histHead = (this.histHead + 1) % this.capacity;
    return this.amplitude;
  }

  checkCoincidence() {
    const oldest = this.history[this.histHead % this.capacity];
    return this.amplitude > this.threshold && oldest > this.threshold * 0.8;
  }

  decay() { this.amplitude *= 0.95; this.phase *= 0.98; }

  silenceDecay() {
    this.amplitude *= 0.5;
    this.phase *= 0.5;
    this.history[this.histHead] = this.amplitude;
    this.histHead = (this.histHead + 1) % this.capacity;
  }

  reset() {
    this.amplitude = 0;
    this.phase = 0;
    this.history.fill(0);
    this.histHead = 0;
  }
}

class DelaySynapse {
  constructor(from, to, delay, inhibitory) {
    this.from = from;
    this.to = to;
    this.delay = delay;
    this.inhibitory = inhibitory;
    this.buffer = new Float32Array(delay).fill(0);
    this.head = 0;
  }

  transmit(signal) {
    const delayed = this.buffer[this.head];
    this.buffer[this.head] = signal;
    this.head = (this.head + 1) % this.delay;
    return this.inhibitory ? -delayed : delayed;
  }

  reset() { this.buffer.fill(0); this.head = 0; }
}

class CricketBrain {
  constructor() {
    this.neurons = [
      new Neuron(0, 'AN1', 4500, 4),
      new Neuron(1, 'LN2', 4500, 3),
      new Neuron(2, 'LN3', 4500, 2),
      new Neuron(3, 'LN5', 4500, 5),
      new Neuron(4, 'ON1', 4500, 4),
    ];
    this.synapses = [
      new DelaySynapse(0, 1, 3, true),
      new DelaySynapse(0, 2, 2, false),
      new DelaySynapse(0, 3, 5, true),
      new DelaySynapse(1, 4, 1, true),
      new DelaySynapse(2, 4, 1, false),
      new DelaySynapse(3, 4, 1, true),
    ];
    this.timeStep = 0;
  }

  step(inputFreq) {
    const phase = (this.timeStep * 0.01) % 1.0;
    const silent = inputFreq <= 0;

    if (silent) {
      this.neurons[0].silenceDecay();
    } else {
      this.neurons[0].resonate(inputFreq, phase);
    }

    const incoming = new Float32Array(5);
    for (const syn of this.synapses) {
      incoming[syn.to] += syn.transmit(this.neurons[syn.from].amplitude);
    }

    for (let i = 1; i < 5; i++) {
      if (silent) {
        this.neurons[i].silenceDecay();
      } else if (Math.abs(incoming[i]) > 0.01) {
        this.neurons[i].resonate(inputFreq, phase);
        this.neurons[i].amplitude = Math.max(0, Math.min(1, this.neurons[i].amplitude + incoming[i] * 0.2));
      } else {
        this.neurons[i].decay();
      }
    }

    this.timeStep++;
    const on1 = this.neurons[4];
    return on1.checkCoincidence() ? on1.amplitude : 0;
  }

  reset() {
    this.neurons.forEach(n => n.reset());
    this.synapses.forEach(s => s.reset());
    this.timeStep = 0;
  }
}

/* --- Renderer --- */
class SimulationRenderer {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    if (!this.canvas) return;
    this.ctx = this.canvas.getContext('2d');
    this.outputHistory = [];
    this.maxHistory = 250;
    this.resize();
    this._resizeObserver = new ResizeObserver(() => this.resize());
    this._resizeObserver.observe(this.canvas.parentElement);
  }

  resize() {
    const parent = this.canvas.parentElement;
    const rect = parent.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    this.w = rect.width;
    this.h = rect.height || 450;
    this.canvas.width = this.w * dpr;
    this.canvas.height = this.h * dpr;
    this.canvas.style.width = this.w + 'px';
    this.canvas.style.height = this.h + 'px';
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Proportional positioning: adapts to any canvas size
    const cx = this.w / 2, cy = this.h * 0.38;
    const sx = this.w * 0.32; // horizontal spread from center
    const sy = Math.min(this.h * 0.18, 100); // vertical spread for interneurons
    this.positions = [
      { x: cx - sx, y: cy, name: 'AN1', role: 'Receptor' },
      { x: cx - sx * 0.05, y: cy - sy, name: 'LN2', role: 'Inhibitory' },
      { x: cx - sx * 0.05, y: cy, name: 'LN3', role: 'Excitatory' },
      { x: cx - sx * 0.05, y: cy + sy, name: 'LN5', role: 'Inhibitory' },
      { x: cx + sx * 0.85, y: cy, name: 'ON1', role: 'Output' },
    ];
  }

  draw(brain, spikeOutput, inputFreq) {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.w, this.h);

    this.outputHistory.push(spikeOutput);
    if (this.outputHistory.length > this.maxHistory) this.outputHistory.shift();

    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const bgText = isDark ? '#94a3b8' : '#475569';
    const bgLine = isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)';
    const accentColor = '#00d4aa';
    const fireColor = '#f59e0b';
    const inhColor = '#ef4444';
    const excColor = '#22c55e';

    // Draw synapses
    const synMeta = [
      { from: 0, to: 1, label: '3ms', inh: true },
      { from: 0, to: 2, label: '2ms', inh: false },
      { from: 0, to: 3, label: '5ms', inh: true },
      { from: 1, to: 4, label: '1ms', inh: true },
      { from: 2, to: 4, label: '1ms', inh: false },
      { from: 3, to: 4, label: '1ms', inh: true },
    ];

    for (const s of synMeta) {
      const p0 = this.positions[s.from];
      const p1 = this.positions[s.to];
      const srcAmp = brain.neurons[s.from].amplitude;
      const alpha = 0.15 + srcAmp * 0.85;

      ctx.beginPath();
      ctx.moveTo(p0.x, p0.y);
      ctx.lineTo(p1.x, p1.y);
      ctx.strokeStyle = s.inh ? inhColor : excColor;
      ctx.globalAlpha = alpha;
      ctx.lineWidth = 1.5 + srcAmp * 2;
      ctx.setLineDash(srcAmp > 0.1 ? [6, 4] : []);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.globalAlpha = 1;

      // Delay label
      const mx = (p0.x + p1.x) / 2;
      const my = (p0.y + p1.y) / 2 - 10;
      ctx.font = '10px system-ui';
      ctx.fillStyle = bgText;
      ctx.textAlign = 'center';
      ctx.fillText(s.label + (s.inh ? ' inh' : ' exc'), mx, my);
    }

    // Draw neurons
    for (let i = 0; i < 5; i++) {
      const n = brain.neurons[i];
      const p = this.positions[i];
      const r = 28 + n.amplitude * 14;
      const firing = i === 4 && spikeOutput > 0;

      // Glow
      if (n.amplitude > 0.3 || firing) {
        const grd = ctx.createRadialGradient(p.x, p.y, r * 0.5, p.x, p.y, r * 2.5);
        grd.addColorStop(0, firing ? 'rgba(245,158,11,0.25)' : 'rgba(0,212,170,0.2)');
        grd.addColorStop(1, 'transparent');
        ctx.fillStyle = grd;
        ctx.fillRect(p.x - r * 2.5, p.y - r * 2.5, r * 5, r * 5);
      }

      // Amplitude ring
      ctx.beginPath();
      ctx.arc(p.x, p.y, r + 4, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * n.amplitude);
      ctx.strokeStyle = firing ? fireColor : accentColor;
      ctx.lineWidth = 3;
      ctx.stroke();

      // Circle
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      const a = n.amplitude;
      if (firing) {
        ctx.fillStyle = `rgba(245,158,11,${0.3 + a * 0.4})`;
      } else if (a > 0.3) {
        ctx.fillStyle = `rgba(0,212,170,${0.15 + a * 0.3})`;
      } else {
        ctx.fillStyle = isDark ? 'rgba(30,41,59,0.8)' : 'rgba(226,232,240,0.8)';
      }
      ctx.fill();
      ctx.strokeStyle = firing ? fireColor : (a > 0.3 ? accentColor : bgLine);
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Name
      ctx.font = 'bold 13px system-ui';
      ctx.fillStyle = firing ? fireColor : (a > 0.3 ? accentColor : bgText);
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(p.name, p.x, p.y - 5);

      // Amplitude value
      ctx.font = '10px monospace';
      ctx.fillStyle = bgText;
      ctx.fillText(a.toFixed(2), p.x, p.y + 10);

      // Role label
      ctx.font = '9px system-ui';
      ctx.fillStyle = bgText;
      ctx.fillText(p.role, p.x, p.y + r + 16);
    }

    // Spike starburst on ON1
    if (spikeOutput > 0) {
      const p = this.positions[4];
      const t = brain.timeStep * 0.15;
      for (let i = 0; i < 8; i++) {
        const angle = (i / 8) * Math.PI * 2 + t;
        const len = 20 + Math.sin(t * 3 + i) * 8;
        ctx.beginPath();
        ctx.moveTo(p.x + Math.cos(angle) * 36, p.y + Math.sin(angle) * 36);
        ctx.lineTo(p.x + Math.cos(angle) * (36 + len), p.y + Math.sin(angle) * (36 + len));
        ctx.strokeStyle = fireColor;
        ctx.lineWidth = 2;
        ctx.globalAlpha = 0.6;
        ctx.stroke();
        ctx.globalAlpha = 1;
      }
    }

    // Input frequency indicator
    ctx.font = '12px monospace';
    ctx.fillStyle = inputFreq > 0 ? accentColor : bgText;
    ctx.textAlign = 'left';
    ctx.fillText(`Input: ${inputFreq > 0 ? inputFreq.toFixed(0) + ' Hz' : 'Silence'}`, 16, 24);
    ctx.fillText(`Step: ${brain.timeStep}`, 16, 40);

    // Output history graph
    const graphY = this.h * 0.78;
    const graphH = this.h * 0.16;
    const graphW = this.w - 32;

    ctx.fillStyle = bgText;
    ctx.font = '10px system-ui';
    ctx.textAlign = 'left';
    ctx.fillText('ON1 Output History', 16, graphY - 6);

    ctx.strokeStyle = bgLine;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(16, graphY + graphH);
    ctx.lineTo(16 + graphW, graphY + graphH);
    ctx.stroke();

    if (this.outputHistory.length > 1) {
      ctx.beginPath();
      for (let i = 0; i < this.outputHistory.length; i++) {
        const x = 16 + (i / this.maxHistory) * graphW;
        const y = graphY + graphH - this.outputHistory[i] * graphH;
        i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
      }
      ctx.strokeStyle = accentColor;
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Fill under curve
      ctx.lineTo(16 + ((this.outputHistory.length - 1) / this.maxHistory) * graphW, graphY + graphH);
      ctx.lineTo(16, graphY + graphH);
      ctx.closePath();
      ctx.fillStyle = 'rgba(0,212,170,0.08)';
      ctx.fill();
    }
  }
}

/* --- Morse Encoder --- */
const MORSE_TABLE = {
  S: '...', O: '---', H: '....', E: '.', L: '.-..', P: '.--.', W: '.--',
  A: '.-', B: '-...', C: '-.-.', D: '-..', F: '..-.', G: '--.', I: '..',
  J: '.---', K: '-.-', M: '--', N: '-.', Q: '--.-', R: '.-.', T: '-',
  U: '..-', V: '...-', X: '-..-', Y: '-.--', Z: '--..', ' ': '/'
};

function encodeMorseSignal(text) {
  const seq = [];
  const chars = text.toUpperCase().split('');
  for (let ci = 0; ci < chars.length; ci++) {
    const code = MORSE_TABLE[chars[ci]];
    if (!code) continue;
    if (code === '/') { seq.push({ freq: 0, dur: 350 }); continue; }
    for (let si = 0; si < code.length; si++) {
      seq.push({ freq: 4500, dur: code[si] === '.' ? 50 : 150 });
      if (si + 1 < code.length) seq.push({ freq: 0, dur: 50 });
    }
    if (ci + 1 < chars.length && chars[ci + 1] !== ' ') seq.push({ freq: 0, dur: 150 });
  }
  return seq;
}

/* --- Controller --- */
class SimulationController {
  constructor() {
    this.brain = new CricketBrain();
    this.renderer = new SimulationRenderer('neuron-canvas');
    this.running = false;
    this.inputFreq = 0;
    this.speed = 3;
    this.spikeCount = 0;
    this.lastOutput = 0;
    this.morseMode = false;
    this.morseSignal = [];
    this.morseIndex = 0;
    this.morseStepInSegment = 0;
    this._raf = null;

    this.bindControls();
    if (this.renderer.canvas) this.draw();
  }

  bindControls() {
    const $ = (id) => document.getElementById(id);

    this.freqSlider = $('freq-slider');
    this.freqDisplay = $('freq-display');
    this.speedSlider = $('speed-slider');
    this.speedDisplay = $('speed-display');
    this.playBtn = $('play-btn');
    this.resetBtn = $('reset-btn');

    if (this.freqSlider) {
      this.freqSlider.addEventListener('input', () => {
        this.inputFreq = +this.freqSlider.value;
        if (this.freqDisplay) this.freqDisplay.textContent = this.inputFreq + ' Hz';
        this.morseMode = false;
        this.updatePresetActive();
      });
    }

    if (this.speedSlider) {
      this.speedSlider.addEventListener('input', () => {
        this.speed = +this.speedSlider.value;
        if (this.speedDisplay) this.speedDisplay.textContent = this.speed + 'x';
      });
    }

    if (this.playBtn) {
      this.playBtn.addEventListener('click', () => this.togglePlay());
    }

    if (this.resetBtn) {
      this.resetBtn.addEventListener('click', () => this.reset());
    }

    document.querySelectorAll('.preset-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const preset = btn.dataset.preset;
        this.morseMode = false;
        if (preset === 'silence') { this.setFreq(0); }
        else if (preset === 'cricket') { this.setFreq(4500); }
        else if (preset === 'off-freq') { this.setFreq(2000); }
        else if (preset === 'morse-sos') { this.startMorse('SOS'); }
        else if (preset === 'morse-hello') { this.startMorse('HELLO'); }
        this.updatePresetActive();
        if (!this.running) this.togglePlay();
      });
    });
  }

  setFreq(f) {
    this.inputFreq = f;
    if (this.freqSlider) this.freqSlider.value = f;
    if (this.freqDisplay) this.freqDisplay.textContent = f + ' Hz';
  }

  startMorse(text) {
    this.morseMode = true;
    this.morseSignal = encodeMorseSignal(text);
    this.morseIndex = 0;
    this.morseStepInSegment = 0;
  }

  updatePresetActive() {
    document.querySelectorAll('.preset-btn').forEach(b => b.classList.remove('active'));
    if (this.morseMode) {
      const btn = document.querySelector('.preset-btn[data-preset^="morse"]');
      if (btn) btn.classList.add('active');
    } else if (this.inputFreq === 0) {
      document.querySelector('.preset-btn[data-preset="silence"]')?.classList.add('active');
    } else if (this.inputFreq === 4500) {
      document.querySelector('.preset-btn[data-preset="cricket"]')?.classList.add('active');
    } else if (this.inputFreq === 2000) {
      document.querySelector('.preset-btn[data-preset="off-freq"]')?.classList.add('active');
    }
  }

  togglePlay() {
    this.running = !this.running;
    if (this.playBtn) {
      this.playBtn.textContent = this.running ? 'Pause' : 'Play';
      this.playBtn.setAttribute('aria-label', this.running ? 'Pause simulation' : 'Start simulation');
    }
    if (this.running) this.loop();
  }

  reset() {
    this.brain.reset();
    this.spikeCount = 0;
    this.lastOutput = 0;
    this.morseMode = false;
    this.morseIndex = 0;
    this.morseStepInSegment = 0;
    this.renderer.outputHistory = [];
    this.setFreq(0);
    this.updateReadout();
    this.updateExplanation();
    this.draw();
    this.updatePresetActive();
  }

  loop() {
    if (!this.running) return;
    for (let s = 0; s < this.speed; s++) {
      let freq = this.inputFreq;

      if (this.morseMode && this.morseIndex < this.morseSignal.length) {
        const seg = this.morseSignal[this.morseIndex];
        freq = seg.freq;
        this.morseStepInSegment++;
        if (this.morseStepInSegment >= seg.dur) {
          this.morseIndex++;
          this.morseStepInSegment = 0;
          if (this.morseIndex >= this.morseSignal.length) {
            this.morseMode = false;
            freq = 0;
          }
        }
      }

      this.lastOutput = this.brain.step(freq);
      if (this.lastOutput > 0) this.spikeCount++;
      this.inputFreq = freq; // update display
    }

    this.draw();
    this.updateReadout();
    this.updateExplanation();
    this._raf = requestAnimationFrame(() => this.loop());
  }

  draw() {
    if (this.renderer.canvas) {
      this.renderer.draw(this.brain, this.lastOutput, this.inputFreq);
    }
  }

  updateReadout() {
    const set = (id, val) => {
      const el = document.getElementById(id);
      if (el) el.textContent = val;
    };
    set('ro-freq', this.inputFreq > 0 ? this.inputFreq.toFixed(0) + ' Hz' : 'Silence');
    set('ro-an1', this.brain.neurons[0].amplitude.toFixed(3));
    set('ro-on1', this.lastOutput.toFixed(3));
    set('ro-spikes', this.spikeCount.toLocaleString());
    set('ro-step', this.brain.timeStep.toLocaleString());

    const on1El = document.getElementById('ro-on1');
    if (on1El) {
      on1El.classList.toggle('firing', this.lastOutput > 0);
    }
  }

  updateExplanation() {
    const el = document.getElementById('sim-explanation');
    if (!el) return;

    const freq = this.inputFreq;
    const an1 = this.brain.neurons[0].amplitude;
    const on1 = this.lastOutput;

    let text = '';
    if (!this.running) {
      text = 'Press <strong>Play</strong> to start the simulation. Use presets or the frequency slider to feed signals into the neural network.';
    } else if (this.morseMode) {
      const seg = this.morseSignal[this.morseIndex];
      if (seg && seg.freq > 0) {
        const dur = seg.dur;
        text = `<strong>Morse signal active</strong>: ${dur === 50 ? 'Dot' : 'Dash'} (${dur}ms burst at 4500 Hz). ${on1 > 0 ? 'ON1 fires — coincidence detected!' : 'Building up resonance...'}`;
      } else {
        text = '<strong>Morse gap</strong>: Silence between elements. All neurons decaying rapidly. No false spikes.';
      }
    } else if (freq <= 0) {
      text = '<strong>Silence</strong>: No input signal. All neurons undergo rapid decay (amplitude &#xd7; 0.5/step). ON1 output = 0.';
    } else if (Math.abs(freq - 4500) / 4500 <= 0.1) {
      if (on1 > 0) {
        text = `<strong>ON1 fires!</strong> Frequency ${freq} Hz is within the Gaussian tuning window. Coincidence detected: current AND delayed amplitude both exceed threshold.`;
      } else if (an1 > 0.3) {
        text = `<strong>Resonating</strong>: AN1 responds to ${freq} Hz (match &gt; 0.3). Signal propagating through delay lines to ON1. Building coincidence window...`;
      } else {
        text = `<strong>Ramp-up</strong>: ${freq} Hz detected. AN1 amplitude building: ${an1.toFixed(2)}. Needs sustained signal for coincidence gate to open.`;
      }
    } else {
      const dev = (Math.abs(freq - 4500) / 4500 * 100).toFixed(0);
      text = `<strong>No resonance</strong>: ${freq} Hz is ${dev}% away from eigenfrequency (4500 Hz). Outside the &#xb1;10% Gaussian window. AN1 decays.`;
    }

    el.innerHTML = text;
  }
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
  window.cricketSim = new SimulationController();
});
