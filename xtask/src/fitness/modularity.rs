//! FF-25 (advisory): coupling report, fan-in and fan-out per crate and per top-level
//! module. FF-26 (advisory): intra-crate module cycles (GR-07, module level).
//!
//! Crate edges come from the workspace manifests (normal dependencies). Module edges
//! come from `crate::` paths in each top-level module's files: `use crate::x::...`,
//! `use crate::{A, b::C}` and inline `crate::x::f()`. A name re-exported by the crate
//! root (`pub use x::{A, B}`) is attributed to module `x`. Known limitations: `super::`
//! paths and macro-generated paths are not followed, so the report can miss edges; it
//! never invents them. Both checks are reports and never fail the build.

use super::deps::workspace_manifests;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Module dependency graph of one crate: module -> modules it uses.
pub type ModuleGraph = BTreeMap<String, BTreeSet<String>>;

pub fn run_coupling(root: &Path) -> Result<(), String> {
    let manifests = workspace_manifests(root)?;
    let names: BTreeSet<String> = manifests.iter().map(|m| m.name.clone()).collect();
    let mut fan_in: BTreeMap<&str, usize> = names.iter().map(|n| (n.as_str(), 0)).collect();
    for m in &manifests {
        for d in m.normal.iter().filter(|d| names.contains(*d)) {
            *fan_in.get_mut(d.as_str()).expect("listed") += 1;
        }
    }
    println!("FF-25 coupling report (advisory; no threshold until trend data exists)");
    println!("  crate            fan-in  fan-out  instability");
    for m in &manifests {
        let out = m.normal.iter().filter(|d| names.contains(*d)).count();
        let inn = fan_in[m.name.as_str()];
        println!(
            "  {:<16} {:>6}  {:>7}  {:>11}",
            m.name,
            inn,
            out,
            instability(inn, out)
        );
    }
    for m in &manifests {
        let graph = crate_graph(&root.join("crates").join(&m.name))?;
        if graph.is_empty() {
            continue;
        }
        let mut inn: BTreeMap<&str, usize> = graph.keys().map(|k| (k.as_str(), 0)).collect();
        for deps in graph.values() {
            for d in deps {
                *inn.entry(d.as_str()).or_default() += 1;
            }
        }
        let cells: Vec<String> = graph
            .iter()
            .map(|(k, v)| format!("{k} {}/{}", inn[k.as_str()], v.len()))
            .collect();
        println!("  {} modules (in/out): {}", m.name, cells.join(", "));
    }
    Ok(())
}

pub fn run_module_cycles(root: &Path) -> Result<(), String> {
    let manifests = workspace_manifests(root)?;
    let mut total = 0;
    let mut lines = Vec::new();
    for m in &manifests {
        let graph = crate_graph(&root.join("crates").join(&m.name))?;
        for cycle in cycles(&graph) {
            total += 1;
            lines.push(format!("  {}: {}", m.name, cycle.join(" <-> ")));
        }
    }
    println!(
        "FF-26 intra-crate module cycles (advisory): {total} cycle(s) in {} crates",
        manifests.len()
    );
    for l in lines {
        println!("{l}");
    }
    Ok(())
}

fn instability(fan_in: usize, fan_out: usize) -> String {
    if fan_in + fan_out == 0 {
        "-".into()
    } else {
        format!("{:.2}", fan_out as f64 / (fan_in + fan_out) as f64)
    }
}

