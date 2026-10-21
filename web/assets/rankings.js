// Progressive enhancement for the rankings table: live search and the
// expandable vote panel under each row. Without it, the search form submits
// and the row toggle links to the tracker's vote section.

const table = document.querySelector("[data-rankings]");
const search = document.querySelector("[data-search]");
const empty = document.querySelector("[data-search-empty]");

function setOpen(row, open) {
	const panel = document.getElementById(row.dataset.panel);
	if (panel) panel.hidden = !open;
	row.toggleAttribute("data-open", open);
	row.querySelector("[data-toggle]")?.setAttribute("aria-expanded", String(open));
	if (open) loadPanel(row);
}

async function loadPanel(row) {
	const slot = document.getElementById(row.dataset.panel)?.querySelector("[data-vote-slot]");
	if (!slot || slot.dataset.state) return;
	slot.dataset.state = "loading";
	slot.textContent = "Loading…";
	const params = new URLSearchParams(window.location.search);
	params.delete("voted");
	const query = params.toString();
	try {
		const response = await fetch(query ? `${row.dataset.voteUrl}?${query}` : row.dataset.voteUrl);
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		const doc = new DOMParser().parseFromString(await response.text(), "text/html");
		const panel = doc.querySelector("[data-vote-panel]");
		if (!panel) throw new Error("no vote panel");
		slot.replaceChildren(document.importNode(panel, true));
		slot.dataset.state = "loaded";
	} catch {
		delete slot.dataset.state;
		const link = document.createElement("a");
		link.href = row.dataset.voteUrl;
		link.className = "underline";
		link.textContent = "open the vote page";
		slot.replaceChildren("Could not load the vote form; ", link, ".");
	}
}

table?.addEventListener("click", (event) => {
	const row = event.target.closest("tr[data-panel]");
	if (!row) return;
	const toggle = event.target.closest("[data-toggle]");
	if (!toggle) {
		if (event.target.closest("a, button, input, label")) return;
		if (window.getSelection()?.toString()) return;
	}
	event.preventDefault();
	setOpen(row, !row.hasAttribute("data-open"));
});

function applySearch() {
	const raw = search.value.trim();
	const needle = raw.toLowerCase();
	let shown = 0;
	for (const row of document.querySelectorAll("tr[data-panel]")) {
		const match = row.dataset.haystack.includes(needle);
		row.hidden = !match;
		if (!match) setOpen(row, false);
		if (match) shown += 1;
	}
	if (empty) empty.hidden = shown > 0;
	for (const mirror of document.querySelectorAll("[data-search-mirror]")) {
		mirror.value = raw;
	}
	for (const link of document.querySelectorAll("[data-keep-search]")) {
		const url = new URL(link.href);
		if (raw) url.searchParams.set("q", raw);
		else url.searchParams.delete("q");
		link.href = url.pathname + url.search;
	}
	const url = new URL(window.location.href);
	if (raw) url.searchParams.set("q", raw);
	else url.searchParams.delete("q");
	url.searchParams.delete("voted");
	window.history.replaceState(null, "", url.pathname + url.search + url.hash);
}

search?.addEventListener("input", applySearch);

document.addEventListener("keydown", (event) => {
	if (!search || event.key !== "/" || event.ctrlKey || event.metaKey || event.altKey) return;
	const target = event.target;
	if (target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea, select"))) return;
	event.preventDefault();
	search.focus();
	search.select();
});
