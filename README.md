Rtwalk is a simple forum application written in rust.

Some features of rtwalk:
- Account creation and email verification.
- Edit account, forum, post, comment after creation.
- Media upload.
- Support for global admin and forum moderator (can be bot).
- User created forum.
- Full text search.
- Graphql api
- Support for bot accounts
- Real time post/comment updates via websockets
- Multi level comments
- Video/voice channel support using [misosoup](https://github.com/midatindex0/misosoup)

For a frontend, checkout [dreamh](https://github.com/midatindex0/dreamh).
For a python client/bot library check [rtlink](https://github.com/midatindex0/rtlink).

### Usage:

#### Required environment variables:
- `MONGODB_URL`
- `REDIS_URL`
- `COOKIE_KEY` 64 bytes base64 encoded string
- `MISOSOUP_URL` (optional) to use VC.

Clone the repo, compile and run it.

```sh
git clone https://github.com/midatindex0/rtwalk
cd rtwalk/
cargo build --release
# Optionally strip the binary
./target/release/rtwalk --host 0.0.0.0 --port 4001
```

The server starts at `127.0.0.1:4001` by default.

GraphiQL interface at `/` and api at `/gql`.
