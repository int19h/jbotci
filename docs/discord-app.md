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
has a web page (gentufa, vlacku, cukta, gimfihi), the form's first line is an
**Open in app** link carrying the exact state; the other three tools have no
page, so their forms have no link and no substitute.

A result that is a list carries **Previous** and **Next** beside it. They turn
the page on the same message, and each page says which results it shows —
`6-10`, or `6-10 of 38` where the number of results is genuinely known. It is
known when the search has reached the end of its results, and when it counted
them all as part of its work; a search that was asked for one page and handed
one back knows nothing about the rest, and then the page says only what it
shows rather than inventing a figure. A direction that does not exist is greyed
rather than hidden, and a result with a single page carries no buttons at all.

Paging goes as far as the results do: there is no page ceiling. Each page is
fetched as itself, in the way its search allows. A word search of the
dictionary or of the book goes straight to the page's first result and copies
only what the page holds. A meaning search ranks up to the end of the page — a
similarity ranking has no way to start in the middle — but what it ranks is a
number and a score each, and only the page's own results are read out of the
dictionary or the book. The gismu search scores every candidate it generates,
keeps the best of them as a word and a score, and works out the full detail of
one page of them. So the expensive half of the work — the cards, the book's
text, a candidate's detail — is one page's worth however deep the page is, and
none of these three has a last page other than the one the results end on. A parse, a word report and a compound are one result each and
have no pages.

Only the reader who ran the command can change what the message shows. Anyone
can open the form to read the settings and use its link; submitting it as
someone else explains that privately and changes nothing.

### Building a word

`jvozba` takes its pieces in one field, in the order they should appear. A
piece written plainly is a word to look up; a piece between hyphens is a rafsi
used exactly as given:

    blanu -blo- zdani

Words are found by the morphology parser, not by splitting on spaces, so
`lojbobangu` is the cmavo `lo` followed by `jbobangu`. A hyphenated piece must
hold one rafsi and no spaces; an unclosed hyphen, an empty pair and text that
is not Lojban are each refused with what is wrong. A message published before
this syntax reopens as the same build, with its old fixed rafsi written after
the words as `-kla-`, which is where the previous version put them.

### Reading a parse

The tree view writes an elided terminator between slashes, as `/ku/` and
`/vau/`, the way the reference grammar writes it. A parse that dropped input
during recovery still says so.

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

**Rollback between builds that both have this application.** A command schema
is replaced, never migrated: deploy the previous image and run its own
`setup --discord-commands`, which writes the schema that build understands.
Published messages keep working as far as the two schemas agree: every message
carries its own state, and a build that meets a setting it does not recognize
says so and refuses rather than guessing. A result recomputed by a build other
than the one that published it says so in the edited message, because its text
and its image are then the work of a different version.

**Rollback across this cutover.** The build before this one has no
`setup --discord-commands`: the command is introduced here. Rolling back to it
means deploying that image and restoring its registration another way — the
registration saved before the upgrade, replayed as a bulk overwrite
(`PUT /applications/{id}/commands`), or the registration script belonging to
that exact older revision. Keep a copy of the current registration
(`GET /applications/{id}/commands`) before upgrading, so there is something to
restore. Messages published by this application do not survive that rollback
as working results: the older endpoint cannot answer a ⚙️ button or a form at
all, so those controls stop working and the reader has to run the command
again on whichever version is deployed.

**Guild and global registrations** are separate lists, and a guild command
shadows the global one of the same name: registering globally leaves a guild
command exactly as it was. Removing one means deleting that guild command
through the API, or overwriting that guild's command list without it; this
application's `setup --discord-commands --guild` always writes its one command,
so it cannot be used to empty a guild.

## What one request may spend

The service answers within Discord's deadlines: three seconds to acknowledge,
fifteen minutes to finish the message. Work is admitted on three lanes,
analysis, network and meaning search, each with a fixed number of workers and
a bounded queue; analysis runs one job at a time, which is what the deployed
instance's memory pays for (measured below); a request that cannot be admitted
is refused with a message saying so, rather than queued behind everything
else. Work that has started
keeps its worker, its delivery and, while it writes, the message it is
writing, until it ends: a caller that stops waiting never releases any of
them, and a meaning search holds them across a cold model load too. One
deadline covers a whole operation, so waiting for a lane and then talking to
Discord share it rather than each being given it whole.

