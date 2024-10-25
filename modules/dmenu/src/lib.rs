use std::{io::stdin, process::exit};

use searchthing_interface::{substring_range, SearchItemHandle, SearchModule};

static EMPTY: &str = "";

pub struct DmenuModule {
    name: String,
    icon: String,
    options: Vec<String>,
}

impl Default for DmenuModule {
    fn default() -> Self {
        let options = stdin()
            .lines()
            .filter_map(|lr| match lr {
                Ok(l) => Some(l),
                Err(_) => None,
            })
            .collect();
        Self {
            name: "Drun".into(),
            icon: "system-search".into(),
            options,
        }
    }
}

impl SearchModule for DmenuModule {
    // TODO: fuzzy matching
    fn queery(
        &self,
        input: &str,
        max_returned: u32,
    ) -> Vec<searchthing_interface::SearchItemHandle> {
        let mut matches = vec![];
        for (idx, opt) in self.options.iter().enumerate() {
            if let Some(r) = substring_range(&opt.to_lowercase(), input) {
                matches.push((r.start, SearchItemHandle(idx as i32)));
            }
        }
        matches.sort_by_key(|i| i.0);
        matches
            .into_iter()
            .take(max_returned as usize)
            .map(|i| i.1)
            .collect()
    }

    fn info(&self) -> searchthing_interface::SearcherInfo {
        searchthing_interface::SearcherInfo {
            name: &self.name,
            icon: &self.icon,
        }
    }

    fn get_match_info(&self, item: SearchItemHandle) -> searchthing_interface::MatchInfo {
        let opt = self.options.get(item.0 as usize).unwrap();
        // NOTE: handles should be valid, as they should have been obtained through the queery fn
        searchthing_interface::MatchInfo {
            name: opt,
            desc: EMPTY,
            icon: EMPTY,
        }
    }

    fn handle_selection(&self, selection: SearchItemHandle) {
        let opt = self.options.get(selection.0 as usize).unwrap();
        println!("{}", opt);
        exit(0);
    }
}
