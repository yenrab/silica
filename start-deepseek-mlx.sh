#!/usr/bin/env bash
set -e

source /Users/leebarney/1TB/mlx-lm-demo/.venv/bin/activate

exec mlx_lm.chat \
  --model mlx-community/DeepSeek-R1-Distill-Qwen-7B-4bit \
  --max-kv-size 4096 \
  --max-tokens 8192
