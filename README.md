<picture>
  <img width="175" alt="The logo for whamm!. Shows a spice jar with the WebAssembly logo, but with the 'h' and 'mm' letters written in between the 'wa' to spell 'whamm'."  src="/docs/logos/whamm!_logo.png">
</picture>

# whamm! #
![build](https://github.com/ejrgilbert/whamm/actions/workflows/rust.yml/badge.svg)

## Debugging Wasm? Put some `whamm!` on it! ##

`whamm!` is a tool for "Wasm Application Monitoring and Manipulation"[^1], a DSL inspired by the D language.

[^1] The 'h' is silent.

## Tutorials ##

To run basic build:
```shell
cargo build
```

To run tests:
```shell
cargo test
cargo test parser # Only run the tests for the `parser` module
cargo test -- --nocapture # With stdout tracing
```

To run project (there are example Whammys in `tests/whammys` folder):
```shell
cargo run -- instr --app <path_to_app_wasm> --whammy <path_to_whammy> <path_for_compiled_output>
```

To specify log level:
```shell
RUST_LOG={ error | warn | info | debug | trace | off } cargo run -- --app <path_to_app_wasm> --whammy <path_to_whammy> <path_for_compiled_output>
```

To visually debug the decision tree used during Wasm bytecode emission:
```shell
cargo run -- vis-tree --whammy <path_to_whammy>
```

## Available Packages ##

NOTE: There was discussion for moving the probe `mode` to the front of the specification (e.g. `mode:provider:package:event`);
however, after thinking through this, I don't think it makes sense until I have a firmer grasp on the types of modes we will
have in this language. If there are more than before/after/alt (that are event-specific), then it would be confusing from a
language-intuition perspective. This is primarily because reading through the spec implies a movement from higher-to-lower
levels of granularity, everything being provided by what proceeds it. If we were to move `mode` to the front, but then have
event-specific options, this property would no longer hold.

Currently available: 
- `wasm:bytecode`

To be added:
- `thread` operation events
- `gc` operation events
- `function` enter/exit/unwind events
- `memory` access (read/write) events
- `table` access (read/write) events
- `component` operation events
- `BEGIN`/`END` events
- `traps`
- `exception` throw/rethrow/catch events

Example:
`wasi:http:send_req:alt`
`wasm:bytecode:call:alt`
`wasm:fn:enter:before`