/// Reads `src/` of a crate: top-level modules are `src/x.rs` and `src/x/` (all files
/// below belong to `x`). The crate root (`lib.rs`, `main.rs`) supplies re-exports.
pub fn crate_graph(crate_dir: &Path) -> Result<ModuleGraph, String> {
    let src = crate_dir.join("src");
    let Ok(entries) = fs::read_dir(&src) else {
        return Ok(ModuleGraph::new());
    };
    let mut modules: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let mut roots = Vec::new();
    for p in entries.flatten().map(|e| e.path()) {
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if p.is_dir() {
            let mut files = Vec::new();
            collect_rs(&p, &mut files);
            modules.entry(stem).or_default().extend(files);
        } else if p.extension().is_some_and(|e| e == "rs") {
            if stem == "lib" || stem == "main" {
                roots.push(p);
            } else {
                modules.entry(stem).or_default().push(p);
            }
        }
    }
    let read = |p: &Path| fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let names: BTreeSet<String> = modules.keys().cloned().collect();
    let mut reexports = BTreeMap::new();
    for r in &roots {
        reexports.extend(reexport_map(&read(r)?, &names));
    }
    let mut graph = ModuleGraph::new();
    for (name, files) in &modules {
        let mut deps = BTreeSet::new();
        for f in files {
            deps.extend(module_refs(&read(f)?, &names, &reexports));
        }
        deps.remove(name);
        graph.insert(name.clone(), deps);
    }
    Ok(graph)
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for p in entries.flatten().map(|e| e.path()) {
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Source without `//` comments (line and doc comments).
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Single spaces only where needed: `crate::{ A,\n b }` -> `crate::{A,b}`, `X as Y` kept.
fn normalize(stmt: &str) -> String {
    let words: Vec<&str> = stmt.split_whitespace().collect();
    let mut out = String::new();
    for w in words {
        let glue = out.ends_with([':', '{', ',']) || w.starts_with([':', '{', '}', ',']);
        if !out.is_empty() && !glue {
            out.push(' ');
        }
        out.push_str(w);
    }
    out
}

/// `use` statements of a source file, each as one string without `use` and `;`.
fn use_statements(text: &str) -> Vec<String> {
    let code = strip_comments(text);
    let mut out = Vec::new();
    let mut rest = code.as_str();
    while let Some(i) = rest.find("use ") {
        let boundary = i == 0
            || rest[..i]
                .chars()
                .last()
                .is_some_and(|c| c.is_whitespace() || c == ';' || c == '{' || c == '}');
        let after = &rest[i + 4..];
        let end = after.find(';').unwrap_or(after.len());
        if boundary {
            out.push(normalize(&after[..end]));
        }
        rest = &after[end.min(after.len())..];
    }
    out
}

/// First path segments of a `use` tree body: `{a,b::c,d::{e,f}}` -> a, b, d; `x::y` -> x.
fn first_segments(tree: &str) -> Vec<String> {
    let tree = tree.trim();
    if let Some(inner) = tree.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        let mut out = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        for (i, c) in inner.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    out.extend(first_segments(&inner[start..i]));
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.extend(first_segments(&inner[start..]));
        return out;
    }
    let first = tree.split("::").next().unwrap_or("");
    let ident: String = first
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if ident.is_empty() {
        Vec::new()
    } else {
        vec![ident]
    }
}

/// Items re-exported by the crate root from a module: `pub use x::{A, B}` -> A, B -> x.
pub fn reexport_map(root_text: &str, modules: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for stmt in use_statements(root_text) {
        let Some((module, rest)) = stmt.split_once("::") else {
            continue;
        };
        if !modules.contains(module) {
            continue;
        }
        let items = match rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
            Some(inner) => inner.split(',').map(str::to_string).collect(),
            None => vec![rest.to_string()],
        };
        for item in items {
            let name = item.rsplit("::").next().unwrap_or("").to_string();
            let name = name.split(" as ").last().unwrap_or("").trim().to_string();
            if !name.is_empty() && name != "*" {
                map.insert(name, module.to_string());
            }
        }
    }
    map
}

