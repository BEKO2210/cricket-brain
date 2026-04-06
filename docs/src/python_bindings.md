# Python Bindings

Python bindings are provided via **PyO3 + maturin** in `crates/python`.

## Build / Install (editable)

```bash
cd crates/python
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
maturin develop
```

## Usage

```python
import cricket_brain
cfg = cricket_brain.BrainConfig()
brain = cricket_brain.Brain(cfg)
print(brain.step(4500.0))
```

See `examples/python_sentinel.py` for a complete script.
