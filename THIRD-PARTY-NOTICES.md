# Third-Party Notices

jbotci's own source code is licensed under the MIT License (see
[LICENSE.md](LICENSE.md)). However, jbotci is *distributed* — as native
binaries, as the WebAssembly web bundle, and as container images — together with
third-party software, fonts, and reference data that carry their own licenses.

This file collects the notices those licenses require us to reproduce when we
redistribute jbotci. It covers:

1. **Fonts** bundled into the UI, gentufa rendering, and the web asset bundle.
2. **Reference data** embedded into the binaries (the CLL grammar reference and
   the Lojban dictionary).
3. **Rust crate dependencies** compiled into the shipped binaries and the web
   bundle.

Fonts and reference data that live only in the `vendor/cll` submodule for
build-time rendering of the reference book (e.g. Linux Libertine/Biolinum and
the Noto CJK serif faces) are **not** embedded in any shipped artifact and are
therefore out of scope here; they retain their own upstream licenses within that
submodule.

---

## 1. Fonts

All fonts bundled with jbotci are licensed under the **SIL Open Font License,
Version 1.1**. The copyright notices for the bundled families are:

- **STIX Two Text** and **STIX Two Math** —
  Copyright © 2001–2021 The STIX Fonts Project Authors
  (<https://github.com/stipub/stixfonts>).
- **Noto Sans** (upright and italic) —
  Copyright © 2022 The Noto Project Authors
  (<https://github.com/notofonts/latin-greek-cyrillic>).
- **Noto Sans Math** —
  Copyright © 2022 Google LLC.
- **Crisa** — a Modified Version of **Lato** —
  Copyright © 2011–2015 tyPoland Łukasz Dziedzic (<http://www.typoland.com/>),
  with Reserved Font Name "Lato".

The full text of the license follows.

```
SIL OPEN FONT LICENSE
Version 1.1 - 26 February 2007

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply to any
document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical writer or
other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining a
copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components, in
Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or in
the appropriate machine-readable metadata fields within text or binary
files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any Modified
Version, except to acknowledge the contribution(s) of the Copyright
Holder(s) and the Author(s) or with their explicit written permission.

5) The Font Software, modified or unmodified, in part or in whole, must be
distributed entirely under this license, and must not be distributed under
any other license. The requirement for fonts to remain under this license
does not apply to any document created using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are not
met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM OTHER
DEALINGS IN THE FONT SOFTWARE.
```

---

## 2. Reference data

### The Complete Lojban Language (CLL)

jbotci embeds text, examples, and the formal grammar of *The Complete Lojban
Language* by John Woldemar Cowan, used to power the grammar reference (the
`cukta` tool) and to drive parser and semantics development. The book carries
the following notice, which its license requires us to preserve on all copies:

```
Copyright © 1997 by The Logical Language Group, Inc. All Rights Reserved.

Permission is granted to make and distribute verbatim copies of this book,
either in electronic or in printed form, provided the copyright notice and
this permission notice are preserved on all copies.

Permission is granted to copy and distribute modified versions of this book,
provided that the modifications are clearly marked as such, and provided that
the entire resulting derived work is distributed under the terms of a
permission notice identical to this one.

Permission is granted to copy and distribute translations of this book into
another language, under the above conditions for modified versions, except that
this permission notice may be stated in a translation that has been approved by
the Logical Language Group, rather than in English.

For information, contact: The Logical Language Group, 2904 Beau Lane,
Fairfax VA 22031-1303 USA. Web Address: http://www.lojban.org
```

The formal grammar (the machine-parseable EBNF) and certain other contents of
the book are placed in the public domain by that same notice. The maintainers
of the CLL sources consider the permission notice above to be equivalent to the
[Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/)
license.

### Lojban dictionary (jbovlaste / Lensisku)

jbotci embeds Lojban dictionary data used by the `vlacku` word-lookup tool and
related features. The data is a snapshot exported from
[Lensisku](https://lensisku.lojban.org/), the current front-end to the
community-maintained **jbovlaste** dictionary database
(<https://jbovlaste.lojban.org/>).

The jbovlaste dictionary databases are placed in the **public domain**. The
individual definitions are contributed by the Lojban community under those
public-domain terms.

---

## 3. Rust crate dependencies

The jbotci binaries and web bundle statically link a number of third-party Rust
crates. Every such crate is distributed under a permissive license (variants of
the MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0, and public-domain-style
licenses). The list below enumerates each crate, its version, and its license,
followed by the full text of each license as published by the respective
copyright holders.

<!-- BEGIN GENERATED CRATE NOTICES -->

_The per-crate dependency notices are generated with
[`cargo about`](https://github.com/EmbarkStudios/cargo-about) and inserted here._

<!-- END GENERATED CRATE NOTICES -->
