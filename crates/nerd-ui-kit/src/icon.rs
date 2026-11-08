//! Vector icon set in the spirit of SF Symbols.
//!
//! Icons are painted at the current text colour with a uniform stroke, so they
//! stay crisp on Retina without shipping font or SVG assets.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Library,
    Timeline,
    Titles,
    Live,
    Settings,
    Sidebar,
    Inspector,
    Plus,
    Minus,
    Search,
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    ChevronUp,
    More,
    Close,
    Play,
    Pause,
    Stop,
    SkipForward,
    SkipBack,
    Go,
    Panic,
    Blackout,
    Monitor,
    Volume,
    Music,
    Video,
    Folder,
    File,
    Trash,
    Warning,
    Check,
    Clock,
    Waveform,
    Link,
    Refresh,
    Download,
    Info,
    Keyboard,
    ArrowRight,
    Sort,
    Grid,
    List,
    Save,
    Open,
    Undo,
    Redo,
    Maximize,
    Dot,
    Sparkle,
}

/// Paints an icon inside `rect`. The icon is centred and scaled to the smaller
/// side; a 16 or 20 pt square gives the intended optical size.
pub fn icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let side = rect.width().min(rect.height());
    if side <= 0.0 {
        return;
    }
    let origin = rect.center() - Vec2::splat(side / 2.0);
    let p = |x: f32, y: f32| Pos2::new(origin.x + x / 24.0 * side, origin.y + y / 24.0 * side);
    let stroke = Stroke::new(width.max(1.0), color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let path = |points: &[(f32, f32)], closed: bool| {
        let mut points: Vec<Pos2> = points.iter().map(|&(x, y)| p(x, y)).collect();
        if closed {
            points.push(points[0]);
        }
        painter.add(Shape::line(points, stroke));
    };
    let circle = |x: f32, y: f32, r: f32| {
        painter.circle_stroke(p(x, y), r / 24.0 * side, stroke);
    };
    let dot = |x: f32, y: f32, r: f32| {
        painter.circle_filled(p(x, y), r / 24.0 * side, color);
    };
    let fill = |points: &[(f32, f32)]| {
        painter.add(Shape::convex_polygon(
            points.iter().map(|&(x, y)| p(x, y)).collect(),
            color,
            Stroke::NONE,
        ));
    };
    let round_rect = |x: f32, y: f32, w: f32, h: f32, r: f32| {
        painter.rect_stroke(
            Rect::from_min_size(p(x, y), Vec2::new(w / 24.0 * side, h / 24.0 * side)),
            egui::CornerRadius::same((r / 24.0 * side).max(1.0) as u8),
            stroke,
            egui::StrokeKind::Inside,
        );
    };
    let filled_round = |x: f32, y: f32, w: f32, h: f32, r: f32| {
        painter.rect_filled(
            Rect::from_min_size(p(x, y), Vec2::new(w / 24.0 * side, h / 24.0 * side)),
            egui::CornerRadius::same((r / 24.0 * side).max(1.0) as u8),
            color,
        );
    };

    match icon {
        Icon::Library => {
            round_rect(3.0, 4.0, 18.0, 16.0, 2.5);
            line((7.0, 9.0), (17.0, 9.0));
            line((7.0, 13.0), (15.0, 13.0));
            line((7.0, 17.0), (12.0, 17.0));
        }
        Icon::Timeline => {
            round_rect(3.0, 4.0, 18.0, 16.0, 2.5);
            line((7.0, 8.0), (7.0, 16.0));
            line((12.0, 6.0), (12.0, 16.0));
            line((17.0, 10.0), (17.0, 16.0));
        }
        Icon::Titles => {
            round_rect(3.0, 4.0, 18.0, 16.0, 2.5);
            line((7.0, 8.0), (17.0, 8.0));
            line((7.0, 12.0), (14.0, 12.0));
            line((7.0, 16.0), (11.0, 16.0));
        }
        Icon::Live => {
            circle(12.0, 12.0, 8.5);
            fill(&[(10.0, 8.0), (16.5, 12.0), (10.0, 16.0)]);
        }
        Icon::Settings => {
            circle(12.0, 12.0, 4.0);
            for angle in 0..8 {
                let a = angle as f32 * std::f32::consts::TAU / 8.0;
                let (sin, cos) = a.sin_cos();
                line(
                    (12.0 + cos * 7.0, 12.0 + sin * 7.0),
                    (12.0 + cos * 9.5, 12.0 + sin * 9.5),
                );
            }
        }
        Icon::Sidebar => {
            round_rect(3.0, 4.0, 18.0, 16.0, 2.5);
            line((9.0, 4.0), (9.0, 20.0));
        }
        Icon::Inspector => {
            round_rect(3.0, 4.0, 18.0, 16.0, 2.5);
            line((15.0, 4.0), (15.0, 20.0));
        }
        Icon::Plus => {
            line((12.0, 5.0), (12.0, 19.0));
            line((5.0, 12.0), (19.0, 12.0));
        }
        Icon::Minus => line((5.0, 12.0), (19.0, 12.0)),
        Icon::Search => {
            circle(10.5, 10.5, 5.5);
            line((15.0, 15.0), (20.0, 20.0));
        }
        Icon::ChevronDown => path(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)], false),
        Icon::ChevronRight => path(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)], false),
        Icon::ChevronLeft => path(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)], false),
        Icon::ChevronUp => path(&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)], false),
        Icon::More => {
            dot(6.0, 12.0, 1.6);
            dot(12.0, 12.0, 1.6);
            dot(18.0, 12.0, 1.6);
        }
        Icon::Close => {
            line((6.0, 6.0), (18.0, 18.0));
            line((18.0, 6.0), (6.0, 18.0));
        }
        Icon::Play => fill(&[(8.0, 5.0), (19.0, 12.0), (8.0, 19.0)]),
        Icon::Pause => {
            filled_round(6.5, 5.0, 4.0, 14.0, 1.4);
            filled_round(13.5, 5.0, 4.0, 14.0, 1.4);
        }
        Icon::Stop => filled_round(6.0, 6.0, 12.0, 12.0, 2.2),
        Icon::SkipForward => {
            fill(&[(5.0, 5.0), (15.0, 12.0), (5.0, 19.0)]);
            filled_round(16.0, 5.0, 3.0, 14.0, 1.2);
        }
        Icon::SkipBack => {
            fill(&[(19.0, 5.0), (9.0, 12.0), (19.0, 19.0)]);
            filled_round(5.0, 5.0, 3.0, 14.0, 1.2);
        }
        Icon::Go => {
            fill(&[(5.0, 4.0), (20.0, 12.0), (5.0, 20.0)]);
        }
        Icon::Panic => {
            path(&[(12.0, 3.0), (21.0, 20.0), (3.0, 20.0)], true);
            line((12.0, 9.0), (12.0, 14.0));
            dot(12.0, 17.0, 1.4);
        }
        Icon::Blackout => {
            painter.add(Shape::line(
                (0..=16)
                    .map(|i| {
                        let a = std::f32::consts::PI * 0.55
                            + i as f32 / 16.0 * std::f32::consts::PI * 0.9;
                        p(12.0 + a.cos() * 8.0, 12.0 + a.sin() * 8.0)
                    })
                    .collect(),
                stroke,
            ));
            path(
                &[(13.0, 4.0), (19.0, 11.0), (17.0, 18.0), (10.0, 19.0)],
                false,
            );
        }
        Icon::Monitor => {
            round_rect(3.0, 4.0, 18.0, 13.0, 2.0);
            line((8.0, 20.0), (16.0, 20.0));
            line((12.0, 17.0), (12.0, 20.0));
        }
        Icon::Volume => {
            fill(&[
                (4.0, 9.0),
                (8.0, 9.0),
                (13.0, 5.0),
                (13.0, 19.0),
                (8.0, 15.0),
                (4.0, 15.0),
            ]);
            path(&[(16.0, 9.0), (18.0, 12.0), (16.0, 15.0)], false);
            path(&[(18.5, 7.0), (21.0, 12.0), (18.5, 17.0)], false);
        }
        Icon::Music => {
            line((9.0, 18.0), (9.0, 6.0));
            line((9.0, 6.0), (19.0, 4.0));
            line((19.0, 4.0), (19.0, 15.0));
            circle(6.5, 18.0, 2.5);
            circle(16.5, 15.0, 2.5);
        }
        Icon::Video => {
            round_rect(3.0, 5.0, 18.0, 14.0, 2.5);
            fill(&[(10.0, 9.0), (16.0, 12.0), (10.0, 15.0)]);
        }
        Icon::Folder => {
            path(
                &[
                    (3.0, 7.0),
                    (10.0, 7.0),
                    (12.0, 9.0),
                    (21.0, 9.0),
                    (21.0, 19.0),
                    (3.0, 19.0),
                ],
                true,
            );
        }
        Icon::File => {
            path(
                &[
                    (6.0, 3.0),
                    (14.0, 3.0),
                    (19.0, 8.0),
                    (19.0, 21.0),
                    (6.0, 21.0),
                ],
                true,
            );
            line((14.0, 3.0), (14.0, 8.0));
            line((14.0, 8.0), (19.0, 8.0));
        }
        Icon::Trash => {
            line((4.0, 7.0), (20.0, 7.0));
            line((9.0, 7.0), (9.0, 4.0));
            line((15.0, 7.0), (15.0, 4.0));
            path(&[(6.0, 7.0), (7.0, 21.0), (17.0, 21.0), (18.0, 7.0)], false);
            line((10.0, 10.0), (10.0, 18.0));
            line((14.0, 10.0), (14.0, 18.0));
        }
        Icon::Warning => {
            path(&[(12.0, 3.0), (21.0, 20.0), (3.0, 20.0)], true);
            line((12.0, 9.0), (12.0, 14.0));
            dot(12.0, 17.0, 1.4);
        }
        Icon::Check => path(&[(4.0, 12.0), (9.5, 18.0), (20.0, 6.0)], false),
        Icon::Clock => {
            circle(12.0, 12.0, 9.0);
            line((12.0, 7.0), (12.0, 12.5));
            line((12.0, 12.5), (16.0, 15.0));
        }
        Icon::Waveform => {
            for (i, h) in [4.0, 7.0, 10.0, 6.0, 9.0, 12.0, 7.0, 4.0]
                .iter()
                .enumerate()
            {
                let x = 4.0 + i as f32 * 2.3;
                line((x, 12.0 - h), (x, 12.0 + h));
            }
        }
        Icon::Link => {
            path(&[(9.0, 15.0), (15.0, 9.0)], false);
            path(&[(7.0, 17.0), (5.0, 19.0), (5.0, 21.0)], false);
            circle(8.0, 16.0, 3.2);
            circle(16.0, 8.0, 3.2);
        }
        Icon::Refresh => {
            path(&[(20.0, 5.0), (20.0, 11.0), (14.0, 11.0)], false);
            path(&[(4.0, 19.0), (4.0, 13.0), (10.0, 13.0)], false);
            painter.add(Shape::line(
                (0..=24)
                    .map(|i| {
                        let a = i as f32 / 24.0 * std::f32::consts::TAU;
                        p(12.0 + a.cos() * 8.0, 12.0 + a.sin() * 8.0)
                    })
                    .collect(),
                stroke,
            ));
        }
        Icon::Download => {
            line((12.0, 3.0), (12.0, 15.0));
            path(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)], false);
            line((4.0, 20.0), (20.0, 20.0));
        }
        Icon::Info => {
            circle(12.0, 12.0, 9.0);
            dot(12.0, 8.0, 1.3);
            line((12.0, 11.0), (12.0, 17.0));
        }
        Icon::Keyboard => {
            round_rect(2.0, 6.0, 20.0, 12.0, 2.0);
            for x in [5.0, 9.0, 13.0, 17.0] {
                dot(x, 10.0, 1.0);
            }
            line((7.0, 14.5), (17.0, 14.5));
        }
        Icon::ArrowRight => {
            line((4.0, 12.0), (20.0, 12.0));
            path(&[(14.0, 6.0), (20.0, 12.0), (14.0, 18.0)], false);
        }
        Icon::Sort => {
            line((4.0, 6.0), (16.0, 6.0));
            line((4.0, 12.0), (12.0, 12.0));
            line((4.0, 18.0), (8.0, 18.0));
        }
        Icon::Grid => {
            round_rect(3.0, 3.0, 7.0, 7.0, 1.5);
            round_rect(14.0, 3.0, 7.0, 7.0, 1.5);
            round_rect(3.0, 14.0, 7.0, 7.0, 1.5);
            round_rect(14.0, 14.0, 7.0, 7.0, 1.5);
        }
        Icon::List => {
            for y in [6.0, 12.0, 18.0] {
                line((8.0, y), (20.0, y));
                dot(4.0, y, 1.3);
            }
        }
        Icon::Save => {
            path(
                &[
                    (4.0, 4.0),
                    (17.0, 4.0),
                    (20.0, 7.0),
                    (20.0, 20.0),
                    (4.0, 20.0),
                ],
                true,
            );
            round_rect(7.0, 4.0, 8.0, 6.0, 1.0);
            round_rect(7.0, 14.0, 10.0, 6.0, 1.0);
        }
        Icon::Open => {
            path(
                &[
                    (3.0, 7.0),
                    (10.0, 7.0),
                    (12.0, 9.0),
                    (21.0, 9.0),
                    (21.0, 19.0),
                    (3.0, 19.0),
                ],
                true,
            );
            line((3.0, 11.0), (21.0, 11.0));
        }
        Icon::Undo => {
            path(&[(9.0, 7.0), (4.0, 12.0), (9.0, 17.0)], false);
            path(
                &[(4.0, 12.0), (15.0, 12.0), (19.0, 16.0), (19.0, 20.0)],
                false,
            );
        }
        Icon::Redo => {
            path(&[(15.0, 7.0), (20.0, 12.0), (15.0, 17.0)], false);
            path(
                &[(20.0, 12.0), (9.0, 12.0), (5.0, 16.0), (5.0, 20.0)],
                false,
            );
        }
        Icon::Maximize => {
            line((4.0, 9.0), (4.0, 4.0));
            line((4.0, 4.0), (9.0, 4.0));
            line((20.0, 15.0), (20.0, 20.0));
            line((20.0, 20.0), (15.0, 20.0));
        }
        Icon::Dot => dot(12.0, 12.0, 3.0),
        Icon::Sparkle => {
            path(
                &[
                    (12.0, 3.0),
                    (14.0, 10.0),
                    (21.0, 12.0),
                    (14.0, 14.0),
                    (12.0, 21.0),
                    (10.0, 14.0),
                    (3.0, 12.0),
                    (10.0, 10.0),
                ],
                true,
            );
        }
    }
}
