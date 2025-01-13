use std::process::Command;

use searchthing_interface::{
    char_from_codepoint, FuzzySearch, SearchItemHandle, SearchMethod, SearchModule,
};

static CODEPOINTS: &str = include_str!("../codepoints");

struct LabeledCodepoints {
    primary_label: &'static str,
    secondary_label: &'static str,
    codepoint: &'static str,
}

pub struct SymbolsModule {
    name: String,
    icon: String,
    options: Vec<LabeledCodepoints>,
}

impl Default for SymbolsModule {
    fn default() -> Self {
        // TODO: make this a compile time calculated ArrayVec?
        let options = CODEPOINTS
            .lines()
            .map(|line| {
                let sp = line.split(';').collect::<Vec<_>>();
                LabeledCodepoints {
                    primary_label: sp[1],
                    secondary_label: sp[2],
                    codepoint: sp[0],
                }
            })
            .collect();
        Self {
            name: "Symbols".into(),
            icon: "emoji-symbols-symbolic".into(),
            options,
        }
    }
}

impl SearchModule for SymbolsModule {
    // TODO: fuzzy matching
    fn queery(
        &self,
        input: &str,
        max_returned: u32,
    ) -> Vec<searchthing_interface::SearchItemHandle> {
        let mut matches = vec![];
        for (idx, opt) in self.options.iter().enumerate() {
            if let Some((s, _)) = FuzzySearch::match_idxs(&opt.primary_label.to_lowercase(), input)
            {
                matches.push((s, SearchItemHandle(idx as i32)));
            } else if let Some((s, _)) =
                FuzzySearch::match_idxs(&opt.secondary_label.to_lowercase(), input)
            {
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
        let opt = self.options.get(item.0 as usize).unwrap();
        // NOTE: handles should be valid, as they should have been obtained through the queery fn
        searchthing_interface::MatchInfo {
            name: opt.primary_label,
            desc: opt.secondary_label,
            icon: opt.codepoint,
        }
    }

    fn handle_selection(&self, selection: SearchItemHandle) {
        let opt = self.options.get(selection.0 as usize).unwrap();
        if let Some(c) = char_from_codepoint(opt.codepoint) {
            Command::new("wl-copy").arg(c.to_string()).spawn().unwrap();
        }
    }
}
