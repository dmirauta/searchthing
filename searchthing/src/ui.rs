use std::collections::{BTreeMap, HashMap};

use egui_inspect::{
    egui::{self, Color32, RichText, ScrollArea, Vec2},
    EguiInspect, DEFAULT_FRAME_STYLE,
};
use searchthing_interface::{MatchInfo, SearchItemHandle, SearchModule, SearcherInfo};

use crate::{icon_search::find_icons, STAY_OPEN};

#[derive(Default)]
pub struct IconPathCache {
    /// name to path map
    store: HashMap<String, Option<String>>,
    // TODO: distinguish between IconName(String) and IconPath(String)
    /// icons to search and find next, only a few per frame, newest added first
    requests: BTreeMap<u64, String>,
    req_no: u64,
}

impl IconPathCache {
    pub fn get(&mut self, name: &String) -> Icon {
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
    // gradual search left tied to gui event loop for simplicity
    pub fn batch_find(&mut self) -> usize {
        let mut batch = vec![];
        let max_per_tick = 5;
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
}

impl<'a> EguiInspect for Icon<'a> {
    fn inspect(&self, _label: &str, ui: &mut egui_inspect::egui::Ui) {
        match self {
            Icon::Searching { .. } => {
                ui.label(RichText::new("⮔").color(Color32::BLUE))
                    .on_hover_text("searching for icon");
            }
            Icon::NotFound => {
                ui.label(RichText::new("⚠").color(Color32::RED))
                    .on_hover_text("icon not found");
            }
            Icon::Found { path } => {
                ui.add(
                    egui::Image::new(format!("file://{path}"))
                        .fit_to_exact_size(Vec2::new(48.0, 48.0)),
                );
            }
        };
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
        let SearcherInfo { name, icon } = searcher.info();
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
            ui.columns(2, |cols| {
                DEFAULT_FRAME_STYLE.to_frame().show(&mut cols[0], |ui| {
                    // draw app details
                    ui.label(&self.name);
                });

                DEFAULT_FRAME_STYLE.to_frame().show(&mut cols[1], |ui| {
                    // draw match details
                    if !self.cached_matches.is_empty() {
                        ScrollArea::vertical()
                            .id_salt(&self.name)
                            .max_height(max_height)
                            .show(ui, |ui| {
                                for (i, handle) in self.cached_matches.iter().enumerate() {
                                    let MatchInfo { name, desc, icon } =
                                        self.searcher.get_match_info(*handle);
                                    if render_match(ui, icon, name, desc, i) {
                                        self.searcher.handle_selection(*handle);
                                        if !STAY_OPEN.with_borrow_mut(|b| *b) {
                                            std::process::exit(0);
                                        }
                                    }
                                }
                            });
                    } else {
                        ui.label("no matches");
                    }
                });
            });
        });
    }
}
