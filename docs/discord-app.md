# The Discord application

`/jbotci` runs the same analyses the CLI and the web app run and shows them as
Discord messages. This describes what the deployment needs, what the service
will spend on one request, and how the command is registered, changed and
rolled back.

## What a reader sees

One slash command with a subcommand per tool: `gentufa`, `vlasei`, `vlatai`,
`vlacku`, `cukta`, `jvozba`, `gimfihi`. The command publishes a result at once
and puts a single ⚙️ button beside it. That button opens a form holding the
input and the settings; submitting it edits the same message. Where the tool
has a web page (gentufa, vlacku, cukta, gimfi'i), the form's first line is an
**Open in app** link carrying the exact state; the other three tools have no
page, so their forms have no link and no substitute.

Only the reader who ran the command can change what the message shows. Anyone
can open the form to read the settings and use its link; submitting it as
someone else explains that privately and changes nothing.

## Configuration

| Variable | Required | Meaning |
| --- | --- | --- |
| `DISCORD_PUBLIC_KEY` | yes, to serve interactions | The application's Ed25519 public key, hex. Without it `/discord` answers 503 and nothing else is affected. |
| `DISCORD_APPLICATION_ID` | to register | The application's id. |
| `DISCORD_BOT_TOKEN` | to register | Sent as `Authorization: Bot …`; never needed to serve interactions. |
| `DISCORD_API_BASE` | no | Discord's API root, `https://discord.com/api/v10` by default. Tests point it at their own server. |
| `JBOTCI_PUBLIC_BASE_URL` | no | The web app's public root for "Open in app" links, `https://jbotci.app` by default. |

The interaction endpoint is `POST /discord`. Every request must carry
`X-Signature-Ed25519` and `X-Signature-Timestamp`. The signature covers the
body, so the body is read and then verified before it is parsed as JSON or
dispatched to anything; an unsigned or wrongly signed request is refused with
401 at that point, which is what Discord's own endpoint check expects.

## Registering the command

    jbotci-server setup --discord-commands [--guild <GUILD_ID>] [--dry-run]

`--dry-run` prints the registration and calls nothing, so a change can be read
before it is sent. `--guild` registers in one guild, which takes effect at once
and is the way to try a schema change; without it the command is registered
globally.

Registration is a bulk overwrite: what is sent replaces the command entirely.
The payload therefore states the installation contexts as well as the
schema — `integration_types` `[0, 1]` (guild and user installs), `contexts`
`[0, 1, 2]` (guild, direct message with the app, and other private channels).
Discord documents these as defaulting to the application's configured
contexts; stating them pins the intended availability to this repository
rather than to a setting someone may change in the developer portal.

**Rollback.** A command schema is replaced, never migrated: registering the
previous build's binary with the same command restores the previous schema.
Published messages are unaffected, since every message carries its own state
and this build reads it back; a message published by a newer build carries a
newer build tag, and an older build refuses settings it does not recognize
rather than guessing at them. When a result is recomputed by a build other
than the one that published it, the edited message says so, because its text
and its image are then the work of a different version.

To roll back: deploy the previous image, then run `setup --discord-commands`
from it. Global and guild registrations are separate lists, and a guild
command shadows the global one of the same name: registering globally leaves a
guild command exactly as it was. Removing one means deleting that guild
command through the API, or overwriting that guild's command list without it;
this application's `setup --discord-commands --guild` always writes its one
command, so it cannot be used to empty a guild.

## What one request may spend

The service answers within Discord's deadlines: three seconds to acknowledge,
fifteen minutes to finish the message. Work is admitted on three lanes,
analysis, network and meaning search, each with a fixed number of workers and
a bounded queue; a request that cannot be admitted is refused with a message
saying so, rather than queued behind everything else. Work that has started
keeps its worker, its delivery and, while it writes, the message it is
writing, until it ends: a caller that stops waiting never releases any of
them, and a meaning search holds them across a cold model load too. One
deadline covers a whole operation, so waiting for a lane and then talking to
Discord share it rather than each being given it whole.

| Bound | Value | Why |
| --- | --- | --- |
| Source field | 4000 UTF-16 units | Discord's own text-input maximum, enforced at the command and at the form alike. |
| Message text | 4000 units across all text components | The application's budget for one message; longer results become an excerpt plus a complete attachment. |
| Result page | 5 results | What reads well on a phone. |
| Pages | 25 (23 for vlacku) | The page selector holds 25 choices; vlacku's shares its selector with two detail choices. |
| App link | 4000 units for the whole link component | The application's own budget for one form text component, not a documented Discord limit. |
| Compound construction | 8192 part placements, 24 pieces, 256 letters | Measured: 4096 placements take about 1.2s and 9216 about 3.1s in release on the development machine. |
| Diagram | 600 blocks, 160 columns, 16 megapixels, 8 MiB | A diagram larger than this is refused with its reason, and the text result still publishes. |
| Attachment | the interaction's own limit, at most 10 MiB | Discord states a per-interaction limit; the smaller of the two applies. |
| Reporting | 5 seconds | Saying what happened is not the work, so it has its own budget: a request that spent all of its own can still report that. |

A source too long to travel inside the message travels as `jbotci-input.txt`
beside it, and is read back when the form opens. A source that cannot travel
exactly is refused rather than published in part, because a message that
cannot rebuild its request cannot reopen its form. The same holds for the
"Open in app" link: a request whose link would not fit the form is refused
before anything is published, so every published result of a tool with a page
has an exact link. Both refusals are final and carry no ⚙️ form, since there
would be nothing there to correct them with.

These two bounds are different things, and a source can pass the first and
fail the second: the link carries the same source percent-encoded inside a
route, which is longer than the source itself, and longer again for text
outside ASCII. A request whose link does not fit is refused with the size its
link would need, so the reader can see how much to cut. The alternative would
be to advertise a source limit low enough that every encoding fits, which
would refuse most requests that are perfectly fine.

A command that cannot produce a result still answers. Discord shows a thinking
indicator from the moment the command is acknowledged, and only editing that
message takes it away, so a failure becomes the message: what happened, and
the ⚙️ form to change the request where the request can still be carried.
Failures of a form submission behave the other way around: the published
result stays exactly as it was, and the reader is told privately, because
losing a result to a typo would be worse than not applying the change.

A tool that ran and found the input wanting is not a failure: that is a
result, showing the diagnostics it produced. An image that could not be
rendered (too complex, too large, or larger than this interaction accepts) is
an operational failure of the image alone: the text result still publishes and
says why there is no image.

## How a message remembers its request

The ⚙️ button's identifier carries the tool, the packed settings, the page, the
revision, the reader who asked, the build tag and a digest of the source. The
input block beside it carries the source fields themselves, escaped so Discord
shows them as written and the encoder can read them back exactly. No published
request is kept in process-local storage: the server does hold queues, recent
deliveries and its embedding model in memory, but none of them are needed to
reopen a form. After a restart, with every cache gone, the form opens on the
state the message shows.

Each edit raises the revision. A form opened before someone else's edit is
refused with an explanation rather than applied, and a submission is checked
against the message as Discord currently holds it, under a per-message lock. A
write whose answer never arrives is settled by reading the message back and
comparing what it shows.
