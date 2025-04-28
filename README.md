# Valence co-processor app template

This is a template for a Valence app.

### Structure

#### `./core`

This primary definition serves as a common functionality between the circuit and the library. Maintaining its separation from both the library and circuit definitions facilitates easier unit testing of the implementation.

#### `./lib`

This library is meant for deployment onto the Registry VM. Within it, you'll find pre-built functions like "get_witnesses" and "entrypoint".

#### `./zkvm`

This is the circuit specification to be proven by the zkVM for the generation of a verifiable proof, along with the transition function.

#### `./script`

This is a CLI application that will act as helper for deployment, proving, and verification.

### Executing

First, on another terminal, start the co-processor.

```sh
git clone https://github.com/timewave-computer/valence-coprocessor.git $HOME/valence-coprocessor
git -C $HOME/valence-coprocessor checkout v0.1.4
cargo run --manifest-path $HOME/valence-coprocessor/Cargo.toml -p valence-coprocessor-service --profile optimized
```

You should see as output the service initialized.

```
INFO config file loaded from `/home/joe/.config/valence-coprocessor-service/config.toml`...
INFO db loaded from `/home/joe/.local/share/valence-coprocessor-service`...
INFO registry loaded...
INFO API loaded, listening on `0.0.0.0:37281`...
INFO listening addr=socket://0.0.0.0:37281
INFO server started
INFO execute: clk = 0 pc = 0x2092e0
INFO execute: gas: 1020123
INFO execute: clk = 0 pc = 0x2092e0
INFO execute: gas: 1020123
```

Then, build the demo.

```sh
cargo run -- deploy
```

You should see the program id as output:

```
{
  "program": "bd755a6103c46f20b0fc9f69c4871ef4f2e159998ee6c09579dbf2d4f59bed58"
}
```

To generate a proof:

```sh
echo '{"name": "Valence"}' | cargo run -- prove
```

You should see the proof as output:

```
{
  "log": [
    "received name: `Valence`",
    "received advice: `When having a clear out, ask yourself if an item has any financial, practical or sentimental value. If not, chuck it.`",
    "computed message: `Hello, Valence! Here is an advice for you: When having a clear out, ask yourself if an item has any financial, practical or sentimental value. If not, chuck it.`"
  ],
  "outputs": "oAAAAAAAAABIZWxsbywgVmFsZW5jZSEgSGVyZSBpcyBhbiBhZHZpY2UgZm9yIHlvdTogV2hlbiBoYXZpbmcgYSBjbGVhciBvdXQsIGFzayB5b3Vyc2VsZiBpZiBhbiBpdGVtIGhhcyBhbnkgZmluYW5jaWFsLCBwcmFjdGljYWwgb3Igc2VudGltZW50YWwgdmFsdWUuIElmIG5vdCwgY2h1Y2sgaXQu",
  "proof": "AAAAAAAAAAAAAAAAqAAAAAAAAACgAAAAAAAAAEhlbGxvLCBWYWxlbmNlISBIZXJlIGlzIGFuIGFkdmljZSBmb3IgeW91OiBXaGVuIGhhdmluZyBhIGNsZWFyIG91dCwgYXNrIHlvdXJzZWxmIGlmIGFuIGl0ZW0gaGFzIGFueSBmaW5hbmNpYWwsIHByYWN0aWNhbCBvciBzZW50aW1lbnRhbCB2YWx1ZS4gSWYgbm90LCBjaHVjayBpdC4LAAAAAAAAAHY0LjAuMC1yYy4zAA=="
}
```

To verify a proof:

```sh
echo '{"name": "Valence"}' | cargo run -- prove | cargo run -- verify
```

You should see the generated outputs:

```
{
  "log": [
    "received name: `Valence`",
    "received advice: `If you've nothing nice to say, say nothing.`",
    "computed message: `Hello, Valence! Here is an advice for you: If you've nothing nice to say, say nothing.`"
  ],
  "output": "Hello, Valence! Here is an advice for you: If you've nothing nice to say, say nothing."
}
```

### Requirements

- [Rust](https://www.rust-lang.org/tools/install)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- WASM toolchain

```sh
rustup target add wasm32-unknown-unknown
```

- [clang and llvm](https://clang.llvm.org/get_started.html)

```sh
sudo pacman -S clang llvm
```

Note: openssl-dev might also be required.

- [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)

```sh
curl -L https://sp1up.succinct.xyz | bash
sp1up
cargo prove --version
```
