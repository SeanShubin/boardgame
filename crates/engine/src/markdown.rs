//! Markdown table padding — align every table's columns so pipes line up in monospace.
//!
//! [`pad_tables`] is the single source of truth for the repo's table style: it is called both by
//! the `pad_tables` example (which walks the tree padding `.md` files in place) and by generated
//! docs (e.g. `deckbound`'s handbook), so a generator emits tables already in the padded form the
//! example would produce. That keeps the two from fighting: the example finds generated docs already
//! aligned and rewrites nothing, and a byte-exact golden test on the generated doc still holds.
//!
//! The transform is idempotent — `pad_tables(&pad_tables(s)) == pad_tables(s)`.

/// Pad every markdown table in `content` so its columns align, leaving all non-table text untouched.
///
/// A table is a run of consecutive `|`-delimited rows whose second row is a separator (`---`,
/// `:---`, `---:`, `:---:`). Alignment markers are preserved; each column is widened to its widest
/// cell (minimum 3, so separators stay at least `---`).
pub fn pad_tables(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;

    while i < lines.len() {
        if parse_row(lines[i]).is_some() {
            // Collect the consecutive run of table rows.
            let mut table_lines = Vec::new();
            while i < lines.len() && parse_row(lines[i]).is_some() {
                table_lines.push(lines[i]);
                i += 1;
            }

            // Only pad a run whose second row is a separator; otherwise it isn't a real table.
            if table_lines.len() >= 2 && is_separator(&parse_row(table_lines[1]).unwrap()) {
                result.extend(pad_table(&table_lines));
            } else {
                result.extend(table_lines.iter().map(|s| s.to_string()));
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Parse a markdown table row into trimmed cell contents.
/// Returns `None` if the line isn't a table row.
fn parse_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 2 {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    Some(cells)
}

/// Check if all cells in a row are separator patterns like `---`, `:---`, `---:`, `:---:`.
fn is_separator(cells: &[String]) -> bool {
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|c| {
        let mut chars = c.chars();
        // Must have at least one character.
        let first = match chars.next() {
            Some(ch) => ch,
            None => return false,
        };
        // Strip optional leading colon.
        let rest_start = if first == ':' {
            match chars.next() {
                Some(ch) => ch,
                None => return false, // just ":"
            }
        } else {
            first
        };
        // Must have at least one dash.
        if rest_start != '-' {
            return false;
        }
        // Remaining chars: dashes, then optional trailing colon.
        let mut saw_colon = false;
        for ch in chars {
            if saw_colon {
                return false; // something after the trailing colon
            }
            if ch == '-' {
                continue;
            } else if ch == ':' {
                saw_colon = true;
            } else {
                return false;
            }
        }
        true
    })
}

/// Visual width of a string — count chars, not bytes.
/// All characters in this project's docs are single-width in Western monospace fonts.
fn visual_width(s: &str) -> usize {
    s.chars().count()
}

/// Format a separator cell preserving alignment markers (`:---`, `---:`, `:---:`).
fn format_separator_cell(original: &str, width: usize) -> String {
    let left = original.starts_with(':');
    let right = original.ends_with(':');
    let colon_width = if left { 1 } else { 0 } + if right { 1 } else { 0 };
    let dash_count = if width > colon_width {
        width - colon_width
    } else {
        1
    };
    let mut s = String::with_capacity(width);
    if left {
        s.push(':');
    }
    for _ in 0..dash_count {
        s.push('-');
    }
    if right {
        s.push(':');
    }
    s
}

/// Pad a table so all columns align.
fn pad_table(lines: &[&str]) -> Vec<String> {
    // Parse all rows.
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in lines {
        match parse_row(line) {
            Some(cells) => rows.push(cells),
            None => return lines.iter().map(|s| s.to_string()).collect(),
        }
    }
    if rows.len() < 2 {
        return lines.iter().map(|s| s.to_string()).collect();
    }

    // Normalize column count.
    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    // Calculate max width per column (skip the separator row).
    let mut col_widths = vec![0usize; max_cols];
    for (i, row) in rows.iter().enumerate() {
        if i == 1 && is_separator(row) {
            continue;
        }
        for (j, cell) in row.iter().enumerate() {
            col_widths[j] = col_widths[j].max(visual_width(cell));
        }
    }

    // Minimum width of 3 so separators are at least `---`.
    for w in &mut col_widths {
        if *w < 3 {
            *w = 3;
        }
    }

    // Format each row.
    let mut result = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let mut line = String::from("|");
        if i == 1 && is_separator(row) {
            for (j, cell) in row.iter().enumerate() {
                line.push(' ');
                line.push_str(&format_separator_cell(cell, col_widths[j]));
                line.push(' ');
                line.push('|');
            }
        } else {
            for (j, cell) in row.iter().enumerate() {
                line.push(' ');
                line.push_str(cell);
                let padding = col_widths[j] - visual_width(cell);
                for _ in 0..padding {
                    line.push(' ');
                }
                line.push(' ');
                line.push('|');
            }
        }
        result.push(line);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::pad_tables;

    #[test]
    fn pads_columns_to_the_widest_cell() {
        // col 0's widest cell is 1 char but the min column width is 3; col 1's is `yyyy` (4).
        let input = "| A | Bee |\n| --- | ---: |\n| x | yyyy |\n";
        let out = pad_tables(input);
        assert_eq!(out, "| A   | Bee  |\n| --- | ---: |\n| x   | yyyy |\n");
    }

    #[test]
    fn is_idempotent() {
        let input = "| Cardset | Cards |\n| --- | ---: |\n| Baseline | 1 |\n| Iron — Wall | 11 |\n";
        let once = pad_tables(input);
        assert_eq!(once, pad_tables(&once), "padding must be a fixed point");
    }

    #[test]
    fn leaves_non_table_text_untouched() {
        let input = "# Title\n\nSome prose with a | pipe but no table.\n";
        assert_eq!(pad_tables(input), input);
    }

    #[test]
    fn preserves_alignment_markers() {
        // Columns shrink to the min width of 3, but the left/right colon markers survive.
        let input = "| L | C | R |\n| :--- | :---: | ---: |\n| a | b | c |\n";
        let out = pad_tables(input);
        assert!(out.contains("| :-- | :-: | --: |"), "got: {out}");
    }
}
