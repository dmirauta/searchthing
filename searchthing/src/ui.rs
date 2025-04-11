use crate::{fonts::SYMBOLCACHE, icon_search::find_icons, ICONSIZE, STAY_OPEN};
use egui_inspect::{
    egui::{self, ScrollArea, Vec2},
    EguiInspect, DEFAULT_FRAME_STYLE,
};
use searchthing_interface::{
    char_from_codepoint, MatchInfo, SearchItemHandle, SearchModule, SearcherInfo,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct AppIconPathCache {
    /// name to path map
    store: HashMap<String, Option<String>>,
    // TODO: should just use a built in priority queue?
    /// icons to search and find next, only a few per frame, newest added first
    requests: BTreeMap<u64, String>,
    req_no: u64,
}

// NOTE: using a hacky async mechanism, tied to egui eventloop for simplicity
impl AppIconPathCache {
    pub fn get(&mut self, name: &String) -> Icon {
        if name.is_empty() {
            return Icon::None;
        }
        if name[..2].eq("U+") {
            return char_from_codepoint(name)
                .map(|c| Icon::Unicode { c })
                .unwrap_or(Icon::NotFound);
        }
        match self.store.get(name) {
            Some(Some(path)) => Icon::Found { path },
            Some(None) => Icon::NotFound,
            None => {
                let kv = self
                    .requests
                    .iter()
                    .find(|(_, v)| **v == *name)
                    .map(|(k, v)| (*k, v.clone()));
                if let Some((old_req_no, _)) = kv {
                    self.requests.remove(&old_req_no);
                }
                self.requests.insert(self.req_no, name.clone());
                self.req_no += 1;
                Icon::Searching
            }
        }
    }
    /// resolve a number of the newest requests
    pub fn batch_find(&mut self, max_per_tick: usize) -> usize {
        let mut batch = vec![];
        for _ in 0..max_per_tick {
            if let Some((_, item)) = self.requests.pop_last() {
                batch.push(item);
            }
        }
        let res = find_icons(batch.clone());
        let n = batch.len();
        for name in batch {
            match res.get(&name) {
                Some(de) => self
                    .store
                    .insert(name, Some(de.path().to_string_lossy().into())),
                None => self.store.insert(name, None),
            };
        }
        n
    }
}

#[derive(Debug)]
pub enum Icon<'a> {
    Searching,
    Found { path: &'a String },
    NotFound,
    Unicode { c: char },
    None,
}

impl EguiInspect for Icon<'_> {
    fn inspect(&self, _label: &str, ui: &mut egui_inspect::egui::Ui) {
        let is = ICONSIZE.with_borrow(|is| *is);
        let unicode = match self {
            Icon::Searching { .. } => Some('⮔'),
            Icon::NotFound => Some('⚠'),
            Icon::Found { path } => {
                ui.add(
                    egui::Image::new(format!("file://{path}"))
                        .fit_to_exact_size(Vec2 { x: is, y: is }),
                );
                None
            }
            Icon::None => None,
            Icon::Unicode { c } => Some(*c),
        };
        if let Some(symbol) = unicode {
            SYMBOLCACHE.with_borrow_mut(|sc| {
                sc.inspect(ui, symbol, Vec2 { x: is, y: is });
            });
        }
    }
}

pub struct WrappedSearcher {
    searcher: Box<dyn SearchModule>,
    name: String,
    #[allow(dead_code)]
    icon: String,
    cached_matches: Vec<SearchItemHandle>,
    max_shown: u32,
}

impl WrappedSearcher {
    pub fn new(searcher: impl SearchModule + 'static, max_shown: u32) -> Self {
        let SearcherInfo { name, icon } = searcher.mod_info();
        Self {
            name: name.into(),
            icon: icon.into(),
            searcher: Box::new(searcher),
            cached_matches: Default::default(),
            max_shown,
        }
    }
    pub fn queery(&mut self, input: &str) {
        self.cached_matches = self.searcher.queery(input, self.max_shown);
    }
    pub fn cached_matches(&self) -> &Vec<SearchItemHandle> {
        &self.cached_matches
    }
    pub fn inspect_with_match_render(
        &mut self,
        ui: &mut egui::Ui,
        mut render_match: impl FnMut(&mut egui::Ui, &str, &str, &str, usize) -> bool,
        max_height: f32,
    ) {
        DEFAULT_FRAME_STYLE.to_frame().show(ui, |ui| {
            ui.strong(&self.name);

            DEFAULT_FRAME_STYLE.to_frame().show(ui, |ui| {
                // draw match details
                if !self.cached_matches.is_empty() {
                    ScrollArea::vertical()
                        .id_salt(&self.name)
                        .max_height(max_height)
                        .show(ui, |ui| {
                            for (i, handle) in self.cached_matches.iter().enumerate() {
                                let MatchInfo { name, desc, icon } =
                                    self.searcher.match_info(*handle);
                                if render_match(ui, icon, name, desc, i) {
                                    self.searcher.handle_selection(*handle);

                                    if !STAY_OPEN.with_borrow_mut(|b| *b) {
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                }
                            }
                        });
                } else {
                    ui.label("no matches");
                }
            });
        });
    }
}
