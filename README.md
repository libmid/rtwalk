Rtwalk is a simple forum application written in rust.

Some features of rtwalk:
- Account creation and email verification.
- Graphql api
- Support for bot accounts
- Real time post/comment updates via websockets
- Multi level comments
- Video/voice channel support using [misosoup](https://github.com/midatindex0/misosoup)

For a frontend, checkout [dreamh](https://github.com/midatindex0/dreamh).
For a python client/bot library check [rtlink](https://github.com/midatindex0/rtlink).

### Usage:

Clone the repo, compile and run it.

```sh
git clone https://github.com/midatindex0/rtwalk
cd rtwalk/
cargo build --release
# Optionally strip the binary
./target/release/rtwalk --host 0.0.0.0 --port 4001
```
To use VC make sure to set `MISOSOUP_URL` environment varianle.
```sh
export MISOSOUP_URL=ws://localhost:4002/
```

Dont provide `--port` to use from `RTWALK_PORT` environment variable. Default `--host` is `127.0.0.1` (Also can be set using `RTWALK_HOST`).

The server starts at `0.0.0.0:4001` or the port/host you provided.

GraphiQL interface at `/` and api interface at `/gql`.
