use egui_inspect::{
    egui::{
        vec2, Color32, ColorImage, FontData, FontDefinitions, FontFamily, Image, RichText, Sense,
        TextureHandle, Ui, Vec2,
    },
    logging::log::info,
};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, sync::Arc};
use swash::{
    scale::{ScaleContext, StrikeWith},
    FontRef,
};
use walkdir::WalkDir;

fn get_font_path(font_name: &str) -> Option<PathBuf> {
    WalkDir::new("/usr/share/fonts/")
        .into_iter()
        .filter_map(|der| der.ok())
        .find(|de| {
            let fna = de.file_name().to_string_lossy();
            fna.contains(format!("{font_name}.ttf").as_str())
                || fna.contains(format!("{font_name}-Regular.ttf").as_str())
        })
        .map(|de| de.path().to_owned())
}

thread_local! {
    /// for use both directly in egui and via swash (for single multicolor glyph previews)
    static STATICFONTS: RefCell<BTreeMap<String, &'static [u8]>> = Default::default();
}

/// stores font in STATICFONTS and registers it with egui
fn add_font_data(font_defs: &mut FontDefinitions, font_name: String) -> Option<String> {
    let font_path = get_font_path(font_name.as_str())?;
    info!("Loading {font_path:?}");
    let font_data_ = std::fs::read(font_path).ok()?;
    let sfd = font_data_.leak();
    STATICFONTS.with_borrow_mut(|sf| {
        sf.insert(font_name.clone(), sfd);
    });
    let font_data = FontData::from_static(sfd);
    font_defs
        .font_data
        .insert(font_name.clone(), Arc::new(font_data));
    Some(font_name)
}

pub fn custom_egui_font_def(main: Option<String>, symbols: Vec<String>) -> FontDefinitions {
    let mut font_defs = FontDefinitions::default();
    font_defs.families.clear();
    let my_font = match main.map(|mf| add_font_data(&mut font_defs, mf)) {
        Some(Some(name)) => name,
        _ => "Ubuntu-Light".to_string(),
    };
    let extra_symbols = symbols
        .into_iter()
        .filter_map(|sf| add_font_data(&mut font_defs, sf));
    let mut default_fonts = vec![
        my_font,
        "NotoEmoji-Regular".to_string(),
        "emoji-icon-font".to_string(),
    ];
    for es in extra_symbols {
        default_fonts.insert(1, es.to_string());
    }
    font_defs
        .families
        .insert(FontFamily::Proportional, default_fonts.clone());
    default_fonts.insert(0, "Hack".to_string());
    font_defs
        .families
        .insert(FontFamily::Monospace, default_fonts);
    font_defs
}

#[derive(Default)]
pub struct SymbolImageCache {
    cache: BTreeMap<char, Option<TextureHandle>>,
}

thread_local! {
    pub static SYMBOLCACHE: RefCell<SymbolImageCache> = Default::default();
}

impl SymbolImageCache {
    fn try_load_image(&mut self, ui: &mut Ui, font: FontRef, symbol: char) -> bool {
        let glyph_id = font.charmap().map(symbol);

        let mut scale_context = ScaleContext::default();
        let mut scaler = scale_context.builder(font).build();
        let handle = scaler
            .scale_color_bitmap(glyph_id, StrikeWith::LargestSize)
            .map(|image| {
                let cimage = ColorImage::from_rgba_unmultiplied(
                    [
                        image.placement.width as usize,
                        image.placement.height as usize,
                    ],
                    image.data.as_slice(),
                );
                ui.ctx().load_texture(symbol, cimage, Default::default())
            });
        self.cache.insert(symbol, handle.clone());
        handle.is_some()
    }
    pub fn inspect(&mut self, ui: &mut Ui, symbol: char, size: Vec2) {
        match self.cache.get(&symbol) {
            Some(Some(th)) => {
                ui.add(Image::new(th).fit_to_exact_size(size));
            }
            None => {
                // NOTE: allowing a 1 frame delay here, also all newly visible glyphs will load at
                // once here
                STATICFONTS.with_borrow(|sf| {
                    for data in sf.values() {
                        let font = swash::FontRef::from_index(data, 0).unwrap();
                        if self.try_load_image(ui, font, symbol) {
                            break;
                        }
                    }
                });
            }
            _ => {
                let rt = RichText::from(format!("{symbol}"))
                    .size(size.y)
                    .color(Color32::WHITE);
                let r = ui.label(rt).rect;
                let s = ui.spacing().item_spacing.x;
                let d = size.x - r.width() - s;
                if d > 0.0 {
                    ui.allocate_exact_size(vec2(d, 0.0), Sense::empty());
                }
            }
        }
    }
}
