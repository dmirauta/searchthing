use clap::Parser;
use dmenu::DmenuModule;
use plugin::PluginModule;
use std::{cell::RefCell, path::PathBuf, process::exit, thread::sleep, time::Duration};
use symbols::SymbolsModule;

use applications::ApplicationsModule;
use egui_inspect::{
    eframe::{CreationContext, NativeOptions, WindowBuilderHook},
    egui::{self, Color32, Key, RichText, Stroke, Vec2},
    logging::{log::warn, setup_mixed_logger, FileLogOption},
    search_select::non_contiguous_highlight,
    EframeMain, EguiInspect, FrameStyle, DEFAULT_FRAME_STYLE,
};
use searchthing_interface::{FuzzySearch, SearchMethod};
use ui::{IconPathCache, WrappedSearcher};

mod icon_search;
mod ui;

/// A program that displays a search bar, a set of options that are filtered by user input,
/// and acts on selection. The options provided, filtering and action perfomed depends on active modules,
/// which can be provided through external plugins.
#[derive(Parser)]
struct SearchThingArgs {
    /// Path to a shared object exporting SearchModule functions, can supply this argument many
    /// times to load multiple plugins.
    #[arg(short, long, value_parser)]
    plugin: Vec<PathBuf>,
    /// Stay open after a selection has been made.
    #[arg(long)]
    stay_open: bool,
    /// Enable the dmenu selection mode, can be followed by an optional prompt,
    /// e.g. -d "Select from the following".
    /// Options are specified by lines in stdin. The selected option is printed on stdout.
    #[arg(short, long)]
    dmenu: Option<Option<String>>,
    /// Unicode symbol picker mode.
    #[arg(short, long)]
    symbols: bool,
    #[arg(short, long)]
    no_builtin_modules: bool,
    /// Do an initial search with this text.
    #[arg(short, long)]
    init_search: Option<String>,
}

thread_local! {
    pub static STAY_OPEN: RefCell<bool> = Default::default();
}

#[derive(EframeMain)]
#[eframe_main(init = "SearchThing::new(_cc)", options = "set_opts()")]
struct SearchThing {
    search_input: String,
    last_queery: String,
    searchers: Vec<WrappedSearcher>,
    icon_path_cache: IconPathCache,
    #[allow(dead_code)]
    max_shown_per_searcher: u32,
    keyboard_idx: usize,
}

impl Default for SearchThing {
    fn default() -> Self {
        let mut args = SearchThingArgs::parse();
        STAY_OPEN.with_borrow_mut(|b| *b = args.stay_open);
        let max_shown_per_searcher = 10;
        let mut searchers = vec![];
        if let Some(prompt) = args.dmenu {
            searchers.push(WrappedSearcher::new(
                DmenuModule::new(prompt),
                max_shown_per_searcher,
            ));
            if args.init_search.is_none() {
                args.init_search = Some(String::new());
            }
        } else if args.symbols {
            searchers.push(WrappedSearcher::new(
                SymbolsModule::default(),
                max_shown_per_searcher,
            ));
        } else if !args.no_builtin_modules {
            searchers.push(WrappedSearcher::new(
                ApplicationsModule::default(),
                max_shown_per_searcher,
            ));
        }
        for path in args.plugin {
            let res = unsafe { PluginModule::new(&path) };
            match res {
                Ok(plug) => searchers.push(WrappedSearcher::new(plug, max_shown_per_searcher)),
                Err(e) => warn!("Failed to load library {path:?}: {e}"),
            }
        }
        let search_input = match args.init_search {
            Some(si) => {
                for searcher in &mut searchers {
                    searcher.queery(&si);
                }
                // HACK: give the user a moment to release the enter key if calling from the command line,
                // otherwise a selection is registered immediately
                sleep(Duration::from_millis(50));
                si
            }
            None => Default::default(),
        };
        Self {
            last_queery: search_input.clone(),
            search_input,
            searchers,
            icon_path_cache: Default::default(),
            max_shown_per_searcher,
            keyboard_idx: 0,
        }
    }
}

impl SearchThing {
    fn new(cc: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self::default()
    }
}

static HIGHLIGHT_FRAME: FrameStyle = FrameStyle {
    stroke: Stroke {
        width: 0.9,
        color: egui::Color32::GREEN,
    },
    ..DEFAULT_FRAME_STYLE
};

impl SearchThing {
    fn match_counts(&self) -> Vec<usize> {
        self.searchers
            .iter()
            .map(|s| s.cached_matches().len())
            .collect()
    }
}

fn is_highlighted_match(match_counts: &[usize], i: usize, j: usize, k: usize) -> bool {
    let count: usize = match_counts.iter().cloned().take(j).sum();
    count + i == k
}

fn kbd_idx(match_counts: &[usize], i: usize, j: usize) -> usize {
    let count: usize = match_counts.iter().cloned().take(j).sum();
    count + i
}

