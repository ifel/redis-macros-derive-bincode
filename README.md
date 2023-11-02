# redis-macros-derive-bincode

Simple macros and wrappers to [redis-rs](https://github.com/redis-rs/redis-rs/) to automatically serialize and deserialize structs with serde using [bincode serializer](https://docs.rs/bincode/latest/bincode/). It's the fastest serizalizer/deserializer according to the [test](https://blog.logrocket.com/rust-serialization-whats-ready-for-production-today/). It's 3.5-6 times faster than json.

## Origin
Most of the code of the macros is taken from the [redis-macros/redis-macros-derive](https://github.com/daniel7grant/redis-macros) by Daniel Grant. This crate appeared because the serializer libraries do not have the same interface, and the origin macros uses to_string/from_string functions to do serialization/deserialization which are supported by text based serializators like Json and Yaml.

## Installation

To install it, simply add the package `redis-macros-derive-bincode`. This package is a helper for `redis` and uses `serde` and `bincode`.

```toml
[dependencies]
redis-macros-derive-bincode = "0.1.0"
redis = { version = "0.22.2" }
serde = { version = "1.0.152", features = ["derive"] }
bincode = { version = "1.3.3" }
```

## Basic usage

### Simple usage

The simplest way to start is to derive `Serialize`, `Deserialize`, `FromRedisValue`, `ToRedisArgs` for any kind of struct... and that's it! You can now get and set these values with regular redis commands:

```rust
use redis::{Client, Commands, RedisResult};
use redis_macros_derive_bincode::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Address {
    Street(String),
    Road(String),
}

// Derive the necessary traits
#[derive(Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
struct User {
    id: u32,
    name: String,
    addresses: Vec<Address>,
}

fn main () -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://localhost:6379/")?;
    let mut con = client.get_connection()?;

    let user = User {
        id: 1,
        name: "Ziggy".to_string(),
        addresses: vec![
            Address::Street("Downing".to_string()),
            Address::Road("Abbey".to_string()),
        ],
    };

    // Just use it as you would a primitive
    con.set("user", user)?;
    // user and stored_user will be the same
    let stored_user: User = con.get("user")?;
}
```

For more information, see the [Basic](./examples/derive_basic.rs) or [Async](./examples/derive_async.rs) examples.


### Using deadpool-redis or other crates

You can still use the macros if you are using a crate that reexports the `redis` traits, for example [deadpool-redis](https://github.com/bikeshedder/deadpool). The only change you have to make is to `use` the reexported `redis` package explicitly:

```rust
// In the case of deadpool-redis, bring the reexported crate into scope
use deadpool_redis::redis;

// Or if you are importing multiple things from redis, use redis::self
use deadpool_redis::{redis::{self, AsyncCommands}, Config, Runtime};
```

For more information, see the [deadpool-redis](./examples/derive_deadpool.rs) example.

## Testing

You can run the unit tests on the code with `cargo test`:

```sh
cargo test
```

For integration testing, you can run the examples. You will need a RedisJSON compatible redis-server on port 6379, [redis-stack docker image](https://hub.docker.com/r/redis/redis-stack) is recommended:

```sh
docker run -d --rm -p 6379:6379 --name redis docker.io/redis/redis-stack
cargo test --examples
# cleanup the container
docker stop redis
```

## Coverage

For coverage, you can use `grcov`. Simply install `llvm-tools-preview` and `grcov` if you don't have it already:

```sh
rustup component add llvm-tools-preview
cargo install grcov
```

You have to export a few flags to make it work properly:

```sh
export RUSTFLAGS='-Cinstrument-coverage'
export LLVM_PROFILE_FILE='.coverage/cargo-test-%p-%m.profraw'
```

And finally, run the tests and generate the output:

```sh
cargo test
cargo test --examples
grcov .coverage/ -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
```

Now you can open `./target/debug/coverage/index.html`, and view it in the browser to see the coverage.
