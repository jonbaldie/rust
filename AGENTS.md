# AGENTS.md

## Cursor Cloud specific instructions

This is the `rust-lang/rust` monorepo (the compiler `rustc`, the standard
library, `rustdoc`, and bundled tools). The build system is the Python wrapper
`x.py`; see `INSTALL.md` and the [rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/)
for the canonical workflow. Standard commands below are not duplicated
elsewhere only where the cloud environment needs a non-obvious caveat.

### Build configuration (`bootstrap.toml`)

- `bootstrap.toml` is git-ignored user config and is required for builds. This
  environment uses the `compiler` profile with `llvm.download-ci-llvm = true`,
  so CI-built LLVM is downloaded instead of compiled from source (the
  `x86_64-unknown-linux-gnu` host is tier 1). Never enable building LLVM from
  source here unless you specifically need it — it is extremely slow.
- Non-obvious gotcha: the system `cc`/`c++` alternatives point at **clang 18**,
  which selects a GCC toolchain root that is missing the libstdc++ dev
  headers/libs. That causes `fatal error: 'iostream' file not found` (building
  the `libcxx-version` shim) and `rust-lld: error: unable to find library
  -lstdc++` (linking `rustc_driver`). `bootstrap.toml` therefore pins
  `cc = "gcc"`, `cxx = "g++"`, and `linker = "gcc"` under
  `[target.x86_64-unknown-linux-gnu]`. If you regenerate `bootstrap.toml`
  (e.g. via `x.py setup`), re-add those three pins or the build will break.

### Building / running the compiler

- `./x.py build` builds the stage1 `rustc` + std (the default). The compiler
  lands at `build/host/stage1/bin/rustc` and is linked into rustup as the
  `stage1` toolchain (`rustup toolchain link stage1 build/host/stage1`).
- `cargo` is **not** part of the stage1 sysroot, so `cargo +stage1 ...` fails.
  To compile a crate with the from-source compiler, drive an existing cargo
  with `RUSTC=/workspace/build/host/stage1/bin/rustc cargo build`, or invoke
  `build/host/stage1/bin/rustc` directly.
- Incremental compilation is on; re-run `./x.py build` after edits. `./x.py
  check` is the fast compile-only smoke test.

### Lint and tests

- Lint/style is `./x.py test tidy` — this also runs the `rustfmt` check
  (`./x.py fmt --check`) and downloads a pinned nightly rustfmt on first run.
- Run test suites with `./x.py test <path>` (defaults to stage1), e.g.
  `./x.py test tests/ui/entry-point`. The full `tests/ui` suite has ~22k tests;
  scope to a subdirectory or specific file when iterating.

### Environment notes

- First `./x.py build` downloads the stage0 beta compiler and CI LLVM into
  `build/` (network required); these are cached for subsequent builds.
- `x.py` manages the needed git submodules automatically during build.
