# ProtoCal

```sh
cargo run                       # bot and web server
cargo run -- publish            # slash commands to DISCORD_GUILD_ID
cargo run -- publish --global   # slash commands everywhere
cargo test
```

Settings come from the environment or `.env`:

| Name | Default |
| --- | --- |
| `DISCORD_BOT_TOKEN` | none; without it only the web server runs |
| `DISCORD_APP_ID`, `DISCORD_GUILD_ID` | used by `publish` |
| `DATABASE_URL` | `file:./data/protocal.db` |
| `PROTOCAL_HTTP_HOST` | `127.0.0.1` |
| `PROTOCAL_HTTP_PORT` | `8660` |
| `PROTOCAL_PUBLIC_URL` | `http://localhost:<port>` |
| `PROTOCAL_EDITOR_URL` | `https://eventsl.ink/edit` |
| `PROTOCAL_VIEWER_URL` | `https://eventsl.ink/event` |
| `PROTOCAL_INDEX_VIEWER_URL` | `https://eventsl.ink/index` |
| `LOG_LEVEL` | `info` |

The database is the one the TypeScript version made with drizzle. Opening an older one
renames `meets`, `meet_forums` and `meet_requests` to `events`, `event_forums` and
`event_requests`, and adds `event_forums.review_channel_id`.

The web pages use Lexend 1.007, built into the binary from `assets/fonts/Lexend.woff2` and
served at `/fonts/lexend.woff2`. It is under the SIL Open Font License, in
`assets/fonts/OFL.txt`.
