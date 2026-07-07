//! Grand Archive TCG reference-data tooling.
//!
//! The public card API at <https://api.gatcg.com> exposes the whole card library
//! through a paginated search endpoint, and every card *edition* (printing) links
//! an image. This crate enumerates that library and mirrors the images to a local
//! directory. It is network + filesystem only; no game crate depends on it.
//!
//! The download is **resumable and safe to interrupt** — see [`download_edition`].

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

/// One printing of a card — the unit that owns an image.
#[derive(Debug, Clone)]
pub struct Edition {
    pub card_name: String,
    pub set_name: String,
    pub set_prefix: String,
    pub slug: String,
    pub collector_number: String,
    /// Path component returned by the API, e.g. `/cards/images/<hash>.jpg`.
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
    pub fn rel_path(&self) -> PathBuf {
        Path::new("images")
            .join(sanitize(&self.set_prefix))
            .join(format!("{}.{}", sanitize(&self.slug), self.ext()))
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

/// Extract the editions (with images) from one page of the search response.
fn parse_page(json: &Value) -> Vec<Edition> {
    let mut out = Vec::new();
    let Some(cards) = json.get("data").and_then(Value::as_array) else {
        return out;
    };
    for card in cards {
        let card_name = card
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let Some(editions) = card.get("editions").and_then(Value::as_array) else {
            continue;
        };
        for e in editions {
            let image_path = e.get("image").and_then(Value::as_str).unwrap_or("");
            if image_path.is_empty() {
                continue; // some editions carry no art
            }
            let set = e.get("set");
            let field = |obj: Option<&Value>, key: &str, default: &str| {
                obj.and_then(|o| o.get(key))
                    .and_then(Value::as_str)
                    .unwrap_or(default)
                    .to_string()
            };
            out.push(Edition {
                card_name: card_name.clone(),
                set_name: field(set, "name", ""),
                set_prefix: field(set, "prefix", "UNKNOWN"),
                slug: e
                    .get("slug")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                collector_number: e
                    .get("collector_number")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                image_path: image_path.to_string(),
            });
        }
    }
    out
}

/// Enumerate every card edition in the library by paging the search endpoint.
///
/// `on_page` is invoked after each page with `(page, total_pages, running_total)`
/// for progress reporting.
pub fn fetch_all_editions(
    agent: &ureq::Agent,
    mut on_page: impl FnMut(u32, u32, usize),
) -> Result<Vec<Edition>, String> {
    let page_url = |page: u32| format!("{API_HOST}/cards/search?page={page}&page_size={PAGE_SIZE}");

    let body = get_with_retry(agent, &page_url(1), 5)?
        .into_string()
        .map_err(|e| e.to_string())?;
    let first: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let total_pages = first
        .get("total_pages")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;

    let mut editions = parse_page(&first);
    on_page(1, total_pages, editions.len());

    for page in 2..=total_pages {
        let body = get_with_retry(agent, &page_url(page), 5)?
            .into_string()
            .map_err(|e| e.to_string())?;
        let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        editions.extend(parse_page(&json));
        on_page(page, total_pages, editions.len());
    }
    Ok(editions)
}

/// Outcome of a single image download.
pub enum Fetched {
    /// Destination already existed and was non-empty; nothing was fetched.
    Skipped,
    /// The image was downloaded and committed to its final path.
    Downloaded,
    /// The download failed; the message describes why.
    Failed(String),
}

/// Download one edition's image into `root`, **resumably and atomically**.
///
/// - If the destination already exists and is non-empty, returns [`Fetched::Skipped`]
///   without any network access — this is what makes re-runs cheap and lets an
///   interrupted run pick up where it left off.
/// - Otherwise the body is streamed to a sibling `<dest>.part` file and only
///   [`fs::rename`]d into place after a fully successful write. A rename on the
///   same filesystem is atomic, so an interrupt (Ctrl-C, crash, power loss) can
///   never leave a truncated file at the real path — at worst a stray `.part`
///   remains, which the next run simply overwrites.
pub fn download_edition(agent: &ureq::Agent, root: &Path, e: &Edition) -> Fetched {
    let dest = root.join(e.rel_path());

    if let Ok(meta) = fs::metadata(&dest) {
        if meta.len() > 0 {
            return Fetched::Skipped;
        }
    }

    if let Some(parent) = dest.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return Fetched::Failed(format!("mkdir {}: {err}", parent.display()));
        }
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

    if let Err(err) = fs::rename(&part, &dest) {
        let _ = fs::remove_file(&part);
        return Fetched::Failed(format!("commit {}: {err}", dest.display()));
    }
    Fetched::Downloaded
}

/// Write `manifest.csv` under `root`, mapping every edition to its local file
/// and source URL. Written up front from the full enumeration, so the mapping is
/// complete even if the image downloads are later interrupted.
pub fn write_manifest(root: &Path, editions: &[Edition]) -> io::Result<()> {
    let mut csv =
        String::from("card_name,set_name,set_prefix,collector_number,rel_path,source_url\n");
    for e in editions {
        let rel = e.rel_path().to_string_lossy().replace('\\', "/");
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
