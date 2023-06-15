WASM maze racing game written in Rust and based off of [Grant Handy's post about ray casting](https://grantshandy.github.io/posts/raycasting/).

## Deployment

Generate a `fly.toml` file by running `fly launch`.

```bash
fly auth login
fly launch
```

This will fail the first time because the port isn't correct.

Modify the experimental section to set the correct internal port.

```toml
[[services]]
  http_checks = []
  internal_port = 8043
```

Then run `fly launch` again and this time it should create a healthy instance.