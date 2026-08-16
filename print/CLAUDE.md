@AGENTS.md

## Machine (coffeeflow / print)

- Prefer **`uv`** for any Python on this machine: `uv run …` or `uv run --with nurb …` for one-off probes that import nurb / `system.py`.
- Do not rely on bare `python` / `python3` — they are often missing or wrong here.
- The `nurb` CLI is already on PATH (`uv tool install nurb`). Use `nurb …` for build/check/dev; use `uv run` only when you need a short Python snippet outside the CLI.
