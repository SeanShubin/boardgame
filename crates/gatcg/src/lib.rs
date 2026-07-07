//! Grand Archive TCG reference-data tooling.
//!
//! The public card API at <https://api.gatcg.com> exposes the whole card library
//! through a paginated search endpoint. Every card *edition* (printing) links an
//! image, and the search payload also carries each card's full rules text. This
//! crate mirrors both to a local directory:
//!
//! - [`download_edition`] writes `images/<set-prefix>/<slug>.jpg`
//! - [`write_card_text`] writes `rules/<set-prefix>/<slug>.md`
//!
//! The two trees are **siblings with identical structure**, so a card's art and
//! its rules sit at matching paths under `images/` and `rules/`.
//!
//! It is network + filesystem only; no game crate depends on it. Both mirrors
//! are **resumable and safe to interrupt** — a non-empty destination is never
//! rewritten, and every file is committed via an atomic temp-then-rename.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// Base host for the Grand Archive card API.
pub const API_HOST: &str = "https://api.gatcg.com";

/// The search endpoint caps `page_size` at 50 regardless of what is requested,
/// so we page in fifties.
pub const PAGE_SIZE: u32 = 50;

/// One printing of a card — the unit that owns an image and a rules file.
#[derive(Debug, Clone)]
pub struct Edition {
    pub card_name: String,
    pub set_name: String,
    pub set_prefix: String,
    pub slug: String,
    pub collector_number: String,
    /// Path component returned by the API, e.g. `/cards/images/<hash>.jpg`.
    /// Empty when this printing has no art.
    pub image_path: String,
}

impl Edition {
    /// Full source URL for this edition's image.
    pub fn image_url(&self) -> String {
        format!("{API_HOST}{}", self.image_path)
    }

    /// Image file extension, defaulting to `jpg`.
    pub fn ext(&self) -> &str {
        Path::new(&self.image_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
    }

    /// Destination relative to the output root: `images/<prefix>/<slug>.<ext>`.
    ///
    /// Grouping by set prefix (not the human set name) keeps paths legal on
    /// Windows, where set names like `Re:Collection Slime Sovereign` contain `:`.
    pub fn image_rel_path(&self) -> PathBuf {
        rel_path("images", &self.set_prefix, &self.slug, self.ext())
    }
}

/// A card's printed rules text, denormalized onto one edition so the `rules/`
/// tree mirrors the `images/` tree 1:1. A card's text is identical across its
/// reprints, so it repeats per edition by design — that is what keeps every
/// image path paired with a matching rules path.
#[derive(Debug, Clone)]
pub struct CardRecord {
    pub edition: Edition,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    pub classes: Vec<String>,
    pub elements: Vec<String>,
    pub cost: String,
    pub level: Option<i64>,
    pub power: Option<i64>,
    pub life: Option<i64>,
    pub durability: Option<i64>,
    pub rarity: Option<i64>,
    pub effect: String,
    pub flavor: String,
}

impl CardRecord {
    /// Destination relative to the output root: `rules/<prefix>/<slug>.md`.
    /// Deliberately the same `<prefix>/<slug>` shape as [`Edition::image_rel_path`].
    pub fn text_rel_path(&self) -> PathBuf {
        rel_path("rules", &self.edition.set_prefix, &self.edition.slug, "md")
    }

