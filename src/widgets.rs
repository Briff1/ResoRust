

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};

use crate::theme::{self, *};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Next,
    Prev,
    Shuffle,
    Repeat,
    Volume,
    Muted,
    Heart,
    HeartFilled,
    Search,
    Folder,
    Refresh,
    Music,
    Disc,
    Sliders,
    Close,
    Playlist,
    Download,
}

#[derive(Default)]
pub struct SeekResult {
    
    pub preview: Option<f32>,
    
    pub commit: Option<f32>,
}





fn arrow_head(painter: &egui::Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let dir = dir.normalized();
    let normal = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * size;
    painter.add(Shape::convex_polygon(
        vec![
            tip,
            base + normal * size * 0.55,
            base - normal * size * 0.55,
        ],
        color,
        Stroke::NONE,
    ));
}

pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let w = rect.width();
    let h = rect.height();
    let p = |x: f32, y: f32| Pos2::new(rect.min.x + x * w, rect.min.y + y * h);
    let stroke = Stroke::new(width, color);

    match icon {
        Icon::Play => {
            painter.add(Shape::convex_polygon(
                vec![p(0.30, 0.16), p(0.82, 0.50), p(0.30, 0.84)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            painter.rect_filled(
                Rect::from_min_max(p(0.28, 0.18), p(0.43, 0.82)),
                CornerRadius::same(1),
                color,
            );
            painter.rect_filled(
                Rect::from_min_max(p(0.57, 0.18), p(0.72, 0.82)),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Next => {
            painter.add(Shape::convex_polygon(
                vec![p(0.16, 0.18), p(0.70, 0.50), p(0.16, 0.82)],
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_min_max(p(0.73, 0.18), p(0.84, 0.82)),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Prev => {
            painter.add(Shape::convex_polygon(
                vec![p(0.84, 0.18), p(0.30, 0.50), p(0.84, 0.82)],
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_min_max(p(0.16, 0.18), p(0.27, 0.82)),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Shuffle => {
            
            painter.add(Shape::line(
                vec![p(0.10, 0.24), p(0.30, 0.24), p(0.72, 0.76), p(0.82, 0.76)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![p(0.10, 0.76), p(0.30, 0.76), p(0.72, 0.24), p(0.82, 0.24)],
                stroke,
            ));
            arrow_head(painter, p(0.94, 0.76), Vec2::new(1.0, 0.0), w * 0.20, color);
            arrow_head(painter, p(0.94, 0.24), Vec2::new(1.0, 0.0), w * 0.20, color);
        }
        Icon::Repeat => {
            
            painter.add(Shape::line(
                vec![p(0.20, 0.26), p(0.80, 0.26), p(0.80, 0.42)],
                stroke,
            ));
            arrow_head(painter, p(0.80, 0.58), Vec2::new(0.0, 1.0), w * 0.20, color);
            painter.add(Shape::line(
                vec![p(0.80, 0.74), p(0.20, 0.74), p(0.20, 0.58)],
                stroke,
            ));
            arrow_head(painter, p(0.20, 0.42), Vec2::new(0.0, -1.0), w * 0.20, color);
        }
        Icon::Volume | Icon::Muted => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(0.10, 0.38),
                    p(0.26, 0.38),
                    p(0.44, 0.20),
                    p(0.44, 0.80),
                    p(0.26, 0.62),
                    p(0.10, 0.62),
                ],
                color,
                Stroke::NONE,
            ));
            if icon == Icon::Muted {
                painter.line_segment([p(0.58, 0.40), p(0.82, 0.62)], stroke);
                painter.line_segment([p(0.58, 0.62), p(0.82, 0.40)], stroke);
            } else {
                for (radius, span) in [(0.16, 0.62), (0.28, 0.52)] {
                    let mut points = Vec::with_capacity(12);
                    for i in 0..=10 {
                        let t = i as f32 / 10.0;
                        let angle = -span + t * (span * 2.0);
                        points.push(Pos2::new(
                            p(0.48, 0.5).x + (angle.cos() * radius * w),
                            p(0.48, 0.5).y + (angle.sin() * radius * h),
                        ));
                    }
                    painter.add(Shape::line(points, stroke));
                }
            }
        }
        Icon::Heart | Icon::HeartFilled => {
            if icon == Icon::HeartFilled {
                let r = w.min(h) * 0.22;
                let center = rect.center();
                let left = Pos2::new(center.x - r * 0.82, center.y - r * 0.35);
                let right = Pos2::new(center.x + r * 0.82, center.y - r * 0.35);
                painter.circle_filled(left, r, color);
                painter.circle_filled(right, r, color);
                painter.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(center.x - r * 1.78, center.y - r * 0.15),
                        Pos2::new(center.x + r * 1.78, center.y - r * 0.15),
                        Pos2::new(center.x, center.y + r * 1.95),
                    ],
                    color,
                    Stroke::NONE,
                ));
            } else {
                
                const STEPS: usize = 96;
                let mut points = Vec::with_capacity(STEPS);
                for i in 0..STEPS {
                    let t = i as f32 / STEPS as f32 * std::f32::consts::TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos()
                        - 5.0 * (2.0 * t).cos()
                        - 2.0 * (3.0 * t).cos()
                        - (4.0 * t).cos();
                    points.push(Pos2::new(x, -y));
                }

                
                for _ in 0..2 {
                    let previous = points.clone();
                    for i in 0..STEPS {
                        let a = previous[(i + STEPS - 1) % STEPS];
                        let b = previous[i];
                        let c = previous[(i + 1) % STEPS];
                        points[i] = Pos2::new(
                            (a.x + 2.0 * b.x + c.x) / 4.0,
                            (a.y + 2.0 * b.y + c.y) / 4.0,
                        );
                    }
                }

                let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
                let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
                for point in &points {
                    min.x = min.x.min(point.x);
                    min.y = min.y.min(point.y);
                    max.x = max.x.max(point.x);
                    max.y = max.y.max(point.y);
                }

                let span = max - min;
                let scale = ((w - 2.0) / span.x).min((h - 2.0) / span.y);
                let offset = Pos2::new(
                    rect.min.x + (w - span.x * scale) * 0.5,
                    rect.min.y + (h - span.y * scale) * 0.5,
                );
                for point in &mut points {
                    *point = Pos2::new(
                        offset.x + (point.x - min.x) * scale,
                        offset.y + (point.y - min.y) * scale,
                    );
                }

                painter.add(Shape::closed_line(points, stroke));
            }
        }
        Icon::Search => {
            painter.circle_stroke(p(0.42, 0.42), w * 0.24, stroke);
            painter.line_segment([p(0.60, 0.60), p(0.84, 0.84)], stroke);
        }
        Icon::Folder => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.10, 0.34), p(0.90, 0.80)),
                CornerRadius::same(3),
                stroke,
                StrokeKind::Inside,
            );
            painter.add(Shape::line(
                vec![p(0.10, 0.34), p(0.10, 0.23), p(0.46, 0.23), p(0.54, 0.34)],
                stroke,
            ));
        }
        Icon::Refresh => {
            let center = p(0.5, 0.5);
            let radius = 0.30;
            let start = 0.15 * std::f32::consts::PI;
            let end = 1.75 * std::f32::consts::PI;
            let mut points = Vec::with_capacity(25);
            for i in 0..=24 {
                let t = i as f32 / 24.0;
                let a = start + (end - start) * t;
                points.push(Pos2::new(
                    center.x + a.cos() * radius * w,
                    center.y + a.sin() * radius * h,
                ));
            }
            painter.add(Shape::line(points, stroke));

            
            let tip = Pos2::new(
                center.x + end.cos() * radius * w,
                center.y + end.sin() * radius * h,
            );
            arrow_head(
                painter,
                tip,
                Vec2::new(-end.sin(), end.cos()),
                w * 0.22,
                color,
            );
        }
        Icon::Music => {
            let c1 = p(0.32, 0.74);
            let c2 = p(0.66, 0.67);
            painter.circle_filled(c1, w * 0.11, color);
            painter.circle_filled(c2, w * 0.11, color);
            painter.line_segment([p(0.43, 0.74), p(0.43, 0.26)], stroke);
            painter.line_segment([p(0.77, 0.67), p(0.77, 0.20)], stroke);
            painter.line_segment([p(0.43, 0.26), p(0.77, 0.20)], Stroke::new(width * 1.8, color));
        }
        Icon::Disc => {
            painter.circle_stroke(rect.center(), w * 0.36, stroke);
            painter.circle_stroke(rect.center(), w * 0.14, stroke);
            painter.circle_filled(rect.center(), w * 0.05, color);
        }
        Icon::Sliders => {
            for (y, x) in [(0.28, 0.60), (0.50, 0.36), (0.72, 0.55)] {
                painter.line_segment([p(0.14, y), p(0.86, y)], stroke);
                painter.circle_filled(p(x, y), w * 0.10, color);
            }
        }
        Icon::Close => {
            painter.line_segment([p(0.28, 0.28), p(0.72, 0.72)], stroke);
            painter.line_segment([p(0.72, 0.28), p(0.28, 0.72)], stroke);
        }
        Icon::Playlist => {
            painter.line_segment([p(0.20, 0.30), p(0.68, 0.30)], stroke);
            painter.line_segment([p(0.20, 0.50), p(0.62, 0.50)], stroke);
            painter.line_segment([p(0.20, 0.70), p(0.56, 0.70)], stroke);
            painter.circle_filled(p(0.78, 0.70), w * 0.075, color);
            painter.line_segment([p(0.84, 0.70), p(0.84, 0.36)], stroke);
            painter.line_segment([p(0.84, 0.36), p(0.94, 0.32)], stroke);
        }
        Icon::Download => {
            painter.line_segment([p(0.50, 0.14), p(0.50, 0.60)], stroke);
            arrow_head(painter, p(0.50, 0.74), Vec2::new(0.0, 1.0), w * 0.22, color);
            painter.line_segment([p(0.26, 0.84), p(0.74, 0.84)], stroke);
        }
    }
}





