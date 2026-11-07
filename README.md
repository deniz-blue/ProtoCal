# ProtoCal

ProtoCal is a Discord bot that turns forum posts into events. It runs at https://protocal.tsx.lt.

It is written in Rust with [twilight](https://github.com/twilight-rs/twilight), a Discord library. One process holds the gateway connection, serves the event pages and feeds over HTTP, and ends events on a timer.

## Setting it up in a server

1. Invite the bot with the Add to a server button on its homepage. The link asks for the `bot` and `applications.commands` scopes and these permissions: Manage Roles (it creates the event roles and hands them out), Pin Messages (it pins messages), and Manage Threads (it renames posts when an event's name is edited). It also needs to view channels, send messages in forums and posts, read message history and add reactions.
2. Set a default reaction on each forum that should hold events. The bot uses it as the attend reaction. A post in a forum without one is not made an event.
3. Run `/config` as someone with Manage Server. The link opens the settings page, where you pick the event forums and, for each one, an announce role and a review channel.

## What it does

When a post is made in an event forum, the bot creates a role named `Event: <post name>`. It reacts to the first message with the forum's default reaction and pins that message. It gives the role to the post's author and to anyone who reacted before (the first 100 reactions). Then it posts a note that mentions the forum's announce role, if one is set, and the event role.

Reacting to the first message with the default reaction gives the role. Removing the reaction takes it away.

While an event runs:

- Changing its status, date or place posts a note that pings the event role. Other changes, a rename included, post "The details were updated." without a ping.
- Renaming the post renames the event role.
- A message that pings the event role is pinned. If it replies to a poll or a bot message, that message is pinned as well. A reply that is only the ping pins the message it replies to instead.
- An event ends when its post is locked or deleted, or a day after its last date. Ending deletes the role and removes the event from the feeds. The date check runs at start and every 15 minutes.

### Posting an event

- `/event new` opens a form with Name, Start, End, Place and Description. Start and End take `2026-12-05 18:00` or `2026-12-05`. Times are in Europe/Vilnius unless the value ends in another zone in brackets, such as `2026-12-05 18:00 [Europe/London]`. A form that does not read comes back with the reason and a Fix it button that reopens it with the same values.
- `/event track`, run inside a post someone made by hand, makes that post an event.
- An event the bot posted has an Edit button, and a View button when everyone can see the forum. Edit offers Edit on Discord (the same form) and Edit via App (the editor on eventsl.ink). Organizers and moderators can edit.

### Review

A forum with a review channel is screened. A post made with `/event new` by someone without Manage Threads goes to that channel as a request. Moderators (people with Manage Threads in the forum) can Post, Edit or Decline it. A decline can carry a reason, up to 300 characters, and the host sees it.

The host can edit the request, withdraw it, or send it again after a decline. A request whose event date has passed is removed. A request still waiting is marked as expired first.

Only `/event new` goes through review. A post made directly in the forum starts as an event at once.

### Calendars

`/mycalendar` sends a private link to the events you have the role for and can see. It opens in the eventsl.ink viewer. The link does not expire. `new:true` replaces it, and the old one stops working.

## Commands

| Command | Who can use it | What it does |
| --- | --- | --- |
| `/config` | Manage Server | Private link to the settings page. It works for an hour, and only while you can still manage the server. |
| `/event new [forum]` | Anyone who can post in an event forum | Opens the event form. Without `forum`, you pick the forum from a list. |
| `/event track` | The poster, or a moderator | Makes the post you run it in an event. In a screened forum, moderators only. |
| `/mycalendar [new]` | Anyone, in a server or in a DM with the bot | Private link to your events. `new` replaces the link. |

## Web

The HTTP server serves these paths:

- `/`: the homepage, with the invite link.
- `/events/<thread id>.evnt.json`: one event. Only when everyone can view its forum.
- `/forums/<forum id>.json`: the events of an event forum, as an index. Same rule.
- `/calendars/<token>.json` and `/calendars/<token>/events/<thread id>.evnt.json`: the `/mycalendar` index and the events in it.
- `/config/<token>`, `/edit/<token>` and `/edit/return`: the pages behind the links the bot sends.
- `/fonts/lexend.woff2`: the font the pages use.

Event documents use the Open Evnt format, media type `application/vnd.evnt+json`. The feed routes send `Access-Control-Allow-Origin: *`. Other paths return 404.

## Running

Needs Rust 1.88 or newer, for edition 2024 and let chains.

```sh
cargo build --release
cargo run                       # bot and web server
cargo run -- publish            # slash commands to DISCORD_GUILD_ID
cargo run -- publish --global   # slash commands everywhere
cargo test
```

- The bot does not publish commands when it starts. Run `publish` after changing a command's name, options or description.
- Without `DISCORD_BOT_TOKEN`, the process logs an error and serves only the web pages.
- The web server listens on `127.0.0.1:8660` by default. Put a TLS reverse proxy in front of it and set `PROTOCAL_PUBLIC_URL` to the public address, since the links and feed URLs are built from it.

Settings come from environment variables. A `.env` file is read too, and a variable that is already set wins.

| Name | Default |
| --- | --- |
| `DISCORD_BOT_TOKEN` | none. The bot does not connect without it. `publish` needs it too. |
| `DISCORD_APP_ID`, `DISCORD_GUILD_ID` | Used by `publish` only. `DISCORD_GUILD_ID` is not needed with `--global`. |
| `DATABASE_URL` | `file:./data/protocal.db` |
| `PROTOCAL_HTTP_HOST` | `127.0.0.1` |
| `PROTOCAL_HTTP_PORT` | `8660` |
| `PROTOCAL_PUBLIC_URL` | `http://localhost:<port>` |
| `PROTOCAL_EDITOR_URL` | `https://eventsl.ink/edit` |
| `PROTOCAL_VIEWER_URL` | `https://eventsl.ink/event` |
| `PROTOCAL_INDEX_VIEWER_URL` | `https://eventsl.ink/index` |
| `LOG_LEVEL` | `info`. A log filter, such as `protocal=debug`. |

The database is SQLite at `DATABASE_URL`, and its folder is created at start. The schema is the one the TypeScript version used. On open, the bot renames `meets`, `meet_forums` and `meet_requests` to `events`, `event_forums` and `event_requests` when the new names are not there yet, and adds `event_forums.review_channel_id` if it is missing. Buttons posted under the old `meet-` ids still work.

The web pages use Lexend 1.007, built into the binary from `assets/fonts/Lexend.woff2` and served at `/fonts/lexend.woff2`. It is under the SIL Open Font License, in `assets/fonts/OFL.txt`.
