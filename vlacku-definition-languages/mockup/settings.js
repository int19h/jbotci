// Mockup behaviour for the "Definition languages" setting.
const byTag = new Map(LANGUAGES.map((l) => [l.tag, l]));
let prefs = loadPrefs().filter((t) => byTag.has(t));
const $ = (id) => document.getElementById(id);
const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

const icon = {
  up: '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 10 8 5.5 12.5 10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  down: '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 6 8 10.5 12.5 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  remove: '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>',
};

const names = (l) => l.real && l.real !== l.en
  ? `${esc(l.en)} <span class="settings-deflang-real">· ${esc(l.real)}</span>` : esc(l.en);

function renderList() {
  $("deflang-list").innerHTML = prefs.map((tag, i) => {
    const l = byTag.get(tag);
    // The list is never empty: with no language left, a card has no rule for
    // which definition to show first.
    const last = prefs.length === 1;
    return `<li class="settings-deflang-row" data-tag="${esc(tag)}">
      <span class="settings-deflang-rank">${i + 1}.</span>
      <span class="settings-deflang-tag">${esc(tag)}</span>
      <span class="settings-deflang-names">${names(l)}</span>
      <span class="settings-deflang-actions">
        <button class="settings-dialect-icon-button" type="button" data-act="up" aria-label="Move ${esc(l.en)} up" title="Move up" ${i === 0 ? "disabled" : ""}>${icon.up}</button>
        <button class="settings-dialect-icon-button" type="button" data-act="down" aria-label="Move ${esc(l.en)} down" title="Move down" ${i === prefs.length - 1 ? "disabled" : ""}>${icon.down}</button>
        <button class="settings-dialect-icon-button" type="button" data-act="remove" aria-label="Remove ${esc(l.en)}" title="${last ? "At least one language stays in the list" : "Remove"}" ${last ? "disabled" : ""}>${icon.remove}</button>
      </span></li>`;
  }).join("");
}

function matches(l, q) {
  q = q.toLowerCase();
  return !q || [l.tag, l.en, l.real, l.jbo].some((s) => s && s.toLowerCase().includes(q));
}

function renderOptions() {
  const q = $("deflang-filter").value.trim();
  const rows = LANGUAGES.filter((l) => !prefs.includes(l.tag) && matches(l, q));
  $("deflang-options").innerHTML = rows.length
    ? rows.map((l) => `<button class="settings-deflang-option" type="button" role="option" data-tag="${esc(l.tag)}"><span class="settings-deflang-tag">${esc(l.tag)}</span><span class="settings-deflang-names">${names(l)}</span></button>`).join("")
    : '<div class="settings-deflang-empty">No language matches.</div>';
}

function setPicker(open) {
  $("deflang-add").setAttribute("aria-expanded", String(open));
  $("deflang-picker").hidden = !open;
  if (open) { $("deflang-filter").value = ""; renderOptions(); $("deflang-filter").focus(); }
}

function commit() { savePrefs(prefs); renderList(); }

$("deflang-list").addEventListener("click", (e) => {
  const b = e.target.closest("button[data-act]");
  if (!b) return;
  const tag = b.closest("li").dataset.tag;
  const i = prefs.indexOf(tag);
  const act = b.dataset.act;
  if (act === "up" && i > 0) [prefs[i - 1], prefs[i]] = [prefs[i], prefs[i - 1]];
  if (act === "down" && i < prefs.length - 1) [prefs[i + 1], prefs[i]] = [prefs[i], prefs[i + 1]];
  if (act === "remove" && prefs.length > 1) prefs.splice(i, 1);
  commit();
  // Keep focus on the same control of the moved row, so repeated presses keep moving it.
  const again = document.querySelector(`li[data-tag="${tag}"] button[data-act="${act}"]:not(:disabled)`)
    || document.querySelector(`li[data-tag="${tag}"] button:not(:disabled)`) || $("deflang-add");
  again.focus();
});
$("deflang-add").addEventListener("click", () => setPicker($("deflang-picker").hidden));
$("deflang-filter").addEventListener("input", renderOptions);
$("deflang-options").addEventListener("click", (e) => {
  const b = e.target.closest(".settings-deflang-option");
  if (!b) return;
  prefs.push(b.dataset.tag);
  commit();
  setPicker(false);
  $("deflang-add").focus();
});
document.addEventListener("pointerdown", (e) => {
  if (!$("deflang-picker").hidden && !e.target.closest(".settings-deflang-add-wrap")) setPicker(false);
}, true);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("deflang-picker").hidden) { setPicker(false); $("deflang-add").focus(); }
}, true);

$("mock-theme").addEventListener("change", (e) => {
  const s = $("mock-shell");
  s.classList.remove("theme-day", "theme-night");
  s.classList.add(`theme-${e.target.value}`);
});
const params = new URLSearchParams(location.search);
if (params.get("theme")) { $("mock-theme").value = params.get("theme"); $("mock-theme").dispatchEvent(new Event("change")); }
const showWidth = () => { $("mock-width").textContent = `${window.innerWidth}px`; };
window.addEventListener("resize", showWidth);
showWidth();
renderList();
if (params.get("add")) { setPicker(true); if (params.get("filter")) { $("deflang-filter").value = params.get("filter"); renderOptions(); } }
