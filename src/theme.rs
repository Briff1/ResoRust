



use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::sync::Arc;





pub const BG: Color32 = Color32::from_rgb(11, 9, 8);
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(15, 12, 10);
pub const PLAYER_BG: Color32 = Color32::from_rgb(13, 10, 9);

pub const SURFACE: Color32 = Color32::from_rgb(26, 21, 18);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(37, 29, 24);
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(47, 36, 29);

pub const ACCENT: Color32 = Color32::from_rgb(177, 121, 77);
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(219, 163, 111);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(104, 67, 41);

pub const TEXT: Color32 = Color32::from_rgb(241, 233, 224);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(165, 147, 133);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(110, 95, 84);

pub const LINE: Color32 = Color32::from_rgb(43, 34, 29);


pub fn alpha(color: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}





pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn font_semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semi".into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let faces: [(&str, &'static [u8]); 3] = [
        (
            "poppins",
            include_bytes!("../assets/fonts/Poppins-Regular.ttf"),
        ),
        (
            "poppins-medium",
            include_bytes!("../assets/fonts/Poppins-Medium.ttf"),
        ),
        (
            "poppins-semibold",
            include_bytes!("../assets/fonts/Poppins-SemiBold.ttf"),
        ),
    ];

    for (name, bytes) in faces {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    
    
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "poppins".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("poppins".to_owned());

    
    
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec!["poppins-medium".to_owned(), "poppins".to_owned()]
            .into_iter()
            .chain(fallbacks.iter().cloned())
            .collect(),
    );
    fonts.families.insert(
        FontFamily::Name("semi".into()),
        vec!["poppins-semibold".to_owned(), "poppins".to_owned()]
            .into_iter()
            .chain(fallbacks.iter().cloned())
            .collect(),
    );

    ctx.set_fonts(fonts);
}

fn install_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = Color32::from_rgb(17, 13, 11);
    v.faint_bg_color = SURFACE;
    v.hyperlink_color = ACCENT_BRIGHT;
    v.selection.bg_fill = alpha(ACCENT, 0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(150),
    };
    v.popup_shadow = v.window_shadow;
    v.striped = false;

    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.weak_bg_fill = SURFACE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    v.widgets.inactive.bg_fill = SURFACE;
    v.widgets.inactive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    v.widgets.inactive.corner_radius = CornerRadius::same(9);

    v.widgets.hovered.bg_fill = SURFACE_HOVER;
    v.widgets.hovered.weak_bg_fill = SURFACE_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, SURFACE_ACTIVE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
    v.widgets.hovered.corner_radius = CornerRadius::same(9);

    v.widgets.active.bg_fill = SURFACE_ACTIVE;
    v.widgets.active.weak_bg_fill = SURFACE_ACTIVE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_DARK);
    v.widgets.active.fg_stroke = Stroke::new(1.5, ACCENT_BRIGHT);
    v.widgets.active.corner_radius = CornerRadius::same(9);

    v.widgets.open.bg_fill = SURFACE_HOVER;
    v.widgets.open.weak_bg_fill = SURFACE_HOVER;
    v.widgets.open.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.open.corner_radius = CornerRadius::same(9);

    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.slider_width = 110.0;
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.bar_inner_margin = 4.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.scroll.floating = true;
    style.animation_time = 0.12;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Name("semi".into())),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.5, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );

    ctx.set_style_of(egui::Theme::Dark, style);
}


pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}






pub fn app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    let c = (size as f32 - 1.0) / 2.0;
    let corner = 13.0f32;

    for y in 0..size {
        for x in 0..size {
            let px = x as f32;
            let py = y as f32;

            
            let cx = if px < corner {
                corner
            } else if px > (size as f32 - 1.0 - corner) {
                size as f32 - 1.0 - corner
            } else {
                px
            };
            let cy = if py < corner {
                corner
            } else if py > (size as f32 - 1.0 - corner) {
                size as f32 - 1.0 - corner
            } else {
                py
            };
            let outside = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() > corner;

            let dx = px - c;
            let dy = py - c;
            let d = (dx * dx + dy * dy).sqrt();

            let color = if outside {
                Color32::TRANSPARENT
            } else if d < 7.0 {
                BG
            } else if d < 12.0 {
                ACCENT_DARK
            } else if d < 27.0 {
                if dy < -3.0 && dx < 6.0 {
                    ACCENT_BRIGHT
                } else {
                    ACCENT
                }
            } else {
                Color32::from_rgb(20, 15, 13)
            };

            let i = (y * size + x) * 4;
            rgba[i] = color.r();
            rgba[i + 1] = color.g();
            rgba[i + 2] = color.b();
            rgba[i + 3] = color.a();
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