pub fn paint_logo(painter: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    painter.circle_filled(c, r, ACCENT_DARK);
    painter.circle_filled(c, r * 0.90, Color32::from_rgb(27, 20, 17));
    painter.circle_filled(c, r * 0.74, ACCENT);
    painter.circle_filled(c, r * 0.36, Color32::from_rgb(24, 18, 15));
    painter.circle_filled(c, r * 0.12, ACCENT_BRIGHT);
    painter.circle_stroke(c, r * 0.98, Stroke::new(1.0, alpha(ACCENT_BRIGHT, 0.25)));
}

pub fn paint_vinyl(painter: &egui::Painter, rect: Rect, rotation: f32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    painter.circle_filled(c, r, Color32::from_rgb(22, 17, 15));
    painter.circle_stroke(c, r * 0.99, Stroke::new(1.0, Color32::from_rgb(52, 40, 33)));

    for k in 1..4 {
        painter.circle_stroke(
            c,
            r * (0.90 - k as f32 * 0.075),
            Stroke::new(1.0, Color32::from_rgb(42, 32, 27)),
        );
    }

    
    let mut gloss = Vec::with_capacity(14);
    for i in 0..=12 {
        let a = rotation + 0.30 + (i as f32 / 12.0) * 0.55;
        gloss.push(Pos2::new(
            c.x + a.cos() * r * 0.80,
            c.y + a.sin() * r * 0.80,
        ));
    }
    painter.add(Shape::line(gloss, Stroke::new(2.0, alpha(ACCENT_BRIGHT, 0.22))));

    painter.circle_filled(c, r * 0.40, Color32::from_rgb(30, 22, 18));
    painter.circle_filled(c, r * 0.34, ACCENT_DARK);
    painter.circle_stroke(c, r * 0.34, Stroke::new(1.0, alpha(ACCENT_BRIGHT, 0.18)));
    painter.circle_filled(c, r * 0.06, BG);

    
    let mark = Pos2::new(
        c.x + (rotation * 1.6).cos() * r * 0.22,
        c.y + (rotation * 1.6).sin() * r * 0.22,
    );
    painter.circle_filled(mark, r * 0.035, alpha(ACCENT_BRIGHT, 0.75));
}

