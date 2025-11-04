DEFAULT_DEV_ARGS := "--features z3_gh_release"
DEFAULT_RELEASE_ARGS := "--features z3_bundled"

default:
    just --list

full: clean doc test-dev dev py-dev test-release release py-release bench install-dev install-release

doc extra-args=DEFAULT_RELEASE_ARGS:
    cargo doc {{ extra-args }}

dev extra-args=DEFAULT_DEV_ARGS:
    cargo run {{ extra-args }}

release extra-args=DEFAULT_RELEASE_ARGS:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    cargo run --release {{ extra-args }}

bench extra-args=DEFAULT_RELEASE_ARGS:
    cargo bench --benches {{ extra-args }}

test-dev extra-args=DEFAULT_DEV_ARGS:
    cargo test {{ extra-args }}

test-release extra-args=DEFAULT_RELEASE_ARGS:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    cargo test --release {{ extra-args }}

py-dev extra-args=DEFAULT_DEV_ARGS:
    maturin develop --features pyo3/extension-module {{ extra-args }}

py-release extra-args=DEFAULT_RELEASE_ARGS:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    maturin build --release --features pyo3/extension-module {{ extra-args }}

install-dev \
    cargo-extra-args=DEFAULT_DEV_ARGS \
    pip-extra-args=DEFAULT_DEV_ARGS:
    #!/bin/bash

    cargo install --path . {{ cargo-extra-args }}

    export MATURIN_PEP517_ARGS="--features pyo3/extension-module {{ pip-extra-args }}"
    pip install . -v

install-release \
    cargo-extra-args=DEFAULT_RELEASE_ARGS \
    pip-extra-args=DEFAULT_RELEASE_ARGS:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    cargo install --path . {{ cargo-extra-args }}

    export MATURIN_PEP517_ARGS="--features pyo3/extension-module {{ pip-extra-args }}"
    pip install .  -v

clean:
    cargo clean

