//! Font showcase — compare candidate UI typefaces side by side on card-like sample text, so the
//! "feel" and small-text legibility can be judged directly rather than from descriptions.
//!
//! Run: `cargo run -p cardtable --example fontshow`
//!
//! Most candidates are free/OFL fonts that aren't on this machine. Drop their `.ttf` files into
//! `crates/cardtable/examples/fonts/` (exact filenames + sources are in that folder's `README.md`) and
//! re-run — each present font renders a sample block; missing ones show a "drop me here" placeholder.
//! Inter (already bundled) and Cascadia Mono (an OFL mono usually installed on Windows) work out of
//! the box.

use bevy::prelude::*;

const FELT: Color = Color::srgb(0.08, 0.09, 0.11);
const PANEL: Color = Color::srgb(0.14, 0.15, 0.18);
const INK: Color = Color::srgb(0.93, 0.94, 0.96);
const MUTED: Color = Color::srgb(0.62, 0.66, 0.72);
const WARN: Color = Color::srgb(0.85, 0.60, 0.30);
const GOOD: Color = Color::srgb(0.45, 0.80, 0.65);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "cardtable - font showcase".into(),
                resolution: (1180u32, 900u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

/// One candidate: display name, a one-line note, and the file paths to try in order.
struct Candidate {
    name: &'static str,
    blurb: &'static str,
    paths: Vec<String>,
}

fn setup(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    commands.spawn(Camera2d);

    let dir = env!("CARGO_MANIFEST_DIR"); // crates/cardtable
    let ex = format!("{dir}/examples/fonts");

    // The label/UI font for headings and notes — Inter from the crate (else Bevy's default font).
    let (label, _) = load_first(&mut fonts, &[format!("{dir}/fonts/Inter-Regular.ttf")]);

    let candidates = vec![
        Candidate {
            name: "Inter (current)",
            blurb: "Neutral, app-like; purpose-built for screen UI, superb at small sizes.",
            paths: vec![
                format!("{dir}/fonts/Inter-Regular.ttf"),
                format!("{ex}/Inter-Regular.ttf"),
            ],
        },
        Candidate {
            name: "Atkinson Hyperlegible",
            blurb: "Engineered for max legibility + character distinction; great for tiny stats.",
            paths: vec![format!("{ex}/AtkinsonHyperlegible-Regular.ttf")],
        },
        Candidate {
            name: "IBM Plex Sans",
            blurb: "Clean with more personality; pairs naturally with Plex Mono for numbers.",
            paths: vec![format!("{ex}/IBMPlexSans-Regular.ttf")],
        },
        Candidate {
            name: "IBM Plex Mono (numbers)",
            blurb: "Mono companion: tabular digits so stat columns line up.",
            paths: vec![
                format!("{ex}/IBMPlexMono-Regular.ttf"),
                // An OFL mono usually installed on Windows, as a stand-in to see number alignment.
                "C:/Windows/Fonts/CascadiaMono.ttf".into(),
            ],
        },
        Candidate {
            name: "Source Sans 3",
            blurb: "Warm humanist; very readable, a touch friendlier than Inter.",
            paths: vec![format!("{ex}/SourceSans3-Regular.ttf")],
        },
        Candidate {
            name: "Figtree",
            blurb: "Friendly geometric; less corporate, suits the 'servant, not warden' feel.",
            paths: vec![format!("{ex}/Figtree-Regular.ttf")],
        },
        Candidate {
            name: "Nunito Sans",
            blurb: "Soft, rounded, approachable.",
            paths: vec![format!("{ex}/NunitoSans-Regular.ttf")],
        },
    ];

    // Resolve each candidate's handle up front (avoids borrowing `fonts` inside the UI closure).
    let resolved: Vec<(Candidate, Option<Handle<Font>>, Option<String>)> = candidates
        .into_iter()
        .map(|c| {
            let (handle, found) = load_first(&mut fonts, &c.paths);
            (c, handle, found)
        })
        .collect();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(14.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            line(root, "Font showcase", 22.0, &label, INK);
            line(
                root,
                "Same card-like text in each candidate. Drop TTFs in examples/fonts/ (see README) to fill in the rest.",
                13.0,
                &label,
                MUTED,
            );
            for (cand, handle, found) in &resolved {
                block(root, &label, cand, handle, found);
            }
        });
}

/// One candidate's block: heading + load status (in the label font), then the sample lines rendered
/// in the candidate font — or a placeholder if it isn't on disk yet.
fn block(
    parent: &mut ChildSpawnerCommands,
    label: &Option<Handle<Font>>,
    cand: &Candidate,
    sample: &Option<Handle<Font>>,
    found: &Option<String>,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(14.0)),
                row_gap: Val::Px(5.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
        ))
        .with_children(|b| {
            line(b, cand.name, 16.0, label, INK);
            line(b, cand.blurb, 12.0, label, MUTED);
            if let Some(src) = found {
                line(b, &format!("loaded: {src}"), 11.0, label, GOOD);
                line(b, "Ironfang Warband", 18.0, sample, INK);
                line(b, "Knight | Vanguard", 13.0, sample, MUTED);
                line(
                    b,
                    "Might 12   Vit 8   Tough 5   HP 24/30",
                    14.0,
                    sample,
                    INK,
                );
                line(b, "Il1  O0Q  rn m  5S 2Z 8B  ()[]{}", 13.0, sample, INK);
                line(
                    b,
                    "When this unit breaches, deal 1 damage to each foe in the Rearguard.",
                    12.0,
                    sample,
                    INK,
                );
                line(b, "12 cards", 15.0, sample, GOOD);
            } else {
                let want = cand.paths.first().cloned().unwrap_or_default();
                line(
                    b,
                    &format!("not found - drop a .ttf at: {want}"),
                    13.0,
                    label,
                    WARN,
                );
            }
        });
}

/// Spawn one line of text in `font` (or the default font if `None`).
fn line(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    size: f32,
    font: &Option<Handle<Font>>,
    color: Color,
) {
    let mut text_font = TextFont {
        font_size: FontSize::Px(size),
        ..default()
    };
    if let Some(handle) = font {
        text_font.font = FontSource::Handle(handle.clone());
    }
    parent.spawn((Text::new(text.to_string()), text_font, TextColor(color)));
}

/// Try each path in turn; load the first that reads into an `Assets<Font>` handle.
fn load_first(
    fonts: &mut Assets<Font>,
    paths: &[String],
) -> (Option<Handle<Font>>, Option<String>) {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            return (Some(fonts.add(Font::from_bytes(bytes))), Some(path.clone()));
        }
    }
    (None, None)
}
