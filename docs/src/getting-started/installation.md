# Installation

## Rust

Add alembic-rs to your `Cargo.toml`:

```toml
[dependencies]
alembic = "0.1"
```

### Feature Flags

- `python` - Enable Python bindings (requires PyO3)

```toml
[dependencies]
alembic = { version = "0.1", features = ["python"] }
```

## Python

### From PyPI

```bash
pip install alembic_rs
```

### From Source

1. Clone the repository:
```bash
git clone https://github.com/your-repo/alembic-rs
cd alembic-rs
```

2. Build with maturin:
```bash
pip install maturin
maturin build --release
pip install target/wheels/alembic_rs-*.whl
```

### Development Installation

For development with editable install:

```bash
maturin develop --release
```

## Verifying Installation

### Rust

```rust
use alembic::abc::IArchive;

fn main() {
    println!("alembic-rs is installed!");
}
```

### Python

```python
import alembic_rs
print(f"alembic_rs version: {alembic_rs.__version__}")
```

## System Requirements

- **Rust**: 1.70 or later
- **Python**: 3.8 or later (for Python bindings)
- **Platform**: Windows, macOS, Linux
