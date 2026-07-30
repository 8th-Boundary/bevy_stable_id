# bevy_stable_id

`bevy_stable_id` provides one small persistent identifier shared by independent
Bevy ecosystem crates.

`StableId` is a compact 126-bit value with:

- a fixed 21-character URL-safe text representation
- strict `u128` conversion
- optional random generation
- deterministic namespaced derivation
- optional human-readable and binary Serde support

It identifies one durable instance. It is not:

- a Bevy `Entity`, which is valid only in one runtime world
- a Rust `TypeId`, which identifies a Rust type in one process
- a semantic `Kind`, which identifies a data or gameplay concept

```rust
use bevy_stable_id::StableId;

let deterministic = StableId::derive(StableId::NIL, b"world/settlement/market");

assert_eq!(
    deterministic.to_string().parse::<StableId>().unwrap(),
    deterministic
);
```

Runtime entity maps, ECS components, save-file ownership, and collision policy
remain consumer-owned. A cognition system can maintain only `StableId → Entity`,
while an editor history system may keep additional indices for lifecycle
bookkeeping.

The text and deterministic derivation algorithms are persistence contracts and
will not change within a compatible release line.

## License

Licensed under either Apache-2.0 or MIT, at your option.
