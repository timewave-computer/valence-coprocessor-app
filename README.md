# Valence co-processor app template

This is a template for a co-processor app.

## Requirements

- [Valence domain clients co-processor utils](https://github.com/timewave-computer/valence-domain-clients?tab=readme-ov-file#cli)

## Deploy

#### Build

The build process expects the following toolchains to be installed:

- [wasm32-unknown-unknown](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
- [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)

```shell
valence-coprocessor build
```

#### Nix

Alternatively to manually installing build toolchains, we can use Nix. Notably, we endorse and facilitate Nix builds in our system. Currently, our platform exclusively supports WebAssembly (WASM) builds using Nix. As such, having SP1 installed is a necessity; we however plan to add SP1 Nix support.

```shell
nix develop --command valence-coprocessor build --only-controller
```

#### Publish

Executing this command initiates the build process, which constructs all artifacts listed within the `valence.toml` configuration file. The resulting folder for these artifacts is designated in `valence.artifacts`, and it will house the runtime binary data along with the circuit files tailored for the ZkVM. These constructed elements will be delivered to the co-processor service upon deployment.

```shell
valence-coprocessor deploy
```

This command deploys all circuits defined in the 'valence.toml' file. The circuit IDs are subsequently listed.

```json
[
  {
    "id": "308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278",
    "name": "app"
  }
]
```

## Debug

The Valence Co-Processor Toolkit enables the inspection of pre-computation methods for witness sets in the context of circuit proofs.

```shell
valence-coprocessor witnesses \
  --circuit 308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278 \
  --args '{"value": 42}'
```

The output is the controller logs, and the computed witnesses:

```json
{
  "log": [
    "received a proof request with arguments {\"value\":42}"
  ],
  "witnesses": {
    "proofs": [],
    "root": [222,236,233,6,92,167,87,33,245,88,93,74,74,188,123,188,206,93,80,59,251,190,92,36,98,1,50,145,105,110,104,118],
    "witnesses": [
      {
        "Data": [42,0,0,0,0,0,0,0]
      }
    ]
  }
}
```

## Prove

The Valence Co-Processor Toolkit enables proving a circuit with a given set of arguments.

```shell
valence-coprocessor prove \
  --circuit 308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278 \
  --args '{"value": 42}'
```

The generated artifact is a cryptographically secure proof document suitable for submission to an on-chain verifier.

```json
{
  "domain": {
    "inputs": "3uzpBlynVyH1WF1KSrx7vM5dUDv7vlwkYgEykWluaHY=",
    "proof": "pFlMWRfN3PMSmiEcGYzUgdNxnHVmuwOln72TOouvU0gsIoZeERCKbfy8bA4B27F8o2XIxFP
QOP+KikbfluhH8BnEyT4sLNAY6rAw2Wc22UcIpsQkabZbdDhkxeFdtR5goZMVVRkvDSVKoklJPPYjvRMjo5/8
+NN/Bqk8dCV0tttnjTMVExR9d3Y+QhnwfXDitLsmjebAFcL0+b6+rgjdwQXuF+8iEnx7i7F37SExZ8JLLKrqo
CqSQ9PGWIO9I2mOG0YtDxklwPeCEfgtDf+rZ9JyVsAzxb3o/RxVDAq4+/NdGFNyKNjkhHxe/lhItek6Kdlohh
uyNQZ0izQcXJ2IaZyhopw="
  },
  "program": {
    "inputs": "3uzpBlynVyH1WF1KSrx7vM5dUDv7vlwkYgEykWluaHYrAAAAAAAAAA==",
    "proof": "pFlMWSUQMaQnsht+AgKaa+5QXcP+01iQTwK6mrVC7gaZ67VqEM0F5i0lWKbTiXbJU+oyqPY
cLph78lZ5GY2Ulu3110QSxdy6C4AHGKoUxiah7UXxk6mA8WCH2Fl1OuiD47QSGCQeieUsOc4EFOoqRJs2enYH
UZWe9Z30HP6HwQqCQb8VGz2nCKHEXVhvMqPFxNqBpB9z9gx1iuxYSZnTO8zy9CAJRIhxZv3JGCN72jzG4etAt
0u658nRwCcAyk0REs0n2iWf3P8+I1Fcj7JQdxReR0p5W5kpNxHKeifPZpuiysb2BEH6hcfmQqZErWKFlz6z+x
i7sAEo7xO+WieBxOVbg0E="
  }
}
```