impl EguiInspect for SearchThing {
    fn inspect_mut(&mut self, _label: &str, ui: &mut egui::Ui) {
        let resp =
            ui.add(egui::TextEdit::singleline(&mut self.search_input).desired_width(f32::INFINITY));
        resp.request_focus();
        if resp.changed() && !self.search_input.is_empty() {
            for searcher in &mut self.searchers {
                searcher.queery(&self.search_input);
            }
            self.last_queery = self.search_input.clone();
        };

        let match_counts = self.match_counts();
        let total_matches = match_counts.iter().sum::<usize>();

        let mut mouse_activated = false;
        let mut mouse_moved = false;
        let mut kbd_activated = false;
        let mut kbd_moved = false;
        let mut scrolling = false;
        let mut requested_exit = false;
        ui.input(|i| {
            mouse_activated = i.pointer.button_clicked(egui::PointerButton::Primary);
            mouse_moved = i.pointer.time_since_last_movement() < 0.01; // TODO: a less arbitrary
                                                                       // threshhold?
            scrolling = i.pointer.middle_down();
            kbd_activated = i.key_released(Key::Enter);
            requested_exit = i.key_released(Key::Escape);
            if i.key_released(Key::ArrowUp) && self.keyboard_idx > 0 {
                self.keyboard_idx -= 1;
                kbd_moved = true;
            } else if i.key_released(Key::ArrowDown) && self.keyboard_idx < total_matches - 1
            // TODO: support holding after delay, but do not trigger more than once per frame...
            {
                self.keyboard_idx += 1;
                kbd_moved = true;
            }
        });
        if requested_exit {
            exit(0);
        }

        let max_height = ui.available_height() / (self.searchers.len() as f32);
        for (j, searcher) in self.searchers.iter_mut().enumerate() {
            searcher.inspect_with_match_render(
                ui,
                |ui, icon_name, match_name, desc, i| {
                    let is_highlighted =
                        is_highlighted_match(&match_counts, i, j, self.keyboard_idx);
                    let fs = match is_highlighted {
                        true => &HIGHLIGHT_FRAME,
                        false => &DEFAULT_FRAME_STYLE,
                    };

                    let resp = fs
                        .to_frame()
                        .show(ui, |ui| {
                            ui.separator(); // horizontal line expands frame to fill outer
                            ui.horizontal(|ui| {
                                self.icon_path_cache.get(&icon_name.into()).inspect("", ui);
                                ui.vertical(|ui| {
                                    let name_mtch = FuzzySearch::match_idxs(
                                        &match_name.to_lowercase(),
                                        &self.last_queery,
                                    );
                                    let desc_mtch = FuzzySearch::match_idxs(
                                        &desc.to_lowercase(),
                                        &self.last_queery,
                                    );
                                    let mtype = match (name_mtch, desc_mtch) {
                                        (Some((ns, nidxs)), Some((ds, didxs))) => match ns > ds {
                                            true => Some((true, nidxs)),
                                            false => Some((false, didxs)),
                                        },
                                        (Some((_, idxs)), None) => Some((true, idxs)),
                                        (None, Some((_, idxs))) => Some((false, idxs)),
                                        (None, None) => None,
                                    };
                                    match mtype {
                                        Some((is_name_mtch, idxs)) => {
                                            if is_name_mtch {
                                                ui.label(non_contiguous_highlight(
                                                    match_name,
                                                    &idxs,
                                                    Color32::GREEN,
                                                    Color32::WHITE,
                                                ));
                                                ui.label(desc);
                                            } else {
                                                ui.label(
                                                    RichText::new(match_name).color(Color32::WHITE),
                                                );
                                                ui.label(non_contiguous_highlight(
                                                    desc,
                                                    &idxs,
                                                    Color32::GREEN,
                                                    Color32::GRAY,
                                                ));
                                            }
                                        }
                                        None => {
                                            ui.label(
                                                RichText::new(match_name).color(Color32::WHITE),
                                            );
                                            ui.label(desc);
                                        }
                                    }
                                });
                            });
                            ui.separator();
                        })
                        .response;
                    let mouse_highlighted = resp.contains_pointer();

                    if mouse_highlighted {
                        if mouse_moved {
                            self.keyboard_idx = kbd_idx(&match_counts, i, j);
                        }
                        mouse_activated
                    } else if is_highlighted {
                        if kbd_moved {
                            resp.scroll_to_me(None);
                        }
                        kbd_activated
                    } else {
                        false
                    }
                },
                max_height,
            );
        }

        self.icon_path_cache.batch_find();
    }
}

fn set_opts() -> NativeOptions {
    setup_mixed_logger(FileLogOption::DefaultTempDir {
        log_name: "searchthing".into(),
    });
    let window_builder: Option<WindowBuilderHook> = Some(Box::new(|mut vb| {
        // NOTE: sadly does not currently seem to work...
        vb.window_level = Some(egui::WindowLevel::AlwaysOnTop);
        vb.window_type = Some(egui::X11WindowType::Dialog);
        vb.min_inner_size = Some(Vec2::new(800.0, 400.0));
        vb.max_inner_size = Some(Vec2::new(800.0, 400.0));
        vb
    }));
    NativeOptions {
        window_builder,
        centered: true,
        ..Default::default()
    }
}
