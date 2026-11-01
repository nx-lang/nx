//! Opt-in direct Skia destination and deterministic host-supplied fonts.
#![cfg(feature = "native")]
use kurbo::{Affine, BezPath, PathEl, Rect};
use nx_graphics::{
    error, Bounds, Destination, Result, Scene, Stroke, Text, TextRequest, TextRun, TextService,
};
use sha2::{Digest, Sha256};
use skia_safe::{self as sk, Canvas, Color4f, Font, FontMgr, Paint, Path, TextBlob, Typeface};
use std::collections::BTreeMap;

struct Face {
    face: Typeface,
    identity: String,
}
struct RetainedRun {
    blob: TextBlob,
    request: TextRequest,
    font: String,
}
/// Exact family/weight/style selection; no automatic system font discovery or fallback.
#[derive(Default)]
pub struct Fonts {
    faces: BTreeMap<(String, u16, bool), Face>,
    runs: Vec<RetainedRun>,
    cache: BTreeMap<String, TextRun>,
}
impl Fonts {
    /// Releases retained layouts once the host has retired their prepared scenes.
    pub fn clear_runs(&mut self) {
        self.runs.clear();
        self.cache.clear();
    }

    pub fn register(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        bytes: &[u8],
    ) -> Result<String> {
        if !(1..=1000).contains(&weight) {
            return Err(error("", "font weight must be in 1..1000"));
        }
        let mgr = FontMgr::custom_empty()
            .ok_or_else(|| error("", "custom font loader unavailable on this platform"))?;
        let face = mgr
            .new_from_data(sk::Data::new_copy(bytes), None)
            .ok_or_else(|| error("", "invalid font bytes"))?;
        let identity = format!("sha256:{:x}", Sha256::digest(bytes));
        self.faces.insert(
            (family.into(), weight, italic),
            Face {
                face,
                identity: identity.clone(),
            },
        );
        Ok(identity)
    }
    pub fn fixture() -> Result<Self> {
        let mut fonts = Self::default();
        fonts.register(
            "Fixture",
            400,
            false,
            include_bytes!("../fixtures/fonts/DejaVuSans.ttf"),
        )?;
        Ok(fonts)
    }
    pub fn identities(&self) -> Vec<String> {
        self.faces.values().map(|f| f.identity.clone()).collect()
    }
}
struct MeasuredHandler {
    builder: sk::TextBlobBuilder,
    advance: f32,
    pen: f32,
    lines: usize,
    glyphs: Vec<u16>,
    positions: Vec<sk::Point>,
    missing: bool,
    ink: Option<Rect>,
}
impl Default for MeasuredHandler {
    fn default() -> Self {
        Self {
            builder: sk::TextBlobBuilder::new(),
            advance: 0.0,
            pen: 0.0,
            lines: 0,
            glyphs: vec![],
            positions: vec![],
            missing: false,
            ink: None,
        }
    }
}
impl sk::shaper::RunHandler for MeasuredHandler {
    fn begin_line(&mut self) {
        self.lines += 1;
        self.pen = 0.0;
    }
    fn run_info(&mut self, info: &sk::shaper::run_handler::RunInfo) {
        self.advance += info.advance.x;
    }
    fn commit_run_info(&mut self) {}
    fn run_buffer(
        &mut self,
        info: &sk::shaper::run_handler::RunInfo,
    ) -> sk::shaper::run_handler::Buffer<'_> {
        self.glyphs.resize(info.glyph_count, 0);
        self.positions
            .resize(info.glyph_count, sk::Point::default());
        sk::shaper::run_handler::Buffer::new(
            &mut self.glyphs,
            &mut self.positions,
            Some(sk::Point::new(self.pen, 0.0)),
        )
    }
    fn commit_run_buffer(&mut self, info: &sk::shaper::run_handler::RunInfo) {
        self.missing |= self.glyphs.contains(&0);
        let mut boxes = vec![sk::Rect::default(); self.glyphs.len()];
        info.font.get_bounds(&self.glyphs, &mut boxes, None);
        let mut outlines = BTreeMap::new();
        for ((b, p), glyph) in boxes.iter().zip(&self.positions).zip(&self.glyphs) {
            let b = outlines.entry(*glyph).or_insert_with(|| {
                info.font
                    .get_path(*glyph)
                    .map(|path| path.compute_tight_bounds())
                    .unwrap_or(*b)
            });
            if !b.is_empty() {
                let r = Rect::new(
                    (b.left + p.x) as f64,
                    (b.top + p.y) as f64,
                    (b.right + p.x) as f64,
                    (b.bottom + p.y) as f64,
                );
                self.ink = Some(self.ink.map(|old| old.union(r)).unwrap_or(r));
            }
        }

        let (glyphs, positions) = self
            .builder
            .alloc_run_pos(info.font, info.glyph_count, None);
        glyphs.copy_from_slice(&self.glyphs);
        positions.copy_from_slice(&self.positions);
        self.pen += info.advance.x;
    }
    fn commit_line(&mut self) {}
}
impl TextService for Fonts {
    fn shape(&mut self, r: &TextRequest) -> Result<TextRun> {
        nx_graphics::validate_text_request(r, "")?;
        let face = self
            .faces
            .get(&(r.family.clone(), r.weight, r.italic))
            .ok_or_else(|| {
                error(
                    "",
                    format!(
                        "missing supplied font: {} weight {} italic {}",
                        r.family, r.weight, r.italic
                    ),
                )
            })?;
        let key =
            serde_json::to_string(&(r, &face.identity)).map_err(|e| error("", e.to_string()))?;
        if let Some(run) = self.cache.get(&key) {
            return Ok(run.clone());
        }
        let mut font = Font::from_typeface(face.face.clone(), r.size as f32);
        font.set_subpixel(true)
            .set_linear_metrics(true)
            .set_hinting(sk::FontHinting::None)
            .set_baseline_snap(false);
        for ch in r.text.chars() {
            if font.unichar_to_glyph(ch as i32) == 0
                && !matches!(ch, '\u{200c}' | '\u{200d}' | '\u{fe0f}')
            {
                return Err(error(
                    "",
                    format!("font {} missing glyph U+{:04X}", r.family, ch as u32),
                ));
            }
        }
        let (_, metrics) = font.metrics();
        if r.text.is_empty() {
            return Ok(TextRun {
                handle: usize::MAX,
                font: face.identity.clone(),
                advance: 0.0,
                ink: Bounds::empty(),
                ascent: -(metrics.ascent as f64),
                descent: metrics.descent as f64,
                baseline: 0.0,
            });
        }
        let shaper = sk::Shaper::new_shape_then_wrap(Some(FontMgr::empty()))
            .ok_or_else(|| error("", "HarfBuzz shaper unavailable"))?;
        let mut handler = MeasuredHandler::default();
        shaper.shape(&r.text, &font, true, 1.0e9, &mut handler);
        if handler.missing {
            return Err(error("", "missing shaped glyph coverage"));
        }
        if handler.lines != 1 {
            return Err(error("", "text exceeds single-line shaping limits"));
        }
        let end = sk::Point::new(handler.advance, 0.0);
        let blob = handler
            .builder
            .make()
            .ok_or_else(|| error("", "text shaping failed"))?;
        let ink = Bounds {
            rect: handler
                .ink
                .map(|b| b.inflate(0.001 * r.size, 0.001 * r.size)),
            conservative: true,
        };
        let handle = self.runs.len();
        self.runs.push(RetainedRun {
            blob,
            request: r.clone(),
            font: face.identity.clone(),
        });
        let run = TextRun {
            handle,
            font: face.identity.clone(),
            advance: end.x as f64,
            ink,
            ascent: -(metrics.ascent as f64),
            descent: metrics.descent as f64,
            baseline: 0.0,
        };
        self.cache.insert(key, run.clone());
        Ok(run)
    }
}
fn path(p: &BezPath, evenodd: bool) -> Path {
    let mut out = sk::PathBuilder::new();
    for e in p.elements() {
        match *e {
            PathEl::MoveTo(p) => {
                out.move_to((p.x as f32, p.y as f32));
            }
            PathEl::LineTo(p) => {
                out.line_to((p.x as f32, p.y as f32));
            }
            PathEl::QuadTo(a, b) => {
                out.quad_to((a.x as f32, a.y as f32), (b.x as f32, b.y as f32));
            }
            PathEl::CurveTo(a, b, c) => {
                out.cubic_to(
                    (a.x as f32, a.y as f32),
                    (b.x as f32, b.y as f32),
                    (c.x as f32, c.y as f32),
                );
            }
            PathEl::ClosePath => {
                out.close();
            }
        }
    }
    out.set_fill_type(if evenodd {
        sk::PathFillType::EvenOdd
    } else {
        sk::PathFillType::Winding
    });
    out.detach()
}
fn paint(value: &nx_graphics::Paint) -> Result<Paint> {
    use nx_graphics::Paint as P;
    let mut p = Paint::default();
    p.set_anti_alias(true);
    let stops = match value {
        P::Color { color } => {
            p.set_color4f(Color4f::new(color[0], color[1], color[2], color[3]), None);
            return Ok(p);
        }
        P::LinearGradient { stops, .. } | P::RadialGradient { stops, .. } => stops,
    };
    let colors = stops
        .iter()
        .map(|s| Color4f::new(s.color[0], s.color[1], s.color[2], s.color[3]))
        .collect::<Vec<_>>();
    let positions = stops.iter().map(|s| s.offset as f32).collect::<Vec<_>>();
    let gradient = sk::gradient::Gradient::new(
        sk::gradient::Colors::new(
            &colors,
            Some(&positions),
            sk::TileMode::Clamp,
            sk::ColorSpace::new_srgb(),
        ),
        sk::gradient::Interpolation {
            in_premul: sk::gradient::interpolation::InPremul::Yes,
            color_space: sk::gradient::interpolation::ColorSpace::SRGB,
            ..Default::default()
        },
    );
    let shader = match value {
        P::LinearGradient { x1, y1, x2, y2, .. } => sk::gradient::shaders::linear_gradient(
            ((*x1 as f32, *y1 as f32), (*x2 as f32, *y2 as f32)),
            &gradient,
            None,
        ),
        P::RadialGradient { cx, cy, radius, .. } => sk::gradient::shaders::radial_gradient(
            ((*cx as f32, *cy as f32), *radius as f32),
            &gradient,
            None,
        ),
        _ => unreachable!(),
    }
    .ok_or_else(|| error("", "backend rejected gradient paint"))?;
    p.set_shader(shader);
    Ok(p)
}
/// Borrows a canvas for one frame. Frame exit and Drop restore its entry state; pixel rollback is
/// the offscreen host's responsibility. No command recorder is installed or needed.
pub struct SkiaDestination<'a> {
    canvas: &'a Canvas,
    fonts: &'a Fonts,
    entry: usize,
    depths: Vec<usize>,
}
impl<'a> SkiaDestination<'a> {
    pub fn new(canvas: &'a Canvas, fonts: &'a Fonts) -> Self {
        Self {
            canvas,
            fonts,
            entry: canvas.save_count(),
            depths: vec![],
        }
    }
}
impl Drop for SkiaDestination<'_> {
    fn drop(&mut self) {
        self.canvas.restore_to_count(self.entry);
    }
}
impl Destination for SkiaDestination<'_> {
    fn begin(&mut self) -> Result<()> {
        self.entry = self.canvas.save_count();
        self.canvas.save();
        Ok(())
    }
    fn end(&mut self) -> Result<()> {
        self.canvas.restore_to_count(self.entry);
        self.depths.clear();
        Ok(())
    }
    fn push(&mut self, t: Affine, clip: Option<(&BezPath, bool)>, opacity: f64) -> Result<()> {
        self.depths.push(self.canvas.save_count());
        self.canvas.save();
        let a = t.as_coeffs();
        self.canvas.concat(&sk::Matrix::new_all(
            a[0] as f32,
            a[2] as f32,
            a[4] as f32,
            a[1] as f32,
            a[3] as f32,
            a[5] as f32,
            0.0,
            0.0,
            1.0,
        ));
        if let Some((p, rule)) = clip {
            self.canvas.clip_path(&path(p, rule), None, true);
        }
        if opacity < 1.0 {
            let mut p = Paint::default();
            p.set_alpha_f(opacity as f32);
            self.canvas
                .save_layer(&sk::canvas::SaveLayerRec::default().paint(&p));
        }
        Ok(())
    }
    fn pop(&mut self) -> Result<()> {
        let count = self
            .depths
            .pop()
            .ok_or_else(|| error("", "unbalanced canvas scope"))?;
        self.canvas.restore_to_count(count);
        Ok(())
    }
    fn path(
        &mut self,
        p: &BezPath,
        evenodd: bool,
        fill: Option<&nx_graphics::Paint>,
        stroke: Option<&Stroke>,
    ) -> Result<()> {
        let geometry = path(p, evenodd);
        if let Some(c) = fill {
            self.canvas.draw_path(&geometry, &paint(c)?);
        }
        if let Some(s) = stroke {
            if s.width > 0.0 {
                let mut p = paint(&s.paint)?;
                p.set_style(sk::paint::Style::Stroke);
                p.set_stroke_width(s.width as f32);
                p.set_stroke_cap(match s.cap.as_str() {
                    "round" => sk::paint::Cap::Round,
                    "square" => sk::paint::Cap::Square,
                    _ => sk::paint::Cap::Butt,
                });
                p.set_stroke_join(match s.join.as_str() {
                    "round" => sk::paint::Join::Round,
                    "bevel" => sk::paint::Join::Bevel,
                    _ => sk::paint::Join::Miter,
                });
                p.set_stroke_miter(s.miter as f32);
                if !s.dashes.is_empty() {
                    let intervals = s.dashes.iter().map(|x| *x as f32).collect::<Vec<_>>();
                    p.set_path_effect(
                        sk::PathEffect::dash(&intervals, s.offset as f32)
                            .ok_or_else(|| error("", "invalid backend dash pattern"))?,
                    );
                }
                self.canvas.draw_path(&geometry, &p);
            }
        }
        Ok(())
    }
    fn text(&mut self, t: &Text, c: &nx_graphics::Paint) -> Result<()> {
        if t.request.text.is_empty() {
            return Ok(());
        }
        let blob = self
            .fonts
            .runs
            .get(t.run.handle)
            .ok_or_else(|| error("", "stale shaped text run"))?;
        if blob.request != t.request || blob.font != t.run.font {
            return Err(error(
                "",
                "shaped run belongs to a different text/font service",
            ));
        }
        self.canvas
            .draw_text_blob(&blob.blob, (t.x as f32, t.y as f32), &paint(c)?);
        Ok(())
    }
}
pub fn render_png(scene: &Scene, fonts: &Fonts) -> Result<Vec<u8>> {
    let w = (scene.config.width * scene.config.scale).ceil();
    let h = (scene.config.height * scene.config.scale).ceil();
    if w > i32::MAX as f64 || h > i32::MAX as f64 {
        return Err(error("", "surface exceeds backend dimensions"));
    }
    let mut surface = sk::surfaces::raster_n32_premul((w as i32, h as i32))
        .ok_or_else(|| error("", "cannot allocate raster surface"))?;
    surface.canvas().clear(sk::Color::TRANSPARENT);
    surface
        .canvas()
        .scale((scene.config.scale as f32, scene.config.scale as f32));
    scene.draw(&mut SkiaDestination::new(surface.canvas(), fonts))?;
    let data = surface
        .image_snapshot()
        .encode(None, sk::EncodedImageFormat::PNG, None)
        .ok_or_else(|| error("", "PNG encoding failed"))?;
    Ok(data.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_graphics::{Config, DeclarationResolver, Identity};
    use serde_json::json;
    struct Resolver;
    impl DeclarationResolver for Resolver {
        fn resolve(&self, p: &str) -> Option<Identity> {
            Some(Identity {
                module: nx_graphics::MODULE.into(),
                version: "test".into(),
                declaration: if p.is_empty() {
                    "Drawing"
                } else if p == "/children/0" {
                    "Group"
                } else {
                    "Rectangle"
                }
                .into(),
            })
        }
    }
    fn scene() -> Scene {
        let value = json!({"viewBox":{"width":40,"height":40},"children":[{"opacity":0.5,"children":[{"x":0,"y":0,"width":25,"height":25,"fill":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}},{"x":10,"y":0,"width":25,"height":25,"fill":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}}]}]});
        nx_graphics::prepare(
            &value,
            &Resolver,
            &mut Fonts::fixture().unwrap(),
            Config {
                width: 40.0,
                height: 40.0,
                graphics_version: "test".into(),
                ..Config::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn gradient_sampling_matches_shared_analytic_corpus() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/gradients/sampling.json")).unwrap();
        for case in corpus.as_array().unwrap() {
            let value: nx_graphics::Paint = serde_json::from_value(case["paint"].clone()).unwrap();
            let info = sk::ImageInfo::new(
                (101, 101),
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                sk::ColorSpace::new_srgb(),
            );
            let mut surface = sk::surfaces::raster(&info, None, None).unwrap();
            surface.canvas().clear(sk::Color::TRANSPARENT);
            surface
                .canvas()
                .draw_rect(sk::Rect::from_wh(101.0, 101.0), &paint(&value).unwrap());
            let info = sk::ImageInfo::new(
                (1, 1),
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                sk::ColorSpace::new_srgb(),
            );
            for sample in case["samples"].as_array().unwrap() {
                let mut pixel = [0u8; 4];
                assert!(surface.canvas().read_pixels(
                    &info,
                    &mut pixel,
                    4,
                    (
                        sample["x"].as_i64().unwrap() as i32,
                        sample["y"].as_i64().unwrap() as i32
                    )
                ));
                for i in 0..4 {
                    assert!(
                        (pixel[i] as f64 / 255.0 - sample["expected"][i].as_f64().unwrap()).abs()
                            <= 2.0 / 255.0,
                        "{}: {pixel:?}",
                        case["name"]
                    );
                }
            }
        }
    }
    #[test]
    fn gradient_text_retains_element_coordinates_at_all_sizes_and_anchors() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/gradients/sampling.json")).unwrap();
        let gradient: nx_graphics::Paint =
            serde_json::from_value(corpus[0]["paint"].clone()).unwrap();
        let mut fonts = Fonts::fixture().unwrap();
        for size in [16.0, 32.0] {
            let request = TextRequest {
                text: "Gradient".into(),
                family: "Fixture".into(),
                size,
                weight: 400,
                italic: false,
            };
            let run = fonts.shape(&request).unwrap();
            for anchor in [0.0, 0.5, 1.0] {
                let mut scene = scene();
                let mut text = scene.root.children[0].children[0].clone();
                scene.root.children.clear();
                scene.root.fill = None;
                scene.root.path = None;
                scene.root.clip = None;
                text.path = None;
                text.stroke = None;
                text.transform = Affine::new([1.25, 0.0, 0.0, 0.8, 10.0, 5.0]);
                text.text = Some(Text {
                    request: request.clone(),
                    run: run.clone(),
                    x: 55.0 - run.advance * anchor,
                    y: 70.0,
                });
                text.fill = Some(gradient.clone());
                scene.root.children.push(text);
                let info = sk::ImageInfo::new(
                    (240, 110),
                    sk::ColorType::RGBA8888,
                    sk::AlphaType::Premul,
                    sk::ColorSpace::new_srgb(),
                );
                let mut pixels = Vec::new();
                for solid in [false, true] {
                    if solid {
                        scene.root.children[0].fill = Some(nx_graphics::Paint::Color {
                            color: [1.0, 0.0, 0.0, 1.0],
                        });
                    }
                    let mut surface = sk::surfaces::raster(&info, None, None).unwrap();
                    surface.canvas().clear(sk::Color::TRANSPARENT);
                    scene
                        .draw(&mut SkiaDestination::new(surface.canvas(), &fonts))
                        .unwrap();
                    let mut bytes = vec![0u8; 240 * 110 * 4];
                    assert!(surface
                        .canvas()
                        .read_pixels(&info, &mut bytes, 240 * 4, (0, 0)));
                    pixels.push(bytes);
                }
                let mut checked = 0;
                for y in 0..110 {
                    for x in 0..240 {
                        let i = (y * 240 + x) * 4;
                        if pixels[1][i + 3] < 250 {
                            continue;
                        }
                        let local_x = (x as f64 + 0.5 - 10.0) / 1.25;
                        let alpha = (1.0 - (local_x - 0.5) / 100.0).clamp(0.0, 1.0);
                        assert!(
                            (pixels[0][i + 3] as f64 / 255.0
                                - alpha * pixels[1][i + 3] as f64 / 255.0)
                                .abs()
                                <= 2.0 / 255.0,
                            "size={size} anchor={anchor} x={x} y={y}"
                        );
                        checked += 1;
                    }
                }
                assert!(checked > 0);
            }
        }
    }
    #[test]
    fn direct_isolated_opacity_and_canvas_state() {
        let mut surface = sk::surfaces::raster_n32_premul((40, 40)).unwrap();
        surface.canvas().clear(sk::Color::TRANSPARENT);
        let entry = surface.canvas().save_count();
        let matrix = surface.canvas().local_to_device_as_3x3();
        let fonts = Fonts::fixture().unwrap();
        scene()
            .draw(&mut SkiaDestination::new(surface.canvas(), &fonts))
            .unwrap();
        assert_eq!(surface.canvas().save_count(), entry);
        assert_eq!(surface.canvas().local_to_device_as_3x3(), matrix);
        let info = sk::ImageInfo::new(
            (1, 1),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Unpremul,
            None,
        );
        for x in [5, 15, 30] {
            let mut pixel = [0u8; 4];
            assert!(surface.canvas().read_pixels(&info, &mut pixel, 4, (x, 10)));
            assert!((pixel[3] as i32 - 128).abs() <= 1, "{x}: {pixel:?}");
        }
        let png = render_png(&scene(), &fonts).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
    #[test]
    fn supplied_font_shaping_and_coverage() {
        let mut fonts = Fonts::fixture().unwrap();
        for text in ["office", "e\u{301}", "Ελλάδα"] {
            let request = TextRequest {
                text: text.into(),
                family: "Fixture".into(),
                size: 24.0,
                weight: 400,
                italic: false,
            };
            let run = fonts.shape(&request).unwrap();
            assert!(run.advance > 0.0);
            assert!(run.ascent > 0.0);
            assert!(run.ink.rect.is_some());
            assert_eq!(run.font, fonts.identities()[0]);
            assert!(fonts.runs.get(run.handle).is_some());
        }
        let mut request = TextRequest {
            text: "x".into(),
            family: "missing".into(),
            size: 24.0,
            weight: 400,
            italic: false,
        };
        assert!(fonts.shape(&request).is_err());
        request.family = "Fixture".into();
        request.text = "\u{10ffff}".into();
        assert!(fonts.shape(&request).is_err());
        request.text = "two\nlines".into();
        assert!(fonts.shape(&request).is_err());
    }
    #[test]
    fn canvas_state_restores_after_drawing_failure() {
        let mut fonts = Fonts::fixture().unwrap();
        let request = TextRequest {
            text: "failure".into(),
            family: "Fixture".into(),
            size: 16.0,
            weight: 400,
            italic: false,
        };
        let mut run = fonts.shape(&request).unwrap();
        run.handle = usize::MAX;
        let mut scene = scene();
        scene.root.children[0].children[0].text = Some(Text {
            request,
            run,
            x: 0.0,
            y: 20.0,
        });
        let mut surface = sk::surfaces::raster_n32_premul((40, 40)).unwrap();
        surface.canvas().translate((3.0, 4.0));
        let entry = surface.canvas().save_count();
        let matrix = surface.canvas().local_to_device_as_3x3();
        assert!(scene
            .draw(&mut SkiaDestination::new(surface.canvas(), &fonts))
            .is_err());
        assert_eq!(surface.canvas().save_count(), entry);
        assert_eq!(surface.canvas().local_to_device_as_3x3(), matrix);
    }
    #[test]
    fn evenodd_clip_hole_is_independent_of_nonzero_fill() {
        struct PathResolver;
        impl DeclarationResolver for PathResolver {
            fn resolve(&self, p: &str) -> Option<Identity> {
                Some(Identity {
                    module: nx_graphics::MODULE.into(),
                    version: "test".into(),
                    declaration: if p.is_empty() { "Drawing" } else { "Path" }.into(),
                })
            }
        }
        let value = json!({"viewBox":{"width":40,"height":40},"children":[{"data":"M0 0H40V40H0Z","fill":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}},"fillRule":"nonzero","clipPath":"M0 0H40V40H0Z M10 10H30V30H10Z","clipRule":"evenodd"}]});
        let fonts = Fonts::fixture().unwrap();
        let scene = nx_graphics::prepare(
            &value,
            &PathResolver,
            &mut Fonts::fixture().unwrap(),
            Config {
                width: 40.0,
                height: 40.0,
                graphics_version: "test".into(),
                ..Config::default()
            },
        )
        .unwrap();
        let mut surface = sk::surfaces::raster_n32_premul((40, 40)).unwrap();
        surface.canvas().clear(sk::Color::TRANSPARENT);
        scene
            .draw(&mut SkiaDestination::new(surface.canvas(), &fonts))
            .unwrap();
        let info = sk::ImageInfo::new(
            (1, 1),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Unpremul,
            None,
        );
        for (x, alpha) in [(5, 255), (20, 0)] {
            let mut pixel = [0u8; 4];
            assert!(surface.canvas().read_pixels(&info, &mut pixel, 4, (x, 20)));
            assert_eq!(pixel[3], alpha);
        }
    }
    #[test]
    fn text_revisions_ignore_allocation_history_and_reject_wrong_run_service() {
        struct TextResolver;
        impl DeclarationResolver for TextResolver {
            fn resolve(&self, p: &str) -> Option<Identity> {
                Some(Identity {
                    module: nx_graphics::MODULE.into(),
                    version: "test".into(),
                    declaration: if p.is_empty() { "Drawing" } else { "Text" }.into(),
                })
            }
        }
        let v = json!({"viewBox":{"width":100,"height":100},"children":[{"text":"label","fontFamily":"Fixture"}]});
        let mut fonts = Fonts::fixture().unwrap();
        let config = Config {
            graphics_version: "test".into(),
            ..Config::default()
        };
        let first = nx_graphics::prepare(&v, &TextResolver, &mut fonts, config.clone()).unwrap();
        let second = nx_graphics::prepare(&v, &TextResolver, &mut fonts, config).unwrap();
        assert_eq!(first.revision, second.revision);
        assert_eq!(
            first.root.children[0].text.as_ref().unwrap().run.handle,
            second.root.children[0].text.as_ref().unwrap().run.handle
        );
        let mut wrong_fonts = Fonts::fixture().unwrap();
        wrong_fonts
            .shape(&TextRequest {
                text: "other".into(),
                family: "Fixture".into(),
                size: 16.0,
                weight: 400,
                italic: false,
            })
            .unwrap();
        assert!(render_png(&first, &wrong_fonts).is_err());
    }
    #[test]
    fn unhinted_advances_scale_and_ink_uses_glyph_bearings() {
        let mut fonts = Fonts::fixture().unwrap();
        let request = |text: &str, size| TextRequest {
            text: text.into(),
            family: "Fixture".into(),
            size,
            weight: 400,
            italic: false,
        };
        let small = fonts.shape(&request("Hello", 2.0)).unwrap();
        let large = fonts.shape(&request("Hello", 20.0)).unwrap();
        assert!((small.advance * 10.0 - large.advance).abs() < 0.01);
        assert!((large.advance - 50.693359375).abs() < 0.02);
        let h = fonts.shape(&request("H", 20.0)).unwrap();
        let ink = h.ink.rect.unwrap();
        assert!(ink.x0 > 1.0 && ink.x0 < 3.0, "{ink:?}");
        assert!(ink.width() < 16.0);
        let count = fonts.runs.len();
        for _ in 0..100 {
            fonts.shape(&request("Hello", 2.0)).unwrap();
        }
        assert_eq!(fonts.runs.len(), count);
        fonts.clear_runs();
        assert!(fonts.runs.is_empty());
    }
}

#[cfg(test)]
mod corpus_tests;
