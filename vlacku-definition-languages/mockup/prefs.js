// The preferred definition languages, shared by settings.html and cards.html.
// The mockup keeps them in localStorage. "?prefs=ru,en" in the URL overrides them
// for one page load, so that screenshots can show a given list.
const PREFS_KEY = "mockup-vlacku-definition-languages";
const PREFS_DEFAULT = ["en"];

function loadPrefs() {
  const fromUrl = new URLSearchParams(location.search).get("prefs");
  if (fromUrl) return fromUrl.split(",").filter(Boolean);
  try {
    const stored = JSON.parse(localStorage.getItem(PREFS_KEY));
    if (Array.isArray(stored) && stored.length) return stored;
  } catch (_) { /* fall through to the default */ }
  return [...PREFS_DEFAULT];
}

function savePrefs(prefs) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify(prefs)); } catch (_) { /* private mode */ }
}
