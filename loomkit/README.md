# Tapestry LoomKit

A Rust-based WebAssembly library for building Loom implementations on top of [`tapestry-weave`](../weave).

## Compiling

Building this library requires [Rust](https://www.rust-lang.org/tools/install) and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

Running `wasm-pack build --release` will build the library and output the compiled package to the `./pkg` directory.

See [the wasm-bindgen documentation](https://rustwasm.github.io/docs/wasm-bindgen/reference/deployment.html) for more information about using the compiled package.

## Licensing

Unlike the rest of Tapestry Loom, this library is licensed under the [Mozilla Public License Version 2.0](./LICENSE).