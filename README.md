# zpack

A fast, configurable module system that builds packages from source.

Build using Rust.

## Example Usage

Goal: install HPL with OpenMPI and OpenBLAS, all using GCC.

```sh
zpack install hpl %openblas %openmpi +fabrics=auto +lto %%gcc
```

Why? We want HPL. HPL requires a BLAS implementation, which is provided by
OpenBLAS. OpenMPI will act as the compiler for HPL and OpenBLAS but requires a
base compiler of its own, for which we will use GCC.

If you mistyped the command with `%gcc` rather than `%%gcc`, you would see an
error informing you that you have duplicate compiler definitions
