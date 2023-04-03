# MuleSoft Flex Gateway Custom Policy Example that change status code
Example project showing how to create a custom policy for MuleSoft Flex Gateway that change status code


# Setup

Add `wasm32` as a compilation target:
```
rustup target add wasm32-unknown-unknown
```

Compile the custom policy with this command:
```
cargo build --target wasm32-unknown-unknown --release
```

Install `wasm-gc` if you don't already have it installed. `wasm-gc` removes unneeded exports, imports, and functions to reduce the size of the final binary file.
```
cargo install wasm-gc
```

Run the optimization by executing the following command. This is the file that you need to publish to Exchange.
```
wasm-gc target/wasm32-unknown-unknown/release/flex_custom_policy.wasm -o target/flex_custom_policy-final.wasm
```

