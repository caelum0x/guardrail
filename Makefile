setup:
	cargo build
	cd dashboard && pnpm install
	cd python-lab && pip install -r requirements.txt

test:
	cargo test --workspace
	cd dashboard && pnpm test || true

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

paper:
	cargo run -p guardrail-agent -- --config configs/paper.toml

live:
	cargo run -p guardrail-agent -- --config configs/production.toml

backtest:
	cargo run -p guardrail-cli -- backtest --config configs/backtest.toml

dashboard:
	cd dashboard && pnpm dev

register:
	./scripts/register_agent.sh

kill:
	./scripts/kill_switch.sh

api:
	cargo run -p guardrail-api

monitor:
	cargo run -p guardrail-monitor

exporter:
	cargo run -p guardrail-exporter

replay:
	cargo run -p guardrail-replay -- journal

stack-up:
	docker compose up -d --build

stack-down:
	docker compose down

metrics:
	curl -fsS http://127.0.0.1:9100/metrics

