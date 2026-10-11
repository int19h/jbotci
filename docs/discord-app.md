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
**Open in app** link carrying the request — the same source, the same settings
— and opening the view the app would open for it; the other three tools have
no page, so their forms have no link and no substitute.

The gismu search is ranked by one of two scorers, chosen in its form: the
classic one compares letters, the phonetic one compares sounds. The result says
which ranked it, the choice travels with the message like every other setting,
and the link opens the app on the same one. Changing it starts the list over,
because it changes which candidates come first.

A result that is a list carries **Previous** and **Next** beside it. They turn
the page on the same message, and each page says which results it shows —
`6-10`, or `6-10 of 38` where the number of results is genuinely known. It is
known when the search has reached the end of its results, and when it counted
them all as part of its work; a search that was asked for one page and handed
one back knows nothing about the rest, and then the page says only what it
shows rather than inventing a figure. A direction that does not exist is greyed
rather than hidden, and a result with a single page carries no buttons at all.
A button the result does not offer — a greyed one, or one from a result that
has no buttons — changes nothing and says so privately, whatever its
identifier says it is for.

Paging goes as far as the results do: there is no page ceiling. What a page
shows is always one page's worth of work — five cards, five passages of the
book, five candidates worked out in full — however deep the page is. What it
takes to reach that page differs by search. A word search of the dictionary or
of the book starts at the page's first result and reads no further back than
its own index. A meaning search ranks up to the end of the page, because a
similarity ranking is an order over everything and has no place to start in
the middle; what it ranks is a number and a score each. The gismu search
generates and scores its whole field whatever page is asked for, and keeps the
best of them as words and scores. So reaching a deep page costs the ranking
that reaches it, showing it costs a page, and none of these three has a last
page other than the one the results end on. A parse, a word report and a compound are one result each and
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
is not Lojban are each refused with what is wrong.
The form also accepts messages that store words and fixed rafsi in separate fields.
It shows those fixed rafsi after the words, for example as `-kla-`.

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

To restore a command schema, deploy the selected image and operate its own `setup --discord-commands`.
Published messages work where that build supports their stored state.
The service refuses configuration values that it does not recognize.
A gismu result whose state has no scorer field uses the classic scorer.

If a build changes while a form is open, the service refuses the form and asks the reader to reopen it.
A result recomputed by another build identifies that build in the edited message.
The result text and image come from the identified build.

**Guild and global registrations** are separate lists, and a guild command
shadows the global one of the same name: registering globally leaves a guild
command exactly as it was. Removing one means deleting that guild command
through the API, or overwriting that guild's command list without it; this
application's `setup --discord-commands --guild` always writes its one command,
so it cannot be used to empty a guild.

## What one request may spend

The service has three seconds to acknowledge a request and fifteen minutes to finish its message.
Analysis, attachment downloads, and meaning search each have a bounded queue and a fixed worker count.
Analysis admits one job at a time to bound concurrent memory use.

A request that cannot enter its queue receives a refusal message.
A worker retains its permit until the work ends, even when the caller stops waiting.
It also retains its delivery allocation and its message reservation while it writes.
Meaning search retains its permit during a cold model load.
One deadline covers both queue time and delivery to Discord.

| Bound | Value | Why |
| --- | --- | --- |
| Source field | 4000 UTF-16 units | Discord's own text-input maximum, enforced at the command and at the form alike. |
| Message text | 4000 units across all text components | The application's budget for one message; longer results become an excerpt plus a complete attachment. |
| Result page | 5 results | What reads well on a phone. Pages themselves are unbounded: each one is fetched when it is asked for. |
| App link results | the web app's own default (20) | An "Open in app" link opens the view the app itself would open for that search. It does not carry the reader's place in Discord's paging, which is a separate thing with its own bound. |
| App link | 4000 units for the whole link component | The application's own budget for one form text component, not a documented Discord limit. |
| Candidate ranking | one word, one score and any colliding word, per generated candidate that passes the filters | What reaching a gismu page costs while the service computes it. |
| Compound construction | 8192 part placements, 24 pieces, 256 letters | The service refuses a construction that exceeds these bounds. |
| Diagram | 600 blocks, 160 columns, 8 megapixels, 8 MiB | A diagram larger than this is refused with its reason, and the submission that asked for it changes nothing. The pixel limit bounds the size of the raster buffer. |
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

## Memory bounds

The service loads the embedding model on the first meaning search and keeps it resident.
Candidate ranking retains words, scores, and collision words while it computes one page.
It does not retain rafsi that a filter computes.
The service builds result details for the requested page only.

The runtime image sets `MALLOC_ARENA_MAX=2` to limit allocator arenas.
Analysis admits one job at a time, so only one diagram is rasterized at a time.

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