/// Modules a file refers to through `crate::` paths.
pub fn module_refs(
    text: &str,
    modules: &BTreeSet<String>,
    reexports: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let resolve = |ident: &str| -> Option<String> {
        if modules.contains(ident) {
            Some(ident.to_string())
        } else {
            reexports.get(ident).cloned()
        }
    };
    let mut out = BTreeSet::new();
    for stmt in use_statements(text) {
        let stmt = stmt
            .trim_start_matches("pub(crate)")
            .trim_start_matches("pub");
        if let Some(tree) = stmt.strip_prefix("crate::") {
            out.extend(first_segments(tree).iter().filter_map(|s| resolve(s)));
        }
    }
    let code = strip_comments(text);
    for (i, _) in code.match_indices("crate::") {
        let before = code[..i].chars().last();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let after = &code[i + "crate::".len()..];
        if after.starts_with('{') {
            continue; // handled with the use statement
        }
        let ident: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if let Some(m) = resolve(&ident) {
            out.insert(m);
        }
    }
    out
}

/// Strongly connected components with more than one module (Tarjan), each sorted.
pub fn cycles(graph: &ModuleGraph) -> Vec<Vec<String>> {
    struct State<'a> {
        graph: &'a ModuleGraph,
        index: BTreeMap<&'a str, usize>,
        low: BTreeMap<&'a str, usize>,
        stack: Vec<&'a str>,
        on_stack: BTreeSet<&'a str>,
        next: usize,
        out: Vec<Vec<String>>,
    }
    fn visit<'a>(s: &mut State<'a>, v: &'a str) {
        s.index.insert(v, s.next);
        s.low.insert(v, s.next);
        s.next += 1;
        s.stack.push(v);
        s.on_stack.insert(v);
        let graph = s.graph;
        for w in graph.get(v).into_iter().flatten() {
            let w = w.as_str();
            if !s.index.contains_key(w) {
                visit(s, w);
                let lw = s.low[w];
                let lv = s.low.get_mut(v).expect("visited");
                *lv = (*lv).min(lw);
            } else if s.on_stack.contains(w) {
                let iw = s.index[w];
                let lv = s.low.get_mut(v).expect("visited");
                *lv = (*lv).min(iw);
            }
        }
        if s.low[v] == s.index[v] {
            let mut component = Vec::new();
            while let Some(w) = s.stack.pop() {
                s.on_stack.remove(w);
                component.push(w.to_string());
                if w == v {
                    break;
                }
            }
            if component.len() > 1 {
                component.sort();
                s.out.push(component);
            }
        }
    }
    let mut s = State {
        graph,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        next: 0,
        out: Vec::new(),
    };
    for v in graph.keys() {
        if !s.index.contains_key(v.as_str()) {
            visit(&mut s, v);
        }
    }
    s.out.sort();
    s.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn module_references_follow_crate_paths_and_reexports() {
        let modules = set(&["chord", "note_name", "pitch", "error"]);
        let root = "mod chord;\npub use chord::{Chord, ChordQuality};\npub use note_name::NoteName;\npub use error::TheoryError as Err2;\n";
        let re = reexport_map(root, &modules);
        assert_eq!(re.get("Chord").map(String::as_str), Some("chord"));
        assert_eq!(re.get("NoteName").map(String::as_str), Some("note_name"));
        assert_eq!(re.get("Err2").map(String::as_str), Some("error"));
        let file = "use crate::{NoteName, pitch::Letter};\n// use crate::chord::X; (comment)\nfn f() { crate::error::boom(); }\n";
        assert_eq!(
            module_refs(file, &modules, &re),
            set(&["note_name", "pitch", "error"])
        );
    }

    #[test]
    fn cycles_are_strongly_connected_components() {
        let mut g = ModuleGraph::new();
        g.insert("a".into(), set(&["b"]));
        g.insert("b".into(), set(&["c"]));
        g.insert("c".into(), set(&["a"]));
        g.insert("d".into(), set(&["a"]));
        g.insert("e".into(), set(&[]));
        assert_eq!(cycles(&g), vec![vec!["a", "b", "c"]]);
        g.get_mut("c").unwrap().clear();
        assert!(cycles(&g).is_empty());
    }

    #[test]
    fn real_workspace_graphs_build() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let g = crate_graph(&root.join("crates/mc-guitar")).unwrap();
        assert!(g["search"].contains("constraints") || g["search"].contains("voicing"));
    }
}