pub fn paint_placeholder_art(
    painter: &egui::Painter,
    rect: Rect,
    initial: &str,
    active: bool,
    font_size: f32,
) {
    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(40, 29, 23));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        theme::font_semi(font_size),
        if active { ACCENT_BRIGHT } else { alpha(TEXT, 0.75) },
    );
}





fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, active: bool, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());

    if active {
        ui.painter()
            .circle_filled(rect.center(), size * 0.5, alpha(ACCENT, 0.16));
    } else if t > 0.01 {
        ui.painter()
            .circle_filled(rect.center(), size * 0.5, alpha(SURFACE_HOVER, t * 0.9));
    }

    let color = if active {
        ACCENT_BRIGHT
    } else if resp.hovered() {
        TEXT
    } else {
        TEXT_MUTED
    };
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(size * 0.52)),
        icon,
        color,
        (size * 0.075).clamp(1.3, 1.8),
    );

    resp.on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

pub fn play_button(ui: &mut Ui, playing: bool, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());

    ui.painter().circle_filled(
        rect.center() + Vec2::new(0.0, 2.0),
        size * 0.5,
        alpha(Color32::BLACK, 0.35),
    );
    ui.painter()
        .circle_filled(rect.center(), size * 0.5, lerp_color(ACCENT, ACCENT_BRIGHT, t));
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(size * (if playing { 0.40 } else { 0.44 }))),
        if playing { Icon::Pause } else { Icon::Play },
        Color32::from_rgb(24, 17, 13),
        2.0,
    );

    resp.on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(if playing { "Pausar (espaço)" } else { "Tocar (espaço)" })
}

