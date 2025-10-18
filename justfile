default:
    just --list

dev:
    cargo run --features z3_gh_release

release:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    cargo run --features z3_bundled --release

pydev:
    maturin develop --features pyo3/extension-module,z3_gh_release

pyrelease:
    #!/bin/bash

    export CFLAGS="-O3 -mtune=native -march=native -flto -fPIC"
    export CXXFLAGS="-O3 -mtune=native -march=native -flto -fPIC"

    maturin build --features pyo3/extension-module,z3_bundled --release

clean:
    cargo clean

