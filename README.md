# srvcs-majority

The majority-vote service of the srvcs.cloud distributed standard library.

Its single concern: **are more than half of the values in a list true?** It is a
**leaf** — it depends on no other service and computes the answer entirely from
the local list of booleans.

```text
count = number of true elements
result = (count * 2) > values.len()
```

So a strict majority is `true`, a **tie** is `false`, and the **empty list** is
`false` (there is no majority of nothing).

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and (empty) dependency list |
| `POST` | `/` | Report whether more than half of `values` are `true` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"values": [true, true, false]}'
# {"values":[true,true,false],"result":true}
```

Responses:

- `200 {"values": [...], "result": bool}` — evaluated.
- `422 {"error": "values must be booleans"}` — an element of `values` is not a
  JSON boolean.

Each element is read with `Value::as_bool`; anything that is not a JSON `true`
or `false` (a string, a number such as `1`/`0`, `null`, an array, an object)
makes the whole request a `422`.

## Dependencies

None. `srvcs-majority` is a leaf: all work is local, so a request fans out to no
other service.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared service
standard and CI workflow.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
