import init, { Brain } from "../../crates/wasm/pkg/cricket_brain_wasm.js";

async function boot() {
  await init();
  const brain = new Brain(1337n);

  const stepBtn = document.getElementById("step")!;
  const silenceBtn = document.getElementById("silence")!;

  const tick = (freq: number) => {
    const out = brain.step(freq);
    const pred = brain.latestPrediction();
    const events = brain.drainTelemetry();
    console.log({ freq, out, pred, events, step: brain.time_step() });
  };

  stepBtn.addEventListener("click", () => tick(4500));
  silenceBtn.addEventListener("click", () => tick(0));
}

boot();
