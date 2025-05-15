# langbench

Tool to run benchmarks on different languages.

## Results

You can find reports [here](reports/README.md).

## Install

### With `uv` (recommended)

```bash
uv venv
source .venv/bin/activate
uv pip install -e .
```

### With `pip`

```bash
python -m venv venv
source .venv/bin/activate
pip install -e .
```

## Run

```bash
langbench generate-data
langbench run
```
