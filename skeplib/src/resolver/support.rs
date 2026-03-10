use std::path::Path;

use super::ResolveError;

pub(super) fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.chars().count();
    }
    if b.is_empty() {
        return a.chars().count();
    }
    let b_chars = b.chars().collect::<Vec<_>>();
    let mut prev = (0..=b_chars.len()).collect::<Vec<_>>();
    let mut cur = vec![0usize; b_chars.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b_chars.len()]
}

pub(super) fn suggest_name<'a>(
    needle: &str,
    haystack: impl Iterator<Item = &'a str>,
) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for cand in haystack {
        let d = levenshtein(needle, cand);
        if d <= 2 {
            match best {
                Some((_, bd)) if d >= bd => {}
                _ => best = Some((cand, d)),
            }
        }
    }
    best.map(|(s, _)| s.to_string())
}

pub(super) fn with_importer_context(
    mut err: ResolveError,
    importer_id: &str,
    importer_path: &Path,
    import_text: &str,
    importer_source: &str,
) -> ResolveError {
    if let Some((line, col)) = find_import_line_col(importer_source, import_text) {
        err = err.with_line_col(line, col);
    }
    err.message = format!(
        "{} (while resolving import `{}` in module `{}` at {})",
        err.message,
        import_text,
        importer_id,
        importer_path.display()
    );
    err
}

fn find_import_line_col(source: &str, import_text: &str) -> Option<(usize, usize)> {
    let pat_import = format!("import {import_text}");
    let pat_from = format!("from {import_text} import");
    for (idx, line) in source.lines().enumerate() {
        if let Some(col) = line
            .find(&pat_import)
            .or_else(|| line.find(&pat_from))
            .map(|v| v + 1)
        {
            return Some((idx + 1, col));
        }
    }
    None
}
