// Mockup behaviour for the per-card definition language.
const OFFICIAL = "officialdata";
const PREFS = loadPrefs();
// The fallback order for a word that has none of the preferred languages
// (owner decision): English, then Lojban, then the language whose top
// definition has the highest score.
const FALLBACK = ["en", "jbo"];

// The English entries come from the app's own dictionary, as on the live page.
const ENGLISH = {
  djacu: {
    definition: "$x_{1}$ is made of/contains/is a quantity/expanse of water; (adjective:) $x_{1}$ is aqueous/[aquatic].",
    notes: "Aquatic (= {jaupli}).  See also {lalxu}, {rirxe}, {xamsi}, {limna}, {litki}, {lumci}, {bumru}, {jinto}.",
    glosses: ["water"], user: OFFICIAL, score: 100000,
  },
  cinba: {
    definition: "$x_{1}$ (agent) kisses/busses $x_{2}$ at locus $x_{3}$.",
    notes: "See also {ctebi}.", glosses: ["kiss"], user: OFFICIAL, score: 100000,
  },
};
for (const [word, en] of Object.entries(ENGLISH)) {
  const entry = CARD_DATA[word].find((d) => d.tag === "en");
  Object.assign(entry, en);
}

const CARDS = [
  { word: "djacu", type: "gismu", typeClass: "is-gismu", rafsi: ["jac", "jau", "djac"], defs: CARD_DATA.djacu },
  { word: "cinba", type: "gismu", typeClass: "is-gismu", rafsi: ["cinb"], defs: CARD_DATA.cinba },
  { word: "jaurbisli", type: "lujvo", typeClass: "is-lujvo", parts: ["djacu", "bisli"], defs: [
    { tag: "en", en: "English", real: "English", definition: "$x_1$ is water ice", notes: "", glosses: ["water ice"], user: "Ilmen", score: 1 },
  ] },
];
const REALNAMES = { officialdata: "Official Data", Ilmen: "Ilmen" };

const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

// "$x_{1}$" becomes MathML and "{word}" a word link, as the app renders them.
function inline(text) {
  return esc(text)
    .replace(/\$([^$]+)\$/g, (_, m) => {
      const sub = m.match(/^([a-zA-Z])_\{?([^}]+)\}?$/);
      const inner = sub ? `<msub><mi>${sub[1]}</mi><mn>${sub[2]}</mn></msub>` : `<mi>${m}</mi>`;
      return `<span class="spa-cll-math"><math class="math-var" display="inline">${inner}</math></span>`;
    })
    .replace(/\{([^}]+)\}/g, (_, w) => `<span class="dictionary-word-link">${w}</span>`);
}

function vote(def) {
  if (def.user === OFFICIAL) return { text: "∞", cls: "is-official" };
  const n = Math.round(def.score);
  const text = n > 0 ? `+${n}` : `${n}`;
  return { text, cls: n >= 5 ? "is-high" : n >= 2 ? "is-medium" : "is-low" };
}

const credit = (user) => REALNAMES[user] && REALNAMES[user] !== user ? `by ${user} (${REALNAMES[user]})` : `by ${user}`;

const chevron = '<svg class="dictionary-lang-chevron" viewBox="0 0 10 6" aria-hidden="true"><path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';

function langSegment(card, def, open) {
  if (card.defs.length === 1) {
    return `<span class="dictionary-meta-segment dictionary-meta-tooltip dictionary-lang-tag" title="Definition in ${esc(def.en)}. This word has no definition in another language."><span class="dictionary-lang-name">${esc(def.tag)}</span></span>`;
  }
  return `<button class="dictionary-meta-segment dictionary-lang-tag" type="button" aria-haspopup="menu" aria-expanded="${open}" aria-controls="menu-${card.word}"
      aria-label="Definition language: ${esc(def.en)}. ${card.defs.length} languages available." title="Definition in ${esc(def.en)}. Choose another of ${card.defs.length} languages.">
    <span class="dictionary-lang-name">${esc(def.tag)}</span>${chevron}</button>`;
}

// The language a card opens with: the first preferred language that the word
// has a definition in.
function defaultTag(card) {
  const has = new Set(card.defs.map((d) => d.tag));
  const preferred = PREFS.find((t) => has.has(t));
  if (preferred) return preferred;
  const fallback = FALLBACK.find((t) => has.has(t));
  if (fallback) return fallback;
  return [...card.defs].sort((a, b) => b.score - a.score)[0].tag;
}

function menu(card, current, open) {
  const preferred = PREFS.map((t) => card.defs.find((d) => d.tag === t)).filter(Boolean);
  const others = card.defs.filter((d) => !PREFS.includes(d.tag)).sort((a, b) => a.en.localeCompare(b.en));
  const row = (d) => {
    const v = vote(d);
    const names = d.real && d.real !== d.en ? `${esc(d.en)} <span class="dictionary-lang-option-real">· ${esc(d.real)}</span>` : esc(d.en);
    return `<button class="dictionary-lang-option" type="button" role="menuitemradio" aria-checked="${d.tag === current}" data-tag="${esc(d.tag)}">
      <span class="dictionary-lang-check" aria-hidden="true">${d.tag === current ? "✓" : ""}</span>
      <span class="dictionary-lang-option-tag">${esc(d.tag)}</span>
      <span class="dictionary-lang-option-names">${names}</span>
      <span class="dictionary-lang-option-score ${v.cls}" title="Score of the top definition">${v.text}</span></button>`;
  };
  const group = (label, defs) => defs.length
    ? `<div class="dictionary-lang-menu-group" role="group" aria-label="${label}">${defs.map(row).join("")}</div>` : "";
  return `<div class="dictionary-lang-menu" id="menu-${card.word}" role="menu" aria-label="Definition language" ${open ? "" : "hidden"}>
    <p class="dictionary-lang-menu-lede">Definition language</p>${group("Preferred languages", preferred)}${group("Other languages", others)}</div>`;
}