    /// Render the card as a self-contained Markdown page for holistic reading.
    pub fn to_markdown(&self) -> String {
        let e = &self.edition;
        let mut s = format!("# {}\n\n", e.card_name);

        let mut typeline = self.types.join(" · ");
        if !self.subtypes.is_empty() {
            typeline.push_str(&format!(" — {}", self.subtypes.join(" · ")));
        }
        if !typeline.is_empty() {
            s.push_str(&format!("- **Type:** {typeline}\n"));
        }
        if !self.classes.is_empty() {
            s.push_str(&format!("- **Class:** {}\n", self.classes.join(" · ")));
        }
        if !self.elements.is_empty() {
            s.push_str(&format!("- **Element:** {}\n", self.elements.join(" · ")));
        }
        s.push_str(&format!("- **Cost:** {}\n", self.cost));
        if let Some(level) = self.level {
            s.push_str(&format!("- **Level:** {level}\n"));
        }

        let mut stats = Vec::new();
        if let Some(p) = self.power {
            stats.push(format!("Power {p}"));
        }
        if let Some(l) = self.life {
            stats.push(format!("Life {l}"));
        }
        if let Some(d) = self.durability {
            stats.push(format!("Durability {d}"));
        }
        if !stats.is_empty() {
            s.push_str(&format!("- **Stats:** {}\n", stats.join(" / ")));
        }

        let mut printing = e.set_name.clone();
        if !e.set_prefix.is_empty() {
            printing.push_str(&format!(" ({})", e.set_prefix));
        }
        if !e.collector_number.is_empty() {
            printing.push_str(&format!(" #{}", e.collector_number));
        }
        if let Some(r) = self.rarity {
            printing.push_str(&format!(" · rarity {r}"));
        }
        s.push_str(&format!("- **Printing:** {printing}\n"));

        s.push_str("\n## Text\n\n");
        if self.effect.is_empty() {
            s.push_str("_(no rules text)_\n");
        } else {
            s.push_str(&self.effect);
            s.push('\n');
        }
        if !self.flavor.is_empty() {
            s.push_str(&format!("\n## Flavor\n\n{}\n", self.flavor));
        }
        s
    }
}

/// Replace any character that isn't ASCII-alphanumeric, `.`, `_`, or `-` with
/// `_`, so set prefixes and slugs are safe path components on every filesystem.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `<top>/<sanitized prefix>/<sanitized slug>.<ext>` — the shared layout used by
/// both the image and rules trees.
fn rel_path(top: &str, prefix: &str, slug: &str, ext: &str) -> PathBuf {
    Path::new(top)
        .join(sanitize(prefix))
        .join(format!("{}.{ext}", sanitize(slug)))
}

/// Build a blocking HTTP agent with sane connect/read timeouts.
pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build()
}

/// GET `url`, retrying transient failures with linear backoff.
fn get_with_retry(agent: &ureq::Agent, url: &str, tries: u32) -> Result<ureq::Response, String> {
    let mut last = String::new();
    for attempt in 1..=tries {
        match agent.get(url).call() {
            Ok(resp) => return Ok(resp),
            Err(err) => {
                last = err.to_string();
                if attempt < tries {
                    std::thread::sleep(Duration::from_millis(500 * u64::from(attempt)));
                }
            }
        }
    }
    Err(format!("{url}: {last}"))
}

