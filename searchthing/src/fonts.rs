use egui_inspect::{
    egui::{FontData, FontDefinitions, FontFamily},
    logging::log::info,
};
use std::sync::Arc;
use walkdir::WalkDir;

fn add_font_data<'a>(font_defs: &mut FontDefinitions, font_name: &'a str) -> Option<&'a str> {
    let font_path = WalkDir::new("/usr/share/fonts/")
        .into_iter()
        .filter_map(|der| der.ok())
        .find(|de| {
            let fna = de.file_name().to_string_lossy();
            fna.contains(format!("{font_name}.ttf").as_str())
                || fna.contains(format!("{font_name}-Regular.ttf").as_str())
        })
        .map(|de| de.path().to_owned())?;
    info!("Loading {font_path:?}");
    let font_data_ = std::fs::read(font_path).ok()?;
    let font_data = FontData::from_owned(font_data_);
    font_defs
        .font_data
        .insert(font_name.into(), Arc::new(font_data));
    Some(font_name)
}

pub fn custom_font_def(main: Option<&str>, symbols: Option<&str>) -> FontDefinitions {
    let mut font_defs = FontDefinitions::default();
    font_defs.families.clear();
    let my_font = match main.map(|mf| add_font_data(&mut font_defs, mf)) {
        Some(Some(name)) => name,
        _ => "Ubuntu-Light",
    };
    let extra_symbols = symbols.map(|sf| add_font_data(&mut font_defs, sf));
    let mut default_fonts = vec![
        my_font.to_string(),
        "NotoEmoji-Regular".to_string(),
        "emoji-icon-font".to_string(),
    ];
    if let Some(Some(es)) = extra_symbols {
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
