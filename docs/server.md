# Server work limits

The server reads these environment variables before it binds its listening socket.
A missing variable uses its default.
An invalid value stops startup with an error that names the variable and its accepted range.
Values must be decimal integers without surrounding spaces.
Queue limits accept nonnegative integers that fit `usize`.
Active limits accept positive integers through `tokio::sync::Semaphore::MAX_PERMITS`.
The compound piece limit starts at two.
All other limits must be positive integers that fit their storage type.

| Variable | Default | Meaning |
| --- | ---: | --- |
| `JBOTCI_COMPUTE_ACTIVE` | 1 | Active computation across all public adapters. |
| `JBOTCI_COMPUTE_QUEUED` | 8 | Requests that wait for computation. |
| `JBOTCI_FETCH_ACTIVE` | 4 | Active attachment downloads. |
| `JBOTCI_FETCH_QUEUED` | 8 | Requests that wait for a download. |
| `JBOTCI_SOURCE_FIELD_UNITS` | 4000 | UTF-16 units in each source field. |
| `JBOTCI_SOURCE_TOTAL_UNITS` | 4000 | UTF-16 units across all source fields of one request. |
| `JBOTCI_RESULT_ENTRIES` | 512 | Entries returned by one operation. |
| `JBOTCI_TEXT_BYTES` | 1048576 | UTF-8 bytes of text and diagnostics in one operation. |
| `JBOTCI_RESPONSE_BYTES` | 12582912 | Bytes in the encoded response body. |
| `JBOTCI_DEADLINE_MS` | 20000 | Milliseconds for admission, computation, and normal response delivery. |
| `JBOTCI_ERROR_DELIVERY_MS` | 1000 | Extra milliseconds to deliver a coded timeout or overload error. |
| `JBOTCI_JVOZBA_PIECES` | 24 | Expanded pieces in a compound search. |
| `JBOTCI_JVOZBA_PLACEMENTS` | 8192 | Sum of piece placements across compound candidates. |
| `JBOTCI_JVOZBA_LETTERS` | 256 | Letters in a candidate spelling. |
| `JBOTCI_DIAGRAM_BLOCKS` | 600 | Layout blocks in a diagram. |
| `JBOTCI_DIAGRAM_COLUMNS` | 160 | Grid columns in a diagram. |
| `JBOTCI_DIAGRAM_PIXELS` | 8000000 | Raster pixels after image scaling. |
| `JBOTCI_DIAGRAM_BYTES` | 8388608 | Bytes in encoded PNG or SVG output. |

A UTF-16 unit is one 16-bit element of UTF-16 text.
Most characters use one unit. Some characters, such as emoji, use two units.
The source limits count the input before normalization.
The server rejects excess work. It does not shorten the input or clamp a requested result count.
Discord also applies its platform limits and its per-interaction attachment limit.
The local CLI keeps its unrestricted operation policy unless its caller supplies server limits.

A full computation queue returns the stable code `server_busy`.
The message is `The server is busy. Try again later.`
HTTP adapters return 503 for overload and 504 when the computation deadline expires.
MCP reports tool errors through `isError` and includes a stable error code in `structuredContent`.
Static assets and health responses do not need a computation slot.

The deadline stops the caller from waiting. It does not interrupt arbitrary Rust computation.
A running worker keeps its slot until it exits, even when its caller disconnects or times out.
These limits do not impose a hard heap limit on one parser operation.

Structured responses count UTF-8 string values against the text limit.
Their encoded bytes also count against the response limit.
Escaping can increase the encoded length.
PNG and SVG use the diagram budget before their bytes enter the final response.

A response retains its computation slot while the transport owns its bytes.
The delivery deadline closes a connection that still holds an expired response.
On HTTP/2, this also closes other streams on that connection.

An MCP request identifier must leave room for a canonical error response within `JBOTCI_RESPONSE_BYTES`.
The server counts that envelope before it admits the operation.
If the envelope cannot fit, HTTP returns 422 with `output_limit_exceeded` before any computation starts.
Canonical HTTP error messages use a separate reserved allowance of their literal byte length.

The HTTP layer also bounds API bodies that come from framework request errors.
It collects their frames before it sends headers, so an oversized response becomes a complete error response.
The collected frames retain their original byte owners and admission guards.
Static assets and health responses do not use this computation response bound.

HTTP headers and transport framing do not use this body budget.

The error delivery window starts at the operation deadline.
Only coded timeout and overload errors use this window.
The server drops a late normal result and sends the coded timeout error instead.
The window does not extend computation or permit another operation.
