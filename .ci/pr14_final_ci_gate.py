from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, got {count}")
    return text.replace(old, new, 1)


ci_path = Path(".github/workflows/ci.yml")
ci = ci_path.read_text(encoding="utf-8")
anchor = '''  doc:
    name: Docs
'''
job = '''  regression-gates:
    name: Memory and performance regression
    runs-on: ubuntu-22.04
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Zero-allocation hot-path contract
        run: cargo test -p finkit --test memory_regression --release --locked -- --test-threads=1
      - name: Relative performance contract
        run: cargo test -p finkit --test performance_regression --release --locked -- --test-threads=1
      - name: Compile benchmark suite
        run: cargo bench -p finkit --no-run --locked

'''
ci = replace_once(ci, anchor, job + anchor, "CI regression gate insertion")
ci_path.write_text(ci, encoding="utf-8")

cargo_path = Path("core/Cargo.toml")
cargo = cargo_path.read_text(encoding="utf-8")
old = '''# B1: Memory profiling benchmark — uses dhat-rs to measure peak heap
# allocation per indicator at 1M / 10M scales. Outputs a JSON report
# consumed by `.github/scripts/check_memory_regression.py`.
'''
new = '''# B1: Optional dhat-rs profiling entry point. The blocking allocation
# regression contract lives in `core/tests/memory_regression.rs`, where the
# caller-owned `_into` hot path is required to stay allocation-free.
'''
cargo = replace_once(cargo, old, new, "memory gate documentation")
cargo_path.write_text(cargo, encoding="utf-8")

print("persistent memory/performance CI gates staged")
