//! FF-20: the fitness inventory matches the catalogue, every active check is
//! implemented and wired into CI, and no check is removed or downgraded
//! without an ADR trailer (GR-14).

use super::trailers::{adr_exists, adr_refs};
use super::{commits_to_check, git};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const INVENTORY: &str = "architecture/fitness-inventory.toml";
const CATALOGUE: &str = "docs/architecture/fitness-functions.md";
const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const DENY: &str = "deny.toml";

/// Check implementations that exist. An `active` entry must name one of these.
pub const IMPLEMENTED: &[&str] = &[
    "xtask:deps",
    "xtask:purity",
    "xtask:wasm",
    "xtask:platform",
    "cargo-deny:licenses",
    "cargo-deny:advisories",
    "xtask:trailers",
    "xtask:inventory",
    "xtask:docs-links",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub phase: u32,
    pub active: bool,
    pub check: Option<String>,
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub current_phase: u32,
    pub entries: BTreeMap<String, Entry>,
}

pub fn run(root: &Path, base: Option<&str>) -> Result<(), String> {
    let inv = parse(&read(root, INVENTORY)?)?;
    let catalogue = catalogue_phases(&read(root, CATALOGUE)?)?;
    let mut errors = consistency(&inv, &catalogue);
    errors.extend(wiring(root, &inv));

    if let Some(b) = base
        && let Ok(old_text) = git(root, &["show", &format!("{b}:{INVENTORY}")])
    {
        let shrink = shrinkage(&parse(&old_text)?, &inv);
        if !shrink.is_empty() && !range_has_valid_trailer(root, base)? {
            errors.extend(
                shrink
                    .into_iter()
                    .map(|s| format!("{s} without an ADR trailer")),
            );
        }
    }

    if errors.is_empty() {
        let active = inv.entries.values().filter(|e| e.active).count();
        println!(
            "FF-20 inventory: ok ({} checks, {active} active)",
            inv.entries.len()
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("cannot read {rel}: {e}"))
}

/// Minimal parser for the inventory format: `[ID]` sections with
/// `key = "string"` or `key = integer` lines.
pub fn parse(text: &str) -> Result<Inventory, String> {
    let mut inv = Inventory::default();
    let mut section: Option<String> = None;
    let mut raw: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if raw.contains_key(name) {
                return Err(format!("{INVENTORY}:{}: duplicate section [{name}]", n + 1));
            }
            raw.insert(name.to_string(), BTreeMap::new());
            section = Some(name.to_string());
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{INVENTORY}:{}: expected key = value", n + 1))?;
        let sec = section
            .as_ref()
            .ok_or_else(|| format!("{INVENTORY}:{}: key outside a section", n + 1))?;
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        raw.get_mut(sec)
            .unwrap()
            .insert(key.trim().to_string(), value.to_string());
    }

    let meta = raw
        .remove("meta")
        .ok_or("inventory has no [meta] section")?;
    inv.current_phase = meta
        .get("current_phase")
        .and_then(|v| v.parse().ok())
        .ok_or("[meta] current_phase missing or not an integer")?;

    for (id, kv) in raw {
        if !id.starts_with("FF-") {
            return Err(format!("unexpected section [{id}]"));
        }
        let phase = kv
            .get("phase")
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("[{id}] phase missing or not an integer"))?;
        let active = match kv.get("status").map(String::as_str) {
            Some("active") => true,
            Some("planned") => false,
            other => {
                return Err(format!(
                    "[{id}] status must be active or planned, got {other:?}"
                ));
            }
        };
        inv.entries.insert(
            id,
            Entry {
                phase,
                active,
                check: kv.get("check").cloned(),
            },
        );
    }
    Ok(inv)
}

/// Reads `| FF-xx | ... | Phase N | ... |` rows from the catalogue table.
pub fn catalogue_phases(markdown: &str) -> Result<BTreeMap<String, u32>, String> {
    let mut phase_col = None;
    let mut map = BTreeMap::new();
    for line in markdown.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 3 {
            continue;
        }
        if cells.get(1) == Some(&"Id") {
            phase_col = cells.iter().position(|c| *c == "Active from");
            continue;
        }
        let Some(col) = phase_col else { continue };
        let id = cells[1];
        if !id.starts_with("FF-") {
            continue;
        }
        let phase = cells
            .get(col)
            .and_then(|c| c.strip_prefix("Phase "))
            .and_then(|n| n.trim().parse().ok())
            .ok_or_else(|| format!("{CATALOGUE}: cannot read phase for {id}"))?;
        if map.insert(id.to_string(), phase).is_some() {
            return Err(format!("{CATALOGUE}: duplicate row {id}"));
        }
    }
    if map.is_empty() {
        return Err(format!("{CATALOGUE}: no catalogue rows found"));
    }
    Ok(map)
}

pub fn consistency(inv: &Inventory, catalogue: &BTreeMap<String, u32>) -> Vec<String> {
    let mut errors = Vec::new();
    for (id, phase) in catalogue {
        match inv.entries.get(id) {
            None => errors.push(format!(
                "{id} is in the catalogue but missing from {INVENTORY}"
            )),
            Some(e) if e.phase != *phase => errors.push(format!(
                "{id}: phase {} in {INVENTORY} differs from Phase {phase} in the catalogue",
                e.phase
            )),
            _ => {}
        }
    }
    for (id, e) in &inv.entries {
        if !catalogue.contains_key(id) {
            errors.push(format!("{id} is in {INVENTORY} but not in the catalogue"));
        }
        if e.phase <= inv.current_phase && !e.active {
            errors.push(format!(
                "{id} belongs to phase {} (current phase {}) but is not active",
                e.phase, inv.current_phase
            ));
        }
        if e.active {
            match e.check.as_deref() {
                None => errors.push(format!("{id} is active but names no check")),
                Some(c) if !IMPLEMENTED.contains(&c) => {
                    errors.push(format!("{id} is active but check '{c}' is not implemented"))
                }
                _ => {}
            }
        }
    }
    errors
}

