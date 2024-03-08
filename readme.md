# rust web server
A blank web server with rust language. Support mysql, redis, restful api, quickwit.

## Introduction

https://medium.com/@raditzlawliet/build-crud-rest-api-with-rust-and-mysql-using-axum-sqlx-d7e50b3cd130

```
# Depedency
cargo add axum
cargo add tokio -F full
cargo add tower-http -F "cors"
cargo add serde_json
cargo add serde -F derive
cargo add chrono -F serde
cargo add dotenv
cargo add uuid -F "serde v4"
cargo add sqlx --features "runtime-async-std-native-tls mysql chrono uuid"

cargo install cargo-watch

# CLI For migration
cargo install sqlx-cli

# create a migration
sqlx migrate add -r create_notes_table


# perform migration up
sqlx migrate run

# (Bonus!, perform migration down/revert)
sqlx migrate revert
```

## How to run

```sh
cargo watch -x run 
or 
cargo watch -q -c -w src/ -x run

```

## How to Debug

