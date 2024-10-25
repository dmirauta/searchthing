use clap::Parser;
use plugin::PluginModule;
use std::{cell::RefCell, path::PathBuf, process::exit};

use applications::ApplicationsModule;
use egui_inspect::{
    eframe::{CreationContext, NativeOptions, WindowBuilderHook},
    egui::{self, Color32, Key, RichText, Stroke, Vec2},
    logging::{log::warn, setup_mixed_logger, FileLogOption},
    utils::concat_rich_text,
    EframeMain, EguiInspect, FrameStyle, DEFAULT_FRAME_STYLE,
};
use searchthing_interface::substring_range;
use ui::{IconPathCache, WrappedSearcher};

mod icon_search;
mod ui;

/// A program that displays a search bar, a set of options that are filtered by it, and acts on
/// selection. The options, filtering on search and action depends on active modules, which can be
/// provided through external plugins.
#[derive(Parser)]
struct SearchThingArgs {
    /// Path to a shared object exporting SearchModule functions, can supply argument many times.
    #[arg(short, long, value_parser)]
    plugin: Vec<PathBuf>,
    /// Stay open after selection.
    #[arg(short, long)]
    stay_open: bool,
}

thread_local! {
    pub static STAY_OPEN: RefCell<bool> = Default::default();
}

#[derive(EframeMain)]
#[eframe_main(init = "SearchThing::new(_cc)", options = "set_opts()")]
struct SearchThing {
    search_input: String,
    last_queery: String,
    // TODO: multiple searchers
    searchers: Vec<WrappedSearcher>,
    icon_path_cache: IconPathCache,
    #[allow(dead_code)]
    max_shown_per_searcher: u32,
    keyboard_idx: usize,
}

impl Default for SearchThing {
    fn default() -> Self {
        let args = SearchThingArgs::parse();
        STAY_OPEN.with_borrow_mut(|b| *b = args.stay_open);
        let max_shown_per_searcher = 10;
        let mut searchers = vec![WrappedSearcher::new(
            ApplicationsModule::default(),
            max_shown_per_searcher,
        )];
        for path in args.plugin {
            let res = unsafe { PluginModule::new(&path) };
            match res {
                Ok(plug) => searchers.push(WrappedSearcher::new(plug, max_shown_per_searcher)),
                Err(e) => warn!("Failed to load library {path:?}: {e}"),
            }
        }
        Self {
            search_input: Default::default(),
            last_queery: Default::default(),
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
        let resp = ui.text_edit_singleline(&mut self.search_input);
        resp.request_focus();
        if resp.changed() && !self.search_input.is_empty() {
            for searcher in &mut self.searchers {
                searcher.queery(&self.search_input);
            }
            self.last_queery = self.search_input.clone();
        };

        let match_counts = self.match_counts();
        let total_matches = match_counts.iter().sum();

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
            } else if i.key_released(Key::ArrowDown) && self.keyboard_idx < total_matches
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
                                    match substring_range(
                                        &match_name.to_lowercase(),
                                        &self.last_queery,
                                    ) {
                                        Some(mr) => {
                                            let prefix = &match_name[..mr.start];
                                            let mid = &match_name[mr.start..mr.end];
                                            let suffix = &match_name[mr.end..];
                                            ui.label(concat_rich_text(vec![
                                                RichText::new(prefix).color(Color32::WHITE),
                                                RichText::new(mid).color(Color32::GREEN),
                                                RichText::new(suffix).color(Color32::WHITE),
                                            ]))
                                        }
                                        None => ui.label(match_name),
                                    };
                                    ui.label(desc);
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
