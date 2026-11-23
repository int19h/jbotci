# vlacku definition languages: design mockups

This branch holds design material only. It has no product code. The design is
described in the GitHub issue that links here.

## Contents

- `mockup/cards.html`: three vlacku result cards with the per-card language
  segment and menu.
- `mockup/settings.html`: the proposed "Definition languages" setting.
- `screenshots/`: the mockups at 900px and 393px wide, in day and night themes.

## How to open the mockups

Open `mockup/settings.html` or `mockup/cards.html` in a browser straight from a
checkout of this branch. No server is needed. The two pages share the list of
preferred languages through the browser's local storage. Thus a change on the
settings page shows on the cards page. The dark bar at the top of each page is
part of the mockup, not of the design.

Both pages accept URL parameters, so that a screenshot can show a given state:

- `?prefs=ru,de,en` sets the preferred languages for one page load.
- `cards.html?open=djacu` opens the language menu of the djacu card.
- `cards.html?djacu=ja` shows the djacu card in Japanese.
- `?theme=night` switches to the night theme.
- `settings.html?add=1` opens the "Add language" list.

## Data

- `mockup/languages.js`: the Lensisku language list from
  `https://lensisku.lojban.org/api/language/languages`, fetched 2026-11-22,
  without the "Test Language" entry.
- `mockup/cards-data.js`: the definitions of djacu (14 languages) and cinba
  (12 languages) from `https://lensisku.lojban.org/api/jbovlaste/definitions`,
  fetched 2026-11-23. For each language, it keeps the definition with the
  highest score. The English definitions come from the dictionary that the app
  embeds. jaurbisli has only an English definition.
- `mockup/assets/main.css` and `mockup/assets/fonts/noto-sans-variable.ttf`:
  copies of `crates/jbotci-ui/assets/main.css` and
  `crates/jbotci-ui/assets/fonts/noto-sans-variable.ttf` from `main` at
  d357fa382e, so that the mockups look like the app.

The new styles are in `mockup/cards.css` and `mockup/settings.css`, below the
"Proposed design" comment. The rules above that comment only frame the mockup
page.
