// Package definition returned by package
//  - Version
//  - Compiler (what about python, rust, R, etc.?)
//      - Just a package everything depends on?
//      - Conflicts with 'compiler'?
//
// Package returns a list of commands to run?
//
// Importable modules for predefiend packages?
//   - make
//   - CMake
//   - Python
//   - Cargo

// Two phase build process:
// 1. Planning Packages process their options and return a planning object of
//    some sort, which details what options are/are not available.
//
//      For example, the object may specify that a version >= 3.14 is required
//      and that OpenBLAS, OpenMPI and LLVM are required for the program to
// build
// 2. Building Packages provide the build steps necessary to compile based on
//    the planned.
//
//      For example, a package could return a set of CMake commands to compile
//      the program with BLAS and MPI support.

// pub mod spec;

pub mod constraint;
pub mod outline;
pub mod version;
