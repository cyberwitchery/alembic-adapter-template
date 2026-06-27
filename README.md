# alembic external adapter template

a starting point for building an [alembic](https://github.com/cyberwitchery/alembic)
external adapter in rust. copy this repo, rename the crate, and fill in the three
trait methods.

## what an external adapter is

alembic delegates backend i/o to adapters. an *external* adapter is a standalone
binary that the alembic cli spawns as a subprocess: it reads a single json request
on stdin and writes a single json response on stdout. that boundary means an
adapter never links into the main binary and can target any backend, as long as it
speaks the protocol.

this template uses the rust sdk in `alembic-engine`, which removes the
request/response boilerplate. you implement the `ExternalAdapter` trait; the
`alembic_external_main!` macro generates `main()` and runs the protocol.

the protocol has three methods:

- `read` — observe backend state for a set of types, so the engine can diff it
  against the desired inventory and build a plan.
- `write` — apply a plan's create/update/delete operations.
- `ensure_schema` — optionally provision backend schema (custom fields, types)
  before apply. defaults to a no-op.

see [`docs/external-adapters.md`](https://github.com/cyberwitchery/alembic/blob/main/docs/external-adapters.md)
for the full request/response shapes.

## using this template

1. copy the repo (on github, use the **Use this template** button).
2. rename the package in `Cargo.toml` and the `ExampleAdapter` type in
   `src/main.rs` to your backend.
3. put your connection details on the struct and parse them in `setup`.
4. implement `read` and `write`. if your backend needs schema set up first,
   uncomment and implement `ensure_schema`.

emit-only adapters (the ones that just render an artifact file, like the ops
ansible/dns/ssh adapters) return an empty vec from `read`, so every desired
object becomes a create; `write` then renders the file.

## build, test, run

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

the repo also comes with a ci definition for github.

the built binary is your adapter. wire it into alembic as an external backend
using a config like [`examples/backend.yaml`](examples/backend.yaml):

```bash
alembic plan  --backend external --backend-config examples/backend.yaml \
  -f inventory.yaml -o plan.json
alembic apply --backend external --backend-config examples/backend.yaml \
  -p plan.json
```

you can debug the protocol by hand by piping a request straight into the binary:

```bash
echo '{"version":1,"setup":{},"method":"read","schema":{"types":{}},"types":[],"state":{"mappings":{}}}' \
  | ./target/release/alembic-adapter-example
```

## license

Apache-2.0
