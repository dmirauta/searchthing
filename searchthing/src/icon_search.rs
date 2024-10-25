use std::{collections::HashMap, env, path::PathBuf};

use walkdir::{DirEntry, WalkDir};

fn icons_iterator() -> impl Iterator<Item = DirEntry> {
    // TODO: gracefully handle missing paths?
    // TODO: priortise higher detail subfolders?
    let home_path = PathBuf::from(env::var("HOME").unwrap());
    let local_icons = home_path.join(".local/share/icons");
    WalkDir::new("/usr/share/icons")
        .follow_links(false)
        .into_iter()
        .chain(WalkDir::new(local_icons).follow_links(false))
        .chain(WalkDir::new("/var/lib/flatpak/exports/share/icons").follow_links(false))
        .chain(WalkDir::new("/usr/share/pixmaps").follow_links(false))
        .filter_map(|der| der.ok())
}

#[allow(dead_code)]
pub fn find_icon(name: &str) -> Option<DirEntry> {
    icons_iterator().find(|de| de.file_name().to_string_lossy().contains(name))
}

#[test]
fn find_icon_test() {
    dbg!(find_icon("firefox"));
    dbg!(find_icon("neovim"));
}

/// retrieve more than one icons in a single directory walk
pub fn find_icons(mut names: Vec<String>) -> HashMap<String, DirEntry> {
    let mut res = HashMap::new();
    if !names.is_empty() {
        for de in icons_iterator() {
            let file_name = de.file_name().to_string_lossy();
            let _match = names
                .iter()
                .enumerate()
                .find(|(_, name)| file_name.contains(name.as_str()));
            if let Some((idx, name)) = _match {
                res.insert(name.clone(), de);
                names.remove(idx);
            }
        }
    }
    res
}