const state = {};
let openWord = null;

function render() {
  const host = document.getElementById("cards");
  host.innerHTML = CARDS.map((card) => {
    const current = state[card.word] || defaultTag(card);
    const def = card.defs.find((d) => d.tag === current);
    const v = vote(def);
    const open = openWord === card.word;
    const composition = card.rafsi
      ? `<span class="dictionary-word-composition-group dictionary-word-rafsi-group"><span class="dictionary-word-inline-separator">≘</span><span class="dictionary-inline-pill-row">${card.rafsi.map((r) => `<span class="chip rafsi-chip">${r}</span>`).join("")}</span></span>`
      : `<span class="dictionary-word-composition-group dictionary-word-decomposition-group"><span class="dictionary-word-inline-separator">=</span>${card.parts.map((p, i) => `${i ? '<span class="dictionary-word-inline-separator">+</span>' : ""}<span class="dictionary-word-link">${p}</span>`).join("")}</span>`;
    return `<section class="result-card" data-word="${card.word}">
      <header class="result-header">
        <div class="dictionary-word-cluster">
          <h2 class="word dictionary-headword-title"><a class="dictionary-headword-link" href="#">${card.word}</a></h2>
          ${composition}
        </div>
        <div class="tag-row">
          <span class="dictionary-author-credit">${esc(credit(def.user))}</span>
          <div class="dictionary-meta-anchor">
            <div class="dictionary-meta-pill">
              <span class="dictionary-meta-segment dictionary-word-type-tag ${card.typeClass}">${card.type}</span>
              ${langSegment(card, def, open)}
              <span class="dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag ${v.cls}">${v.text}</span>
            </div>
            ${card.defs.length > 1 ? menu(card, current, open) : ""}
          </div>
        </div>
      </header>
      <p class="dictionary-definition-copy" lang="${esc(def.tag)}" dir="auto">${inline(def.definition)}<button class="dictionary-definition-copy-button" type="button" aria-label="Copy definition" title="Copy definition"><svg class="dictionary-copy-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 7h9a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2zM5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg></button></p>
      ${def.glosses.length ? `<div class="chip-row dictionary-gloss-row" lang="${esc(def.tag)}">${def.glosses.map((g) => `<span class="chip dictionary-gloss-pill">${esc(g)}</span>`).join("")}</div>` : ""}
      ${def.notes ? `<p class="dictionary-note-copy" lang="${esc(def.tag)}" dir="auto">${inline(def.notes)}</p>` : ""}
    </section>`;
  }).join("");
}

function setOpen(word, focusCurrent) {
  openWord = word;
  render();
  if (word && focusCurrent) document.querySelector(`#menu-${word} [aria-checked="true"]`)?.focus();
}

document.addEventListener("click", (e) => {
  const segment = e.target.closest("button.dictionary-lang-tag");
  if (segment) {
    const word = segment.closest(".result-card").dataset.word;
    setOpen(openWord === word ? null : word, true);
    return;
  }
  const option = e.target.closest(".dictionary-lang-option");
  if (option) {
    const word = option.closest(".result-card").dataset.word;
    state[word] = option.dataset.tag;
    setOpen(null);
    document.querySelector(`[data-word="${word}"] button.dictionary-lang-tag`)?.focus();
  }
});
document.addEventListener("pointerdown", (e) => {
  if (openWord && !e.target.closest(".dictionary-lang-menu, button.dictionary-lang-tag")) setOpen(null);
}, true);
document.addEventListener("keydown", (e) => {
  if (!openWord) return;
  if (e.key === "Escape") {
    const word = openWord;
    setOpen(null);
    document.querySelector(`[data-word="${word}"] button.dictionary-lang-tag`)?.focus();
  } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    const items = [...document.querySelectorAll(`#menu-${openWord} .dictionary-lang-option`)];
    const i = items.indexOf(document.activeElement);
    items[(i + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
    e.preventDefault();
  }
}, true);

document.getElementById("mock-theme").addEventListener("change", (e) => {
  const s = document.getElementById("mock-shell");
  s.classList.remove("theme-day", "theme-night");
  s.classList.add(`theme-${e.target.value}`);
});
const showWidth = () => { document.getElementById("mock-width").textContent = `${window.innerWidth}px`; };
window.addEventListener("resize", showWidth);

// ?djacu=ru&open=cinba&prefs=ru,en&theme=night set the state for screenshots.
const params = new URLSearchParams(location.search);
for (const card of CARDS) if (params.get(card.word)) state[card.word] = params.get(card.word);
document.getElementById("mock-prefs").textContent = `preferred: ${PREFS.join(", ")}`;
if (params.get("theme")) { document.getElementById("mock-theme").value = params.get("theme"); document.getElementById("mock-theme").dispatchEvent(new Event("change")); }
openWord = params.get("open");
showWidth();
render();