| Bound | Value | Why |
| --- | --- | --- |
| Source field | 4000 UTF-16 units | Discord's own text-input maximum, enforced at the command and at the form alike. |
| Message text | 4000 units across all text components | The application's budget for one message; longer results become an excerpt plus a complete attachment. |
| Result page | 5 results | What reads well on a phone. Pages themselves are unbounded: each one is fetched when it is asked for. |
| App link results | 116 | What the "Open in app" link asks the web app to show. Discord's own paging is not bounded by it: a reader who has paged past it opens the app on the same search from its beginning. |
| App link | 4000 units for the whole link component | The application's own budget for one form text component, not a documented Discord limit. |
| Candidate ranking | one word and one score per generated candidate that passes the filters | What a gismu page costs while it is being worked out, measured below. |
| Compound construction | 8192 part placements, 24 pieces, 256 letters | Measured: 4096 placements take about 1.2s and 9216 about 3.1s in release on the development machine. |
| Diagram | 600 blocks, 160 columns, 8 megapixels, 8 MiB | A diagram larger than this is refused with its reason, and the submission that asked for it changes nothing. Eight megapixels is what one image may spend of the instance's memory, measured below. |
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
result, and it shows the diagnostics it produced. An image that could not be
rendered (too complex, too large, or larger than this interaction accepts) is
an operational failure of the whole submission: the message keeps the result
it had, unchanged, and the reader is told privately why the image was refused,
so they can turn it off or shorten the text and submit again. Only a form can
ask for an image, so this never arises on a first publication.

## What it costs to run

Measured on the development machine against the release binary, driving signed
interactions through the real endpoint with a local stand-in for Discord. Peak
is the kernel's own high-water mark for the process (`VmHWM`), so it is the
true peak rather than a sample.

That machine is Linux on aarch64; the service is deployed on AMD64. Compiled
code, allocation sizes and the allocator's behaviour all differ between the
two, so these figures are the shape of the cost and the reason for each limit,
not a measurement of the deployment. Nothing here has been measured on the
deployed instance.

| After | Resident | Peak |
| --- | --- | --- |
| Start, nothing asked | 42 MiB | 42 MiB |
| A four-sentence parse, no image | 81 MiB | 81 MiB |
| The same with its diagram | 84 MiB | 134 MiB |
| Six diagrams asked for at once | 97 MiB | 147 MiB |
| First meaning search (model loads) | 324 MiB | 324 MiB |
| Four diagrams and four meaning searches together | 380 MiB | 429 MiB |
| Everything above, at rest | 419 MiB | 429 MiB |

The gismu search is the one paged result whose page cannot be reached without
ranking everything before it, so what a deep page costs was measured on its own.
The largest request in the test corpus — twelve phonetic sources over the Ilmen
twelve-letter inventory — generates 96,475 candidates, all of which pass the
filters. Asking it for the deepest page a page number can name (results 327,671
onwards, past the end of any real result set) took 0.80s and brought the
process to a 56 MiB peak, against 39.8 MiB for the first page of the same
request; the ranking it holds is a word and a score per candidate, 88 bytes
each, so it cannot exceed about 10 MiB however deep the page, and it is held
only while that one page is computed. The same request at page 100 measured
40.4 MiB. Measured in release on the development machine.

Each of those diagrams is 8516×776 pixels, 6.6 of the 8 million a diagram may
have, and 226 KB of PNG: a request near the cap rather than a small one.

The embedding model is what dominates: about 240 MiB in this run, loaded on
the first meaning search and resident afterwards. One diagram near the cap
added about 50 MiB while it rendered. Resident memory did not fall back after
the peaks: it ended within 10 MiB of the highest point reached, so a service
that has once been busy stays about that large.

Two things brought the run inside the service's 512 MiB budget on this host.
Analysis runs one job at a time, so at most one diagram is ever being
rasterized; and the runtime image sets `MALLOC_ARENA_MAX=2`. Without that
variable the same sequence reached 591 MiB, and with the previous limits (two
analysis workers, sixteen megapixels) it reached 616 MiB even with the variable
set. Neither of those fits.

These are measurements of this sequence of requests on this host and this
architecture, not a ceiling for every input: they say what these representative and near-limit
requests cost, not what the largest possible request would. The figures above
include the embedding model; the same sequence up to the first meaning search,
which is where the model is loaded, stayed at 147 MiB.

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
