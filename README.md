# aggkit-rust

Proof of concept implementation of aggkit in Rust. Barely tested and not meant for production use.

## Features

Covered features:
- [x] aggbridge
- [x] l1infotree indexer
- [ ] aggsender
- [ ] aggoracle

Important stuff:
* Uses alloy v1.
* Heavily parallelizes event indexing using async tokio.
* Stores bridge exits in a key-value db. All intermediate levels are prehashed, which allows for really fast lookups for merkle proofs of the local exit root and rollup exit root.
* Stores bridge exits in an SQLite database.
* Allows indexing an arbitrary number of chains. Pass as many `--l2-rpc-url` as you like.

## Run

Run as follows. This will index the L1InfoTree and both L1 + L2 (1=PolygonZKEVM) bridges. It does so in around 8 minutes.
```
cargo run -- \
--l1-rpc-url="https://mainnet.gateway.tenderly.co/YOU_API_KEY" \
--l2-rpc-url="1:https://zkevm-rpc.com" \
--l2-rpc-url="20:https://rpc.katanarpc.com"
```

Mainnet addresses are hardcoded, but you can configure `--ger-address`, `--bridge-address` and `--rollup-manager-address`. For help:
```
cargo run -- --help
```

## API Docs


`sync-status`

Check sync status.
```
curl "http://localhost:3000/sync-status"
```

`merkle-proof`

Get Merkle proofs to claim a deposit.
```
curl "http://localhost:3000/merkle-proof?deposit_cnt=15&net_id=20"
```

`bridges`

Get the bridge exits of a given network. Note that the `network_id` parameter is mandatory.

Get all bridge events that started in `network_id=0`. It returns an array with them in `deposits` and `total_cnt`. See paging.
```
curl "http://localhost:3000/bridges?network_id=0"
```

You can use paging as follows. The `deposit_cnt` is used for ordering, ascending.

```
curl "http://localhost:3000/bridges?network_id=0&page_number=1&page_size=2"
```

Get all bridges with a given origin_addr.

```
curl "http://localhost:3000/bridges?network_id=0&dest_addr=0xCE27d8BCee45dB3E457EcF8629264Ca7893AAaAc"
```

All the fields present in the response can be used for filtering.