# Spectre 👻

A scroll trace builder tool. 

## Broken parts:
- mainnet mode
- gui tool

## Getting Started

```bash
~ cargo run --features scroll -- --help
Usage: spectre [OPTIONS] [BUILDER] [OUT]

Arguments:
  [BUILDER]  [default: spectre.toml]
  [OUT]      [default: trace.json]

Options:
      --new      Create a new builder file
  -h, --help     Print help
  -V, --version  Print version
```

See examples folder for how to write a builder file.

```
~ cargo run --release --features scroll -- ./examples/full.toml
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `target/release/spectre ./examples/full.toml`
Loaded Spectre 👻
💳 3 genesis accounts:
- 🔐 0xCafE13B757E6f4E1781CD790cb392Fc796674E10: 💵   100.000000000000000000 Ether | 🔢    0 | 🗄️Empty | </>       Empty
- 👤 0xDeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF: 💵                      0.0 wei   | 🔢    0 | 🗄️Empty | </>    28 bytes
- 👤 0xdEADCAfEDeaDCAfeDeadCafEdEAdcaFEDEAdcAFe: 💵                      0.0 wei   | 🔢    0 | 🗄️    1 | </>    10 bytes

💸 4 transactions:
- legacy    | 0xCafE13B757E6f4E1781CD790cb392Fc796674E10 -> 0x0000000000000000000000000000000000000000 |      1.000000000000000000 Ether
- legacy    | 0xCafE13B757E6f4E1781CD790cb392Fc796674E10 -> 0xDeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF |                       0.0 wei  
- eip2930   | 0xCafE13B757E6f4E1781CD790cb392Fc796674E10 -> 0x0000000000000000000000000000000000000000 |                       0.0 wei  
- eip1559   | 0xCafE13B757E6f4E1781CD790cb392Fc796674E10 -> 0x0000000000000000000000000000000000000000 |                       0.0 wei  

✨  witness dumped in 470.801125ms
                                   
```