/// A string field on a JSON object, or `default` if absent.
fn field(obj: Option<&Value>, key: &str, default: &str) -> String {
    obj.and_then(|o| o.get(key))
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

/// A JSON array of strings, or empty.
fn str_vec(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// An integer field that may be encoded as a JSON number or a numeric string.
fn opt_num(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

/// Format the `cost` object as `<value> (<type>)`, e.g. `3 (reserve)`.
fn format_cost(card: &Value) -> String {
    if let Some(cost) = card.get("cost") {
        let value = cost.get("value").and_then(|v| {
            v.as_str()
                .map(String::from)
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        });
        let ty = cost.get("type").and_then(Value::as_str);
        if let (Some(value), Some(ty)) = (value, ty) {
            return format!("{value} ({ty})");
        }
    }
    "—".to_string()
}

/// Extract one card's editions. `require_image` drops printings with no art
/// (used by the image mirror); the rules mirror keeps every printing.
fn editions_of(card: &Value, require_image: bool) -> Vec<Edition> {
    let name = field(Some(card), "name", "");
    let Some(editions) = card.get("editions").and_then(Value::as_array) else {
        return Vec::new();
    };
    editions
        .iter()
        .filter_map(|e| {
            let slug = e.get("slug").and_then(Value::as_str)?;
            let image_path = field(Some(e), "image", "");
            if require_image && image_path.is_empty() {
                return None;
            }
            let set = e.get("set");
            Some(Edition {
                card_name: name.clone(),
                set_name: field(set, "name", ""),
                set_prefix: field(set, "prefix", "UNKNOWN"),
                slug: slug.to_string(),
                collector_number: field(Some(e), "collector_number", ""),
                image_path,
            })
        })
        .collect()
}

/// Editions (with images) from one page of the search response.
fn parse_page(json: &Value) -> Vec<Edition> {
    json.get("data")
        .and_then(Value::as_array)
        .map(|cards| cards.iter().flat_map(|c| editions_of(c, true)).collect())
        .unwrap_or_default()
}

/// Card records (one per printing, with rules text) from one page.
fn parse_page_records(json: &Value) -> Vec<CardRecord> {
    let Some(cards) = json.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for card in cards {
        let types = str_vec(card.get("types"));
        let subtypes = str_vec(card.get("subtypes"));
        let classes = str_vec(card.get("classes"));
        let elements = str_vec(card.get("elements"));
        let cost = format_cost(card);
        let level = opt_num(card.get("level"));
        let power = opt_num(card.get("power"));
        let life = opt_num(card.get("life"));
        let durability = opt_num(card.get("durability"));
        let effect = field(Some(card), "effect_raw", "");
        let flavor = field(Some(card), "flavor", "");

        for edition in editions_of(card, false) {
            // rarity is edition-level; recover it from the matching entry.
            let rarity = card
                .get("editions")
                .and_then(Value::as_array)
                .and_then(|es| {
                    es.iter()
                        .find(|e| field(Some(e), "slug", "") == edition.slug)
                })
                .and_then(|e| opt_num(e.get("rarity")));
            out.push(CardRecord {
                edition,
                types: types.clone(),
                subtypes: subtypes.clone(),
                classes: classes.clone(),
                elements: elements.clone(),
                cost: cost.clone(),
                level,
                power,
                life,
                durability,
                rarity,
                effect: effect.clone(),
                flavor: flavor.clone(),
            });
        }
    }
    out
}

/// Fetch and parse the JSON body at `url`.
fn fetch_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    let body = get_with_retry(agent, url, 5)?
        .into_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// Page the whole search endpoint, applying `parse` to each page. `on_page` is
/// called after each page with `(page, total_pages, running_total)`.
fn fetch_pages<T>(
    agent: &ureq::Agent,
    parse: impl Fn(&Value) -> Vec<T>,
    mut on_page: impl FnMut(u32, u32, usize),
) -> Result<Vec<T>, String> {
    let page_url = |page: u32| format!("{API_HOST}/cards/search?page={page}&page_size={PAGE_SIZE}");

    let first = fetch_json(agent, &page_url(1))?;
    let total_pages = first
        .get("total_pages")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;

    let mut items = parse(&first);
    on_page(1, total_pages, items.len());
    for page in 2..=total_pages {
        let json = fetch_json(agent, &page_url(page))?;
        items.extend(parse(&json));
        on_page(page, total_pages, items.len());
    }
    Ok(items)
}

/// Enumerate every card edition that has an image.
pub fn fetch_all_editions(
    agent: &ureq::Agent,
    on_page: impl FnMut(u32, u32, usize),
) -> Result<Vec<Edition>, String> {
    fetch_pages(agent, parse_page, on_page)
}

/// Enumerate every card printing together with its rules text.
pub fn fetch_all_records(
    agent: &ureq::Agent,
    on_page: impl FnMut(u32, u32, usize),
) -> Result<Vec<CardRecord>, String> {
    fetch_pages(agent, parse_page_records, on_page)
}

/// Outcome of writing a single file.
pub enum Fetched {
    /// Destination already existed and was non-empty; nothing was written.
    Skipped,
    /// The file was fetched/written and committed to its final path.
    Downloaded,
    /// The operation failed; the message describes why.
    Failed(String),
}

/// Return `Skipped` if `dest` already exists non-empty; otherwise ensure its
/// parent directory exists. `None` means "proceed with the write".
fn prepare(dest: &Path) -> Option<Fetched> {
    if let Ok(meta) = fs::metadata(dest) {
        if meta.len() > 0 {
            return Some(Fetched::Skipped);
        }
    }
    if let Some(parent) = dest.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return Some(Fetched::Failed(format!(
                "mkdir {}: {err}",
                parent.display()
            )));
        }
    }
    None
}

/// Commit `part` to `dest` atomically, cleaning up `part` on failure.
fn commit(part: &Path, dest: &Path) -> Result<(), String> {
    fs::rename(part, dest).map_err(|err| {
        let _ = fs::remove_file(part);
        format!("commit {}: {err}", dest.display())
    })
}

/// Download one edition's image into `root`, **resumably and atomically**.
///
/// - If the destination already exists and is non-empty, returns [`Fetched::Skipped`]
///   without any network access — this makes re-runs cheap and lets an interrupted
///   run pick up where it left off.
/// - Otherwise the body streams to a sibling `.part` file and is only
///   [`fs::rename`]d into place after a fully successful, synced write. That
///   rename is atomic, so an interrupt (Ctrl-C, crash, power loss) can never leave
///   a truncated file at the real path — at worst a stray `.part` remains, which
///   the next run overwrites.
pub fn download_edition(agent: &ureq::Agent, root: &Path, e: &Edition) -> Fetched {
    let dest = root.join(e.image_rel_path());
    if let Some(outcome) = prepare(&dest) {
        return outcome;
    }

    let part = dest.with_extension(format!("{}.part", e.ext()));
    let resp = match get_with_retry(agent, &e.image_url(), 4) {
        Ok(r) => r,
        Err(err) => return Fetched::Failed(err),
    };
    let write = (|| -> io::Result<()> {
        let mut reader = resp.into_reader();
        let mut file = fs::File::create(&part)?;
        io::copy(&mut reader, &mut file)?;
        file.sync_all()
    })();
    if let Err(err) = write {
        let _ = fs::remove_file(&part);
        return Fetched::Failed(format!("write {}: {err}", part.display()));
    }
    match commit(&part, &dest) {
        Ok(()) => Fetched::Downloaded,
        Err(msg) => Fetched::Failed(msg),
    }
}

/// Write one card record's Markdown into `root`, with the same resume + atomic
/// guarantees as [`download_edition`] (skip-existing, `.part` then rename).
pub fn write_card_text(root: &Path, rec: &CardRecord) -> Fetched {
    let dest = root.join(rec.text_rel_path());
    if let Some(outcome) = prepare(&dest) {
        return outcome;
    }

    let part = dest.with_extension("md.part");
    // No fsync here: these are thousands of tiny files, and fsync-per-file is the
    // dominant cost. The atomic rename still guarantees no truncated file at the
    // final path on an interrupt (Ctrl-C/kill); only a power-loss could lose an
    // unsynced write, which for a rebuildable local mirror is not worth the ~100x
    // slowdown.
    if let Err(err) = fs::write(&part, rec.to_markdown()) {
        let _ = fs::remove_file(&part);
        return Fetched::Failed(format!("write {}: {err}", part.display()));
    }
    match commit(&part, &dest) {
        Ok(()) => Fetched::Downloaded,
        Err(msg) => Fetched::Failed(msg),
    }
}

/// Write `manifest.csv` under `root`, mapping every edition to its local image
/// and source URL. Written up front from the full enumeration, so the mapping is
/// complete even if the image downloads are later interrupted.
pub fn write_manifest(root: &Path, editions: &[Edition]) -> io::Result<()> {
    let mut csv =
        String::from("card_name,set_name,set_prefix,collector_number,rel_path,source_url\n");
    for e in editions {
        let rel = e.image_rel_path().to_string_lossy().replace('\\', "/");
        csv.push_str(&format!(
            "\"{}\",\"{}\",{},{},{},{}\n",
            e.card_name.replace('"', "\"\""),
            e.set_name.replace('"', "\"\""),
            sanitize(&e.set_prefix),
            e.collector_number,
            rel,
            e.image_url(),
        ));
    }
    fs::write(root.join("manifest.csv"), csv)
}
