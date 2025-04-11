use std::{env, process::Command};

use find_desktop_entries::{get_desktop_entries, DesktopEntry};
use log::{error, info};
use searchthing_interface::{FuzzySearch, SearchItemHandle, SearchMethod, SearchModule};

mod find_desktop_entries;

struct WrappedDesktopEntry {
    entry: DesktopEntry,
    search_text: String,
}

impl From<DesktopEntry> for WrappedDesktopEntry {
    fn from(entry: DesktopEntry) -> Self {
        let mut search_text = format!("{}||{}", &entry.name, &entry.keywords.join("||"));
        if let Some(desc) = &entry.desc {
            search_text = format!("{search_text}||{desc}");
        }
        search_text = search_text.to_lowercase();
        Self { entry, search_text }
    }
}

static EMPTY: &str = "";

pub struct ApplicationsModule {
    name: String,
    icon: String,
    entries: Vec<WrappedDesktopEntry>,
}

impl Default for ApplicationsModule {
    fn default() -> Self {
        let entries = get_desktop_entries(true)
            .unwrap()
            .into_iter()
            .map(|de| de.into())
            .collect();
        Self {
            entries,
            name: "Applications".into(),
            icon: "application-x-executable".into(),
        }
    }
}

impl SearchModule for ApplicationsModule {
    fn queery(
        &self,
        input: &str,
        max_returned: u32,
    ) -> Vec<searchthing_interface::SearchItemHandle> {
        let mut matches = vec![];
        for (idx, wrapped) in self.entries.iter().enumerate() {
            if let Some((s, _)) = FuzzySearch::match_idxs(&wrapped.search_text, input) {
                matches.push((s, SearchItemHandle(idx as i32)));
            }
        }
        matches.sort_by_key(|i| i.0);
        matches
            .into_iter()
            .rev()
            .take(max_returned as usize)
            .map(|i| i.1)
            .collect()
    }

    fn mod_info(&self) -> searchthing_interface::SearcherInfo {
        searchthing_interface::SearcherInfo {
            name: &self.name,
            icon: &self.icon,
        }
    }

    fn match_info(&self, item: SearchItemHandle) -> searchthing_interface::MatchInfo {
        let w = self.entries.get(item.0 as usize).unwrap();
        // NOTE: handles should be valid, as they should have been obtained through the queery fn
        searchthing_interface::MatchInfo {
            name: &w.entry.name,
            desc: match &w.entry.desc {
                Some(desc) => desc.as_str(),
                None => EMPTY,
            },
            icon: &w.entry.icon,
        }
    }

    fn handle_selection(&self, selection: SearchItemHandle) {
        let w = self.entries.get(selection.0 as usize).unwrap();
        info!(
            "Selected {}, which has search text: {}",
            w.entry.name, w.search_text
        );
        if w.entry.term {
            let term = env::var("TERMINAL").or(env::var("TERM"));
            match term {
                Ok(term) => {
                    let mut cmd = Command::new(term);
                    cmd.arg("-e").arg(w.entry.exec.trim());
                    if let Err(why) = cmd.spawn() {
                        error!(
                            "Error: {why}, running desktop entry {:?} (with term, cmd: {cmd:?})",
                            &w.entry
                        );
                    }
                }
                Err(_) => {
                    error!("Expecting $TERMINAL or $TERM to be set for running terminal programs.")
                }
            }
        } else {
            let current_dir = if w.entry.path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                w.entry.path.as_ref().unwrap()
            } else {
                &env::current_dir().unwrap()
            };

            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(&w.entry.exec).current_dir(current_dir);
            if let Err(why) = cmd.spawn() {
                error!(
                    "Error: {why}, running desktop entry {:?} (with sh, cmd: {cmd:?})",
                    &w.entry
                );
            }
        }
    }
}
