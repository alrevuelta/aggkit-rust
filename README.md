# aggkit-rust

Proof of concept implementation of aggkit in Rust. Barely tested and not meant for production use.

Covered features:
- [x] aggbridge
- [ ] aggsender
- [ ] aggoracle

Important stuff:
* Uses alloy v1.
* Heavily parallelizes event indexing using async tokio.
* Stores bridge exits in a key-value db. All intermediate levels are prehashed, which should allow for really fast lookups.
* Allows indexing an arbitrary number of chains. Pass as many `--l2-rpc-url` as you like.


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

Check sync status.
```
curl "http://localhost:3000/sync-status"
```


Get Merkle proofs to claim a deposit.
```
curl "http://localhost:3000/merkle-proof?deposit_cnt=15&net_id=20"
```