pub fn nav_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    count: Option<usize>,
    active: bool,
) -> Response {
    let height = 40.0;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        Sense::click(),
    );
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());

    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), alpha(ACCENT, 0.15));
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 1.0, rect.top() + 11.0),
                Vec2::new(3.0, height - 22.0),
            ),
            CornerRadius::same(2),
            ACCENT,
        );
    } else if t > 0.01 {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), alpha(SURFACE_HOVER, t * 0.75));
    }

    let fg = if active {
        TEXT
    } else if resp.hovered() {
        TEXT
    } else {
        TEXT_MUTED
    };
    let icon_color = if active { ACCENT_BRIGHT } else { fg };

    paint_icon(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.left() + 27.0, rect.center().y),
            Vec2::splat(17.0),
        ),
        icon,
        icon_color,
        1.5,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 46.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font_medium(12.5),
        fg,
    );
    if let Some(count) = count {
        ui.painter().text(
            Pos2::new(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            theme::font(10.5),
            if active { alpha(ACCENT_BRIGHT, 0.9) } else { TEXT_FAINT },
        );
    }

    resp.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn action_button(ui: &mut Ui, label: &str, primary: bool, min_width: f32) -> Response {
    let text_color = if primary {
        Color32::from_rgb(26, 18, 13)
    } else {
        TEXT
    };
    let font = theme::font_medium(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, text_color);
    let padding = Vec2::new(16.0, 8.0);
    let size = Vec2::new(
        (galley.size().x + padding.x * 2.0).max(min_width),
        galley.size().y + padding.y * 2.0,
    );
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());

    let base = if primary { ACCENT } else { SURFACE };
    let hover = if primary { ACCENT_BRIGHT } else { SURFACE_HOVER };
    let fill = lerp_color(base, hover, t);

    ui.painter().rect_filled(rect, CornerRadius::same(9), fill);
    if !primary {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(9),
            Stroke::new(1.0, LINE),
            StrokeKind::Inside,
        );
    }
    ui.painter().galley(
        rect.center() - galley.size() * 0.5,
        galley,
        text_color,
    );

    resp.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn heart_button(ui: &mut Ui, rect: Rect, id: Id, filled: bool) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let color = if filled {
        ACCENT
    } else if resp.hovered() {
        TEXT
    } else {
        TEXT_FAINT
    };
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(17.0)),
        if filled { Icon::HeartFilled } else { Icon::Heart },
        color,
        1.4,
    );
    resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(
        if filled {
            "Remover dos favoritos"
        } else {
            "Adicionar aos favoritos"
        },
    )
}





