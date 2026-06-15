setup:
    cargo build
    cd dashboard && pnpm install
    cd python-lab && pip install -r requirements.txt

test:
    cargo test --workspace

paper:
    cargo run -p guardrail-agent -- --config configs/paper.toml

api:
    cargo run -p guardrail-api