/// Active checks must actually run: cargo-deny sections exist and CI invokes the tools.
fn wiring(root: &Path, inv: &Inventory) -> Vec<String> {
    let ci = fs::read_to_string(root.join(CI_WORKFLOW)).unwrap_or_default();
    let deny = fs::read_to_string(root.join(DENY)).unwrap_or_default();
    let mut errors = Vec::new();
    for (id, e) in inv.entries.iter().filter(|(_, e)| e.active) {
        let Some(check) = e.check.as_deref() else {
            continue;
        };
        if let Some(section) = check.strip_prefix("cargo-deny:") {
            if !deny.contains(&format!("[{section}]")) {
                errors.push(format!("{id}: {DENY} has no [{section}] section"));
            }
            if !ci.contains("cargo-deny") {
                errors.push(format!("{id}: {CI_WORKFLOW} does not run cargo-deny"));
            }
        } else if check.starts_with("xtask:") && !ci.contains("xtask fitness all") {
            errors.push(format!(
                "{id}: {CI_WORKFLOW} does not run 'cargo xtask fitness all'"
            ));
        }
        // FF-03 is enforced by clippy with -D warnings; the xtask part only checks configuration.
        if check == "xtask:purity" && !(ci.contains("cargo clippy") && ci.contains("-D warnings")) {
            errors.push(format!(
                "{id}: {CI_WORKFLOW} does not run 'cargo clippy ... -D warnings'"
            ));
        }
    }
    errors
}

/// Removed entries and active -> planned downgrades relative to `old`.
pub fn shrinkage(old: &Inventory, new: &Inventory) -> Vec<String> {
    let mut out = Vec::new();
    for (id, e) in &old.entries {
        match new.entries.get(id) {
            None => out.push(format!("{id} was removed")),
            Some(n) if e.active && !n.active => out.push(format!("{id} was downgraded to planned")),
            _ => {}
        }
    }
    out
}

fn range_has_valid_trailer(root: &Path, base: Option<&str>) -> Result<bool, String> {
    for sha in commits_to_check(root, base)? {
        let msg = git(root, &["log", "-1", "--format=%B", &sha])?;
        let refs = adr_refs(&msg);
        if !refs.is_empty() && refs.iter().all(|id| adr_exists(root, id)) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INV: &str = r#"
[meta]
current_phase = 0

[FF-01]
phase = 1
status = "planned"

[FF-19]
phase = 0
status = "active"
check = "xtask:trailers"
"#;

    const CAT: &str = "\
| Id | Check | Rules | Tool | Criticality | Active from | Threshold |
|---|---|---|---|---|---|---|
| FF-01 | Matrix | GR-01 | xtask | Critical | Phase 1 | Binary |
| FF-19 | Trailers | GR-13 | xtask | Critical | Phase 0 | Binary |
";

    #[test]
    fn consistent_inventory_passes() {
        let inv = parse(INV).unwrap();
        assert!(consistency(&inv, &catalogue_phases(CAT).unwrap()).is_empty());
    }

    #[test]
    fn missing_catalogue_entry_fails() {
        let inv = parse(INV).unwrap();
        let cat = format!("{CAT}| FF-02 | Bans | GR-02 | deny | Critical | Phase 1 | Binary |\n");
        let errs = consistency(&inv, &catalogue_phases(&cat).unwrap());
        assert!(errs.iter().any(|e| e.contains("FF-02")), "{errs:?}");
    }

    #[test]
    fn due_check_that_is_planned_fails() {
        let inv =
            parse(&INV.replace("\"active\"\ncheck = \"xtask:trailers\"", "\"planned\"")).unwrap();
        let errs = consistency(&inv, &catalogue_phases(CAT).unwrap());
        assert!(
            errs.iter()
                .any(|e| e.contains("FF-19") && e.contains("not active")),
            "{errs:?}"
        );
    }

    #[test]
    fn active_check_must_be_implemented() {
        let inv = parse(&INV.replace("xtask:trailers", "xtask:imaginary")).unwrap();
        let errs = consistency(&inv, &catalogue_phases(CAT).unwrap());
        assert!(
            errs.iter().any(|e| e.contains("not implemented")),
            "{errs:?}"
        );
    }

    #[test]
    fn removal_and_downgrade_are_shrinkage() {
        let old = parse(INV).unwrap();
        let mut new = parse(INV).unwrap();
        new.entries.remove("FF-01");
        new.entries.get_mut("FF-19").unwrap().active = false;
        let s = shrinkage(&old, &new);
        assert_eq!(s.len(), 2, "{s:?}");
    }

    #[test]
    fn real_inventory_matches_real_catalogue() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let inv = parse(&read(root, INVENTORY).unwrap()).unwrap();
        let cat = catalogue_phases(&read(root, CATALOGUE).unwrap()).unwrap();
        assert_eq!(cat.len(), 33);
        let errs = consistency(&inv, &cat);
        assert!(errs.is_empty(), "{errs:?}");
    }
}