pub fn seek_bar(ui: &mut Ui, fraction: f32, enabled: bool) -> SeekResult {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 18.0),
        Sense::click_and_drag(),
    );

    let active = enabled && (resp.hovered() || resp.dragged());
    let fraction = fraction.clamp(0.0, 1.0);
    let center_y = rect.center().y;
    let thickness = if active { 4.0 } else { 3.0 };
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), center_y - thickness * 0.5),
        Pos2::new(rect.right(), center_y + thickness * 0.5),
    );

    ui.painter()
        .rect_filled(track, CornerRadius::same(2), Color32::from_rgb(52, 41, 34));

    let mut result = SeekResult::default();
    let pointer_frac = resp.interact_pointer_pos().map(|pos| {
        ((pos.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0)
    });

    if enabled {
        if resp.dragged() {
            result.preview = pointer_frac;
        }
        if resp.drag_stopped() {
            result.commit = pointer_frac;
            result.preview = None;
        }
        if resp.clicked() {
            result.commit = pointer_frac;
            result.preview = pointer_frac;
        }
    }

    let shown = result
        .preview
        .or(result.commit)
        .unwrap_or(fraction)
        .clamp(0.0, 1.0);

    let x = rect.left() + rect.width() * shown;
    let filled = Rect::from_min_max(track.min, Pos2::new(x, track.max.y));
    ui.painter().rect_filled(
        filled,
        CornerRadius::same(2),
        if enabled { ACCENT } else { TEXT_FAINT },
    );

    if enabled && (active || resp.clicked()) {
        ui.painter().circle_filled(
            Pos2::new(x, center_y),
            5.5,
            if resp.dragged() { ACCENT_BRIGHT } else { ACCENT },
        );
        ui.painter().circle_stroke(
            Pos2::new(x, center_y),
            8.0,
            Stroke::new(1.0, alpha(ACCENT_BRIGHT, 0.25)),
        );
    }

    if enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    result
}

pub fn volume_bar(ui: &mut Ui, value: f32, width: f32) -> Option<f32> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());
    let hovered = resp.hovered() || resp.dragged();
    let center_y = rect.center().y;
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), center_y - 1.5),
        Pos2::new(rect.right(), center_y + 1.5),
    );

    let mut new_value = None;
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.clicked() || resp.dragged() {
            new_value = Some(((pos.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0));
        }
    }
    let shown = new_value.unwrap_or(value).clamp(0.0, 1.0);
    let x = rect.left() + rect.width() * shown;

    ui.painter()
        .rect_filled(track, CornerRadius::same(2), Color32::from_rgb(52, 41, 34));
    ui.painter().rect_filled(
        Rect::from_min_max(track.min, Pos2::new(x, track.max.y)),
        CornerRadius::same(2),
        if hovered { ACCENT_BRIGHT } else { ACCENT },
    );
    if hovered {
        ui.painter()
            .circle_filled(Pos2::new(x, center_y), 4.5, ACCENT_BRIGHT);
    }
    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);

    new_value
}


pub fn ellipsize(ui: &Ui, text: &str, font: FontId, max_width: f32) -> String {
    if max_width <= 4.0 {
        return String::new();
    }
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    if measure(text) <= max_width {
        return text.to_owned();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let candidate: String = chars[..mid].iter().collect::<String>() + "…";
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = chars[..lo].iter().collect();
    out.push('…');
    out
}


#[allow(dead_code)]
pub fn hairline(painter: &egui::Painter, rect: Rect, color: Color32) {
    painter.add(Shape::line_segment(
        [
            Pos2::new(rect.left(), rect.center().y),
            Pos2::new(rect.right(), rect.center().y),
        ],
        Stroke::new(1.0, color),
    ));
}
