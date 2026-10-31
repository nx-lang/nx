//! Backend-neutral preparation and direct frame painting for the unstable graphics-core profile.
//! Hosts supply declaration identities separately from generic NX value serialization.
mod paint;
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
pub use paint::{GradientStop, Paint};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PROFILE: &str = "graphics-core-v4";
pub const MODULE: &str = "@nx/graphics/graphics.nx";
pub type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub element: String,
    pub code: String,
    pub message: String,
    pub provenance: Option<Value>,
}
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.code, self.message, self.element)
    }
}
impl std::error::Error for Diagnostic {}
pub fn error(element: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        element: if element.starts_with("element") {
            element.into()
        } else {
            format!("element{element}")
        },
        code: "graphics-profile".into(),
        message: message.into(),
        provenance: None,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub module: String,
    pub version: String,
    pub declaration: String,
}
/// A trusted host resolves the declaration of the value at a JSON pointer, using linked program
/// metadata. Merely looking up the value's $type string is insufficient.
pub trait DeclarationResolver {
    fn resolve(&self, path: &str) -> Option<Identity>;
    fn provenance(&self, _path: &str) -> Option<Value> {
        None
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub gradient_stops: usize,
    pub nodes: usize,
    pub depth: usize,
    pub path_commands: usize,
    pub coordinate: f64,
    pub surface_pixels: f64,
    /// Maximum peak area of concurrent isolated opacity layers, in physical pixels.
    pub layer_pixels: f64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            gradient_stops: 100000,
            nodes: 10000,
            depth: 64,
            path_commands: 100000,
            coordinate: 1e7,
            surface_pixels: 16_777_216.0,
            layer_pixels: 67_108_864.0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    /// Host rasterization uncertainty, in physical pixels, applied to painted text bounds.
    #[serde(default)]
    pub text_ink_padding: f64,
    pub program_revision: String,
    pub renderer: String,
    pub graphics_version: String,
    pub fonts: Vec<String>,
    pub limits: Limits,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            scale: 1.0,
            text_ink_padding: 0.0,
            program_revision: String::new(),
            renderer: "neutral".into(),
            graphics_version: String::new(),
            fonts: vec![],
            limits: Limits::default(),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub rect: Option<Rect>,
    pub conservative: bool,
}
impl Bounds {
    pub fn empty() -> Self {
        Self {
            rect: None,
            conservative: false,
        }
    }
    fn intersect(self, r: Rect) -> Self {
        Self {
            rect: self.rect.and_then(|b| {
                let i = b.intersect(r);
                if i.width() <= 0.0 || i.height() <= 0.0 {
                    None
                } else {
                    Some(i)
                }
            }),
            conservative: self.conservative || self.rect.is_some_and(|b| !r.contains_rect(b)),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f64,
    pub cap: String,
    pub join: String,
    pub miter: f64,
    pub dashes: Vec<f64>,
    pub offset: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextRequest {
    pub text: String,
    pub family: String,
    pub size: f64,
    pub weight: u16,
    pub italic: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    /// Transient host resource handle; not part of scene identity or an inspection manifest.
    #[serde(skip)]
    pub handle: usize,
    pub font: String,
    pub advance: f64,
    pub ink: Bounds,
    pub ascent: f64,
    pub descent: f64,
    pub baseline: f64,
}
/// Shared scene/standalone text-profile validation, before host font lookup.
pub fn validate_text_request(request: &TextRequest, path: &str) -> Result<()> {
    if !request.size.is_finite() || request.size <= 0.0 {
        return Err(error(path, "font size must be positive and finite"));
    }
    if request.size > 1e6 || (request.size as f32) == 0.0 {
        return Err(error(path, "font size outside supported range"));
    }
    if !(1..=1000).contains(&request.weight) {
        return Err(error(path, "invalid font weight"));
    }
    if request.text.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return Err(error(path, "multiline text is unsupported"));
    }
    if request.text.len() > 100_000 {
        return Err(error(path, "text length limit exceeded"));
    }
    Ok(())
}
pub trait TextService {
    fn shape(&mut self, request: &TextRequest) -> Result<TextRun>;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub request: TextRequest,
    pub run: TextRun,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub parent: Option<String>,
    pub order: usize,
    pub identity: Identity,
    pub provenance: Option<Value>,
    pub accessibility: Option<Value>,
    pub transform: Affine,
    pub clip: Option<BezPath>,
    pub clip_evenodd: bool,
    pub opacity: f64,
    pub path: Option<BezPath>,
    pub evenodd: bool,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
    pub text: Option<Text>,
    pub geometry: Bounds,
    pub painted: Bounds,
    pub visible: Bounds,
    pub children: Vec<Node>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub revision: String,
    pub profile: String,
    pub config: Config,
    pub root: Node,
}
impl Scene {
    pub fn inspect(&self, revision: &str, id: &str) -> Result<&Node> {
        if revision != self.revision {
            return Err(error(id, "stale revision"));
        }
        fn find<'a>(n: &'a Node, id: &str) -> Option<&'a Node> {
            if n.id == id {
                Some(n)
            } else {
                n.children.iter().find_map(|c| find(c, id))
            }
        }
        find(&self.root, id).ok_or_else(|| error(id, "unknown element"))
    }
    pub fn draw(&self, destination: &mut dyn Destination) -> Result<()> {
        if self.profile != PROFILE {
            return Err(error("", "unsupported graphics profile"));
        }
        destination.begin()?;
        let result = draw_node(&self.root, destination);
        let restored = destination.end();
        result.and(restored)
    }
}
/// Each scope is balanced on every return path, including destination failures. Implementations
/// must make failed save operations atomic; end restores the complete frame entry state.
pub trait Destination {
    fn begin(&mut self) -> Result<()>;
    fn end(&mut self) -> Result<()>;
    fn push(
        &mut self,
        transform: Affine,
        clip: Option<(&BezPath, bool)>,
        opacity: f64,
    ) -> Result<()>;
    fn pop(&mut self) -> Result<()>;
    fn path(
        &mut self,
        path: &BezPath,
        evenodd: bool,
        fill: Option<&Paint>,
        stroke: Option<&Stroke>,
    ) -> Result<()>;
    fn text(&mut self, text: &Text, paint: &Paint) -> Result<()>;
}
fn draw_node(n: &Node, d: &mut dyn Destination) -> Result<()> {
    if n.opacity == 0.0 {
        return Ok(());
    }
    let locate = |mut e: Diagnostic| {
        if e.element.is_empty() || e.element == "element" {
            e.element = n.id.clone();
            e.provenance = n.provenance.clone();
        }
        e
    };
    d.push(
        n.transform,
        n.clip.as_ref().map(|p| (p, n.clip_evenodd)),
        n.opacity,
    )
    .map_err(locate)?;
    let result = (|| {
        if let Some(p) = &n.path {
            d.path(p, n.evenodd, n.fill.as_ref(), n.stroke.as_ref())?;
        }
        if let Some(t) = &n.text {
            if let Some(color) = &n.fill {
                d.text(t, color)?;
            }
        }
        for child in &n.children {
            draw_node(child, d)?;
        }
        Ok(())
    })();
    let restored = d.pop();
    result.and(restored).map_err(locate)
}

fn numeric(v: &Value, path: &str, limit: f64) -> Result<f64> {
    let n = v
        .as_f64()
        .ok_or_else(|| error(path, "expected a finite number"))?;
    if !n.is_finite()
        || n.abs() > limit
        || !(n as f32).is_finite()
        || (n != 0.0 && (n as f32) == 0.0)
    {
        Err(error(path, "number outside host range"))
    } else {
        Ok(n)
    }
}
fn num(v: &Value, key: &str, default: f64, path: &str, l: &Limits) -> Result<f64> {
    match v.get(key) {
        Some(n) if !absent(n) => numeric(n, path, l.coordinate),
        _ => Ok(default),
    }
}
fn absent(v: &Value) -> bool {
    v.is_null() || v.as_array().is_some_and(|a| a.is_empty())
}
fn text<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(default)
}
fn choice(v: &Value, key: &str, default: &str, choices: &[&str], path: &str) -> Result<String> {
    let s = text(v, key, default);
    if choices.contains(&s) {
        Ok(s.into())
    } else {
        Err(error(path, format!("unsupported {key}: {s}")))
    }
}

fn rule(v: &Value, key: &str, path: &str) -> Result<bool> {
    Ok(choice(v, key, "nonzero", &["nonzero", "evenodd"], path)? == "evenodd")
}
fn list(v: &Value) -> Vec<&Value> {
    if absent(v) {
        vec![]
    } else if let Some(a) = v.as_array() {
        a.iter().collect()
    } else {
        vec![v]
    }
}
fn svg(data: &str, path: &str, l: &Limits, remaining: usize) -> Result<BezPath> {
    if let Some(reason) = data.strip_prefix("nx-graphics-invalid: ") {
        let mut e = error(path, reason);
        e.code = "graphics-helper-invalid".into();
        return Err(e);
    }
    if data.is_empty() {
        return Ok(BezPath::new());
    }
    let mut count = 0;
    for segment in svgtypes::PathParser::from(data) {
        let segment = segment.map_err(|e| error(path, format!("malformed SVG path: {e}")))?;
        use svgtypes::PathSegment::*;
        let numbers = match segment {
            MoveTo { x, y, .. } | LineTo { x, y, .. } | SmoothQuadratic { x, y, .. } => vec![x, y],
            HorizontalLineTo { x, .. } => vec![x],
            VerticalLineTo { y, .. } => vec![y],
            CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
                ..
            } => vec![x1, y1, x2, y2, x, y],
            SmoothCurveTo { x2, y2, x, y, .. } => vec![x2, y2, x, y],
            Quadratic { x1, y1, x, y, .. } => vec![x1, y1, x, y],
            EllipticalArc {
                rx,
                ry,
                x_axis_rotation,
                x,
                y,
                ..
            } => vec![rx, ry, x_axis_rotation, x, y],
            ClosePath { .. } => vec![],
        };
        for n in numbers {
            numeric(&Value::from(n), path, l.coordinate)?;
        }

        count += 1;
        if count > remaining {
            return Err(error(path, "path command limit exceeded"));
        }
    }
    let p = normalize_svg(data, remaining).map_err(|e| error(path, e.to_string()))?;
    for e in p.elements() {
        for point in match *e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => vec![p],
            PathEl::QuadTo(a, b) => vec![a, b],
            PathEl::CurveTo(a, b, c) => vec![a, b, c],
            PathEl::ClosePath => vec![],
        } {
            numeric(&Value::from(point.x), path, l.coordinate)?;
            numeric(&Value::from(point.y), path, l.coordinate)?;
        }
    }
    Ok(p)
}
fn transforms(v: &Value, path: &str, l: &Limits) -> Result<Affine> {
    let mut result = Affine::IDENTITY;
    for t in list(v) {
        let x = num(t, "x", 0.0, path, l)?;
        let y = num(t, "y", x, path, l)?;
        let radians = num(t, "degrees", 0.0, path, l)?.to_radians();
        let a = match text(t, "$type", "").rsplit('.').next().unwrap_or("") {
            "translate" => Affine::translate((x, num(t, "y", 0.0, path, l)?)),
            "scale" => Affine::scale_non_uniform(x, y),
            "rotate" => {
                let c = (num(t, "cx", 0.0, path, l)?, num(t, "cy", 0.0, path, l)?);
                Affine::translate(c) * Affine::rotate(radians) * Affine::translate((-c.0, -c.1))
            }
            "skewX" => Affine::new([1.0, 0.0, radians.tan(), 1.0, 0.0, 0.0]),
            "skewY" => Affine::new([1.0, radians.tan(), 0.0, 1.0, 0.0, 0.0]),
            "matrix" => Affine::new([
                num(t, "a", 1.0, path, l)?,
                num(t, "b", 0.0, path, l)?,
                num(t, "c", 0.0, path, l)?,
                num(t, "d", 1.0, path, l)?,
                num(t, "e", 0.0, path, l)?,
                num(t, "f", 0.0, path, l)?,
            ]),
            _ => return Err(error(path, "unsupported transform")),
        };
        result *= a;
    }
    check_affine(result, path, l)?;
    Ok(result)
}
fn check_affine(a: Affine, path: &str, l: &Limits) -> Result<()> {
    for x in a.as_coeffs() {
        numeric(&Value::from(x), path, l.coordinate)
            .map_err(|_| error(path, "transform outside host/backend range"))?;
    }
    Ok(())
}
fn bounds(p: &BezPath) -> Bounds {
    if p.segments().next().is_none() {
        Bounds::empty()
    } else {
        Bounds {
            rect: Some(p.bounding_box()),
            conservative: false,
        }
    }
}
fn stroke_bounds(p: &BezPath, cap: &str) -> Bounds {
    let mut result = bounds(p);
    if cap == "round" || cap == "square" {
        for pair in p.elements().windows(2) {
            if let [PathEl::MoveTo(point), PathEl::ClosePath] = pair {
                result = union(
                    result,
                    Bounds {
                        rect: Some(Rect::from_points(*point, *point)),
                        conservative: false,
                    },
                );
            }
        }
    }
    result
}

fn union(a: Bounds, b: Bounds) -> Bounds {
    Bounds {
        rect: match (a.rect, b.rect) {
            (Some(a), Some(b)) => Some(a.union(b)),
            (a, b) => a.or(b),
        },
        conservative: a.conservative || b.conservative,
    }
}
fn map_bounds(b: Bounds, a: Affine, path: &str, l: &Limits) -> Result<Bounds> {
    let rect = b.rect.map(|r| a.transform_rect_bbox(r));
    if let Some(r) = rect {
        for n in [r.x0, r.y0, r.x1, r.y1] {
            numeric(&Value::from(n), path, l.coordinate)?;
        }
    }
    Ok(Bounds {
        rect,
        conservative: b.conservative || a.as_coeffs()[1] != 0.0 || a.as_coeffs()[2] != 0.0,
    })
}
#[derive(Clone, Copy)]
struct NodeScope {
    world: Affine,
    depth: usize,
    painting: bool,
}
struct Prepare<'a> {
    resolver: &'a dyn DeclarationResolver,
    text: &'a mut dyn TextService,
    config: &'a Config,
    nodes: usize,
    commands: usize,
    gradient_stops: usize,
    layers: f64,
}
pub fn prepare(
    value: &Value,
    resolver: &dyn DeclarationResolver,
    text: &mut dyn TextService,
    config: Config,
) -> Result<Scene> {
    if config.limits.gradient_stops == 0 {
        return Err(error(
            "",
            "invalid host limits: gradient_stops must be positive",
        ));
    }
    if !config.limits.coordinate.is_finite()
        || config.limits.coordinate <= 0.0
        || config.limits.coordinate > 1e7
    {
        return Err(error(
            "",
            "invalid host limits: coordinate must be finite, positive and at most 10000000",
        ));
    }
    for (field, value) in [
        ("surface_pixels", config.limits.surface_pixels),
        ("layer_pixels", config.limits.layer_pixels),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(error(
                "",
                format!("invalid host limits: {field} must be finite and positive"),
            ));
        }
    }
    for n in [config.width, config.height, config.scale] {
        if !n.is_finite() || n <= 0.0 {
            return Err(error("", "invalid viewport or scale"));
        }
        numeric(&Value::from(n), "", config.limits.coordinate).map_err(|_| {
            error(
                "",
                "viewport dimensions or scale exceed the coordinate limit",
            )
        })?;
    }
    if !config.text_ink_padding.is_finite()
        || config.text_ink_padding < 0.0
        || config.text_ink_padding > 10.0
    {
        return Err(error(
            "",
            "text_ink_padding must be in 0..10 physical pixels",
        ));
    }
    if (config.width * config.scale).ceil() * (config.height * config.scale).ceil()
        > config.limits.surface_pixels
    {
        return Err(error("", "surface limit exceeded"));
    }
    let mut p = Prepare {
        resolver,
        text,
        config: &config,
        nodes: 0,
        commands: 0,
        gradient_stops: 0,
        layers: 0.0,
    };
    let root = p
        .node(
            value,
            "",
            None,
            0,
            NodeScope {
                world: Affine::IDENTITY,
                depth: 0,
                painting: true,
            },
        )
        .map_err(|mut e| {
            e.provenance =
                resolver.provenance(e.element.strip_prefix("element").unwrap_or(&e.element));
            if !e.element.starts_with("element") {
                e.element = format!("element{}", e.element);
            }
            e
        })?;
    let revision = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(PROFILE, value, &config, &root))
                .map_err(|e| error("", e.to_string()))?
        )
    );
    Ok(Scene {
        revision,
        profile: PROFILE.into(),
        config,
        root,
    })
}
impl Prepare<'_> {
    fn node(
        &mut self,
        v: &Value,
        path: &str,
        parent: Option<String>,
        order: usize,
        scope: NodeScope,
    ) -> Result<Node> {
        let NodeScope {
            world,
            depth,
            painting,
        } = scope;
        let l = &self.config.limits;
        self.nodes += 1;
        if self.nodes > l.nodes || depth > l.depth {
            return Err(error(path, "node or depth limit exceeded"));
        }
        let identity = self
            .resolver
            .resolve(path)
            .ok_or_else(|| error(path, "missing linked declaration identity"))?;
        if identity.module != MODULE || identity.version != self.config.graphics_version {
            return Err(error(path, "unrecognized graphics declaration identity"));
        }
        let kind = identity.declaration.as_str();
        let required: &[&str] = match kind {
            "Rectangle" => &["width", "height"],
            "Circle" => &["cx", "cy", "r"],
            "Ellipse" => &["cx", "cy", "rx", "ry"],
            "Line" => &["x1", "y1", "x2", "y2"],
            "Path" => &["data"],
            "Text" => &["fontFamily", "text"],
            _ => &[],
        };
        for key in required {
            if v.get(*key).is_none_or(absent) {
                return Err(error(path, format!("missing required property {key}")));
            }
        }

        let allowed = [
            "$type",
            "opacity",
            "transform",
            "clipPath",
            "clipRule",
            "accessibility",
            "fill",
            "stroke",
            "fillRule",
            "viewBox",
            "fit",
            "contentAlignment",
            "background",
            "children",
            "x",
            "y",
            "width",
            "height",
            "rx",
            "ry",
            "cx",
            "cy",
            "r",
            "x1",
            "y1",
            "x2",
            "y2",
            "points",
            "data",
            "text",
            "fontFamily",
            "fontSize",
            "fontWeight",
            "fontStyle",
            "textAnchor",
        ];
        if let Some(object) = v.as_object() {
            for key in object.keys() {
                if !allowed.contains(&key.as_str()) {
                    return Err(error(path, format!("unsupported property {key}")));
                }
            }
        } else {
            return Err(error(path, "expected graphics record"));
        }
        if ![
            "Drawing",
            "Group",
            "Rectangle",
            "Circle",
            "Ellipse",
            "Line",
            "Polyline",
            "Polygon",
            "Path",
            "Text",
        ]
        .contains(&kind)
        {
            return Err(error(path, "unsupported graphics declaration"));
        }
        if (depth == 0) != (kind == "Drawing") {
            return Err(error(path, "Drawing must be the sole root"));
        }
        for key in [
            "shadows",
            "image",
            "letterSpacing",
            "dominantBaseline",
            "lineHeight",
        ] {
            if v.get(key).is_some_and(|x| !absent(x)) {
                return Err(error(path, format!("unsupported feature {key}")));
            }
        }
        let id = format!("element{path}");
        let mut transform = transforms(&v["transform"], path, l)?;
        let mut clip = v
            .get("clipPath")
            .filter(|x| !absent(x))
            .map(|x| {
                svg(
                    x.as_str().unwrap_or("invalid"),
                    path,
                    l,
                    l.path_commands.saturating_sub(self.commands),
                )
            })
            .transpose()?;
        let clip_evenodd = rule(v, "clipRule", path)?;
        let opacity = num(v, "opacity", 1.0, path, l)?;
        if !(0.0..=1.0).contains(&opacity) {
            return Err(error(path, "opacity outside 0..1"));
        }
        let painting = painting && opacity > 0.0;
        let layer_area = if painting && opacity < 1.0 {
            (self.config.width * self.config.scale).ceil()
                * (self.config.height * self.config.scale).ceil()
        } else {
            0.0
        };
        self.layers += layer_area;
        if self.layers > l.layer_pixels {
            return Err(error(path, "concurrent layer area limit exceeded"));
        }
        let mut geometry = None;
        let mut text_run = None;
        let n = |k, d| num(v, k, d, path, l);
        let nonnegative = |k, d| -> Result<f64> {
            let x = n(k, d)?;
            if x < 0.0 {
                Err(error(path, format!("negative {k}")))
            } else {
                Ok(x)
            }
        };
        let mut fill = paint::prepare(
            v.get("fill"),
            if ["Group", "Drawing", "Line", "Polyline"].contains(&kind) {
                None
            } else {
                Some(Paint::Color {
                    color: [0.0, 0.0, 0.0, 1.0],
                })
            },
            path,
            "fill",
            l,
            &mut self.gradient_stops,
        )?;
        match kind {
            "Drawing" => {
                let vb = &v["viewBox"];
                let x = num(vb, "x", 0.0, path, l)?;
                let y = num(vb, "y", 0.0, path, l)?;
                let w = num(vb, "width", 0.0, path, l)?;
                let h = num(vb, "height", 0.0, path, l)?;
                if w <= 0.0 || h <= 0.0 {
                    return Err(error(path, "viewBox dimensions must be positive"));
                }
                let fit = choice(v, "fit", "contain", &["contain", "cover", "fill"], path)?;
                let sx = self.config.width / w;
                let sy = self.config.height / h;
                let (sx, sy) = match fit.as_str() {
                    "contain" => (sx.min(sy), sx.min(sy)),
                    "cover" => (sx.max(sy), sx.max(sy)),
                    _ => (sx, sy),
                };
                let alignment = &v["contentAlignment"];
                let anchor = |k| -> Result<f64> {
                    Ok(
                        match choice(alignment, k, "center", &["start", "center", "end"], path)?
                            .as_str()
                        {
                            "start" => 0.0,
                            "end" => 1.0,
                            _ => 0.5,
                        },
                    )
                };
                transform = Affine::translate((
                    (self.config.width - w * sx) * anchor("x")?,
                    (self.config.height - h * sy) * anchor("y")?,
                )) * Affine::scale_non_uniform(sx, sy)
                    * Affine::translate((-x, -y));
                // Root viewport clipping is in viewBox-local coordinates; no inverse is required for local transforms.
                let viewport = Rect::new(
                    x - (self.config.width - w * sx) * anchor("x")? / sx,
                    y - (self.config.height - h * sy) * anchor("y")? / sy,
                    x + (self.config.width - (self.config.width - w * sx) * anchor("x")?) / sx,
                    y + (self.config.height - (self.config.height - h * sy) * anchor("y")?) / sy,
                );
                clip = Some(viewport.to_path(0.1));
                fill = paint::prepare(
                    v.get("background"),
                    None,
                    path,
                    "background",
                    l,
                    &mut self.gradient_stops,
                )?;
                if fill.is_some() {
                    geometry = Some(viewport.to_path(0.1));
                }
            }
            "Group" => {}
            "Rectangle" => {
                let x = n("x", 0.0)?;
                let y = n("y", 0.0)?;
                let w = nonnegative("width", 0.0)?;
                let h = nonnegative("height", 0.0)?;
                let specified_rx = nonnegative("rx", 0.0)?;
                let specified_ry = nonnegative("ry", specified_rx)?;
                let rx = specified_rx.min(w / 2.0);
                let ry = specified_ry.min(h / 2.0);
                geometry = Some(rounded_rect(x, y, w, h, rx, ry));
            }
            "Circle" | "Ellipse" => {
                let cx = n("cx", 0.0)?;
                let cy = n("cy", 0.0)?;
                let rx = nonnegative(if kind == "Circle" { "r" } else { "rx" }, 0.0)?;
                let ry = if kind == "Circle" {
                    rx
                } else {
                    nonnegative("ry", 0.0)?
                };
                geometry = Some(if rx == 0.0 || ry == 0.0 {
                    BezPath::new()
                } else {
                    kurbo::Ellipse::new((cx, cy), (rx, ry), 0.0).to_path(1e-6 * rx.max(ry))
                });
            }
            "Line" => {
                let mut p = BezPath::new();
                p.move_to((n("x1", 0.0)?, n("y1", 0.0)?));
                p.line_to((n("x2", 0.0)?, n("y2", 0.0)?));
                geometry = Some(p);
                fill = None;
            }
            "Polyline" | "Polygon" => {
                let points = list(&v["points"]);
                if points.len() < if kind == "Polygon" { 3 } else { 2 } {
                    return Err(error(path, "too few points"));
                }
                let required = points.len() + usize::from(kind == "Polygon");
                if required
                    > l.path_commands.saturating_sub(
                        self.commands + clip.as_ref().map_or(0, |p| p.elements().len()),
                    )
                {
                    return Err(error(path, "scene command limit exceeded"));
                }
                let mut p = BezPath::new();
                for (i, point) in points.iter().enumerate() {
                    let pnt = Point::new(
                        num(point, "x", 0.0, path, l)?,
                        num(point, "y", 0.0, path, l)?,
                    );
                    if i == 0 {
                        p.move_to(pnt)
                    } else {
                        p.line_to(pnt)
                    }
                }
                if kind == "Polygon" {
                    p.close_path()
                }
                geometry = Some(p);
            }
            "Path" => {
                geometry = Some(svg(
                    text(v, "data", ""),
                    path,
                    l,
                    l.path_commands.saturating_sub(
                        self.commands + clip.as_ref().map_or(0, |p| p.elements().len()),
                    ),
                )?);
            }
            "Text" => {
                if v.get("stroke").is_some_and(|x| !absent(x)) {
                    return Err(error(path, "text stroke is unsupported"));
                }
                let content = text(v, "text", "");
                let size = n("fontSize", 16.0)?;
                let weight = n("fontWeight", 400.0)?;
                if weight.fract() != 0.0 || !(1.0..=1000.0).contains(&weight) {
                    return Err(error(path, "invalid font weight"));
                }
                let request = TextRequest {
                    text: content.into(),
                    family: text(v, "fontFamily", "").into(),
                    size,
                    weight: weight as u16,
                    italic: choice(v, "fontStyle", "normal", &["normal", "italic"], path)?
                        == "italic",
                };
                validate_text_request(&request, path)?;
                if let Some(paint) = &fill {
                    // Shared acceptance limit also covers Canvas reference-font coordinates.
                    paint.validate_span((size / 1000.0).max(1.0), path, "fill")?;
                }
                let run = self.text.shape(&request).map_err(|mut e| {
                    e.element = path.into();
                    e
                })?;
                let anchor = choice(v, "textAnchor", "start", &["start", "middle", "end"], path)?;
                let x = n("x", 0.0)?
                    - run.advance
                        * match anchor.as_str() {
                            "middle" => 0.5,
                            "end" => 1.0,
                            _ => 0.0,
                        };
                let y = n("y", 0.0)?;
                if let Some(paint) = &fill {
                    paint.validate_text_span(x, y, size / 1000.0, path, "fill")?;
                }
                text_run = Some(Text { request, run, x, y });
            }
            _ => return Err(error(path, "unsupported primitive")),
        }
        check_affine(transform, path, l)?;
        let world = world * transform;
        check_affine(world, path, l)?;
        let stroke = if let Some(s) = v.get("stroke").filter(|s| !absent(s)) {
            let width = num(s, "width", 1.0, path, l)?;
            let miter = num(s, "miterLimit", 4.0, path, l)?;
            if width < 0.0 || miter < 1.0 {
                return Err(error(path, "invalid stroke width or miter limit"));
            }
            let mut dashes = Vec::new();
            for d in list(&s["dashArray"]) {
                let x = numeric(d, path, l.coordinate)?;
                if x < 0.0 {
                    return Err(error(path, "negative dash"));
                }
                dashes.push(x)
            }
            // Defensive guard for future profiles; the current coordinate cap prevents overflow
            // for any practical payload, but a destination must never receive infinite totals.
            let total: f64 = dashes.iter().sum();
            if !total.is_finite() || !(total as f32).is_finite() {
                return Err(error(path, "dash pattern total outside backend range"));
            }
            if dashes.iter().all(|x| *x == 0.0) {
                dashes.clear()
            } else if dashes.len() % 2 == 1 {
                dashes.extend(dashes.clone())
            }
            paint::prepare(
                s.get("paint"),
                None,
                path,
                "stroke.paint",
                l,
                &mut self.gradient_stops,
            )?
            .filter(|_| width > 0.0)
            .map(|paint| -> Result<Stroke> {
                Ok(Stroke {
                    paint,
                    width,
                    cap: choice(s, "lineCap", "butt", &["butt", "round", "square"], path)?,
                    join: choice(s, "lineJoin", "miter", &["miter", "round", "bevel"], path)?,
                    miter,
                    dashes,
                    offset: num(s, "dashOffset", 0.0, path, l)?,
                })
            })
            .transpose()?
        } else {
            None
        };
        let local = if let Some(t) = &text_run {
            map_bounds(t.run.ink, Affine::translate((t.x, t.y)), path, l)?
        } else {
            geometry.as_ref().map(bounds).unwrap_or_else(Bounds::empty)
        };
        if let Some(p) = &geometry {
            self.commands += p.elements().len();
        }
        if let Some(p) = &clip {
            self.commands += p.elements().len();
        }
        if self.commands > l.path_commands {
            return Err(error(path, "scene command limit exceeded"));
        }
        let mut painted = if fill.as_ref().is_some_and(Paint::visible) {
            local
        } else {
            Bounds::empty()
        };
        if let Some(s) = stroke.as_ref().filter(|s| s.paint.visible()) {
            let local = geometry
                .as_ref()
                .map(|p| stroke_bounds(p, &s.cap))
                .unwrap_or(local);
            painted = union(
                painted,
                Bounds {
                    rect: local.rect.map(|r| {
                        r.inflate(
                            s.width
                                * s.miter.max(if s.cap == "square" {
                                    std::f64::consts::SQRT_2
                                } else {
                                    1.0
                                })
                                / 2.0,
                            s.width
                                * s.miter.max(if s.cap == "square" {
                                    std::f64::consts::SQRT_2
                                } else {
                                    1.0
                                })
                                / 2.0,
                        )
                    }),
                    conservative: true,
                },
            )
        }
        if fill.as_ref().is_some_and(Paint::has_transparency)
            || stroke.as_ref().is_some_and(|s| s.paint.has_transparency())
        {
            painted.conservative = true;
        }
        painted = map_bounds(painted, world, path, l)?;
        if text_run.is_some() && painted.rect.is_some() && self.config.text_ink_padding > 0.0 {
            let pad = self.config.text_ink_padding / self.config.scale;
            painted.rect = painted.rect.map(|r| r.inflate(pad, pad));
            painted.conservative = true;
            painted = map_bounds(painted, Affine::IDENTITY, path, l)?;
        }
        let mut children = Vec::new();
        if kind == "Drawing" || kind == "Group" {
            for (index, child) in list(&v["children"]).iter().enumerate() {
                let child_path = if v["children"].is_array() {
                    format!("{path}/children/{index}")
                } else {
                    format!("{path}/children")
                };
                let child = self.node(
                    child,
                    &child_path,
                    Some(id.clone()),
                    index,
                    NodeScope {
                        world,
                        depth: depth + 1,
                        painting,
                    },
                )?;
                let mut extent = if child.opacity == 0.0 {
                    Bounds::empty()
                } else {
                    child.painted
                };
                if let Some(clip) = &child.clip {
                    let child_world = world * child.transform;
                    extent = match bounds(clip).rect {
                        Some(rect) => extent.intersect(child_world.transform_rect_bbox(rect)),
                        None => Bounds::empty(),
                    };
                    if axis_rect(clip)
                        && child_world.as_coeffs()[1] == 0.0
                        && child_world.as_coeffs()[2] == 0.0
                        && child.stroke.is_none()
                        && child.path.as_ref().is_some_and(axis_rect)
                    {
                        extent.conservative = child.painted.conservative;
                    }
                    if !axis_rect(clip)
                        || child_world.as_coeffs()[1] != 0.0
                        || child_world.as_coeffs()[2] != 0.0
                    {
                        extent.conservative = extent.rect.is_some();
                    }
                }
                painted = union(painted, extent);
                children.push(child);
            }
        } else if v.get("children").is_some_and(|v| !absent(v)) {
            return Err(error(path, "primitive cannot have children"));
        }
        let axis_aligned = world.as_coeffs()[1] == 0.0 && world.as_coeffs()[2] == 0.0;
        let rectangular_content = axis_aligned
            && stroke.is_none()
            && fill.is_some()
            && geometry.as_ref().is_some_and(axis_rect);
        let conservative_clip = clip
            .as_ref()
            .is_some_and(|p| !axis_aligned || !axis_rect(p));
        let mut visible = painted;
        if let Some(p) = &clip {
            visible = match bounds(p).rect {
                Some(r) => visible.intersect(world.transform_rect_bbox(r)),
                None => Bounds::empty(),
            };
        }
        visible = visible.intersect(Rect::new(0.0, 0.0, self.config.width, self.config.height));
        if rectangular_content && !conservative_clip {
            visible.conservative = painted.conservative;
        }
        if conservative_clip && visible.rect.is_some() {
            visible.conservative = true;
        }
        if opacity == 0.0 {
            visible = Bounds::empty()
        }
        if let Some(r) = visible.rect {
            for c in &mut children {
                clip_descendants(c, r, conservative_clip);
            }
        } else {
            for c in &mut children {
                empty_descendants(c);
            }
        }
        self.layers -= layer_area;
        Ok(Node {
            id,
            parent,
            order,
            identity,
            provenance: self.resolver.provenance(path),
            accessibility: v.get("accessibility").cloned(),
            transform,
            clip,
            clip_evenodd,
            opacity,
            path: geometry,
            evenodd: rule(v, "fillRule", path)?,
            fill,
            stroke,
            text: text_run,
            geometry: local,
            painted,
            visible,
            children,
        })
    }
}
fn clip_descendants(n: &mut Node, r: Rect, conservative: bool) {
    n.visible = n.visible.intersect(r);
    if conservative && n.visible.rect.is_some() {
        n.visible.conservative = true;
    }
    for c in &mut n.children {
        clip_descendants(c, r, conservative)
    }
}
fn empty_descendants(n: &mut Node) {
    n.visible = Bounds::empty();
    for c in &mut n.children {
        empty_descendants(c)
    }
}
fn rounded_rect(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> BezPath {
    if w == 0.0 || h == 0.0 {
        return BezPath::new();
    }
    if rx == 0.0 || ry == 0.0 {
        return Rect::new(x, y, x + w, y + h).to_path(0.1);
    }
    let mut p = BezPath::new();
    let k = 0.5522847498307936;
    p.move_to((x + rx, y));
    p.line_to((x + w - rx, y));
    p.curve_to(
        (x + w - rx + k * rx, y),
        (x + w, y + ry - k * ry),
        (x + w, y + ry),
    );
    p.line_to((x + w, y + h - ry));
    p.curve_to(
        (x + w, y + h - ry + k * ry),
        (x + w - rx + k * rx, y + h),
        (x + w - rx, y + h),
    );
    p.line_to((x + rx, y + h));
    p.curve_to(
        (x + rx - k * rx, y + h),
        (x, y + h - ry + k * ry),
        (x, y + h - ry),
    );
    p.line_to((x, y + ry));
    p.curve_to((x, y + ry - k * ry), (x + rx - k * rx, y), (x + rx, y));
    p.close_path();
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Resolver {
        spoof: bool,
    }
    impl DeclarationResolver for Resolver {
        fn resolve(&self, path: &str) -> Option<Identity> {
            Some(Identity {
                module: if self.spoof { "main.nx" } else { MODULE }.into(),
                version: "test".into(),
                declaration: if path.is_empty() { "Drawing" } else { "Path" }.into(),
            })
        }
    }
    struct NoText;
    impl TextService for NoText {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            Err(error("", "no text host"))
        }
    }
    fn config() -> Config {
        Config {
            width: 100.0,
            height: 100.0,
            graphics_version: "test".into(),
            ..Config::default()
        }
    }
    fn drawing(data: &str) -> Value {
        json!({"viewBox":{"width":100,"height":100},"children":[{"data":data,"fill":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}}]})
    }
    fn gradient() -> Value {
        json!({"$type":"Paint.linearGradient","x1":0,"y1":0,"x2":100,"y2":0,"stops":[
            {"offset":0,"color":{"r":1,"g":0,"b":0,"a":1}},
            {"offset":1,"color":{"r":0,"g":0,"b":1,"a":0}}
        ]})
    }
    #[test]
    fn gradients_validate_budget_visibility_and_roundtrip() {
        let mut value = drawing("M0 0H100V100H0Z");
        value["children"][0]["fill"] = gradient();
        let scene = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert!(scene.root.children[0].painted.conservative);
        let decoded: Scene = serde_json::from_value(serde_json::to_value(&scene).unwrap()).unwrap();
        assert_eq!(decoded.root.children[0].fill, scene.root.children[0].fill);
        let mut c = config();
        c.limits.gradient_stops = 3;
        let child = value["children"][0].clone();
        value["children"].as_array_mut().unwrap().push(child);
        assert!(prepare(&value, &Resolver { spoof: false }, &mut NoText, c)
            .unwrap_err()
            .message
            .contains("gradient_stops"));
        value["children"].as_array_mut().unwrap().pop();
        value["children"][0]["fill"]["stops"][0]["color"]["a"] = json!(0);
        let scene = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert!(scene.root.children[0].painted.rect.is_none());
        assert!(scene.root.children[0].fill.is_some());
        let transparent = value["children"][0]["fill"].take();
        value["children"][0]["fill"] = json!("none");
        value["children"][0]["stroke"] = json!({"paint":transparent,"width":10});
        let scene = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert!(scene.root.children[0].painted.rect.is_none());
    }
    #[test]
    fn gradient_stop_channels_offsets_and_radial_ranges_are_checked() {
        for (pointer, bad) in [
            ("/stops/0/offset", json!(-0.1)),
            ("/stops/1/offset", json!(1.1)),
            ("/stops/0/color/r", json!(2.0)),
            ("/stops/1/color/a", json!(-1.0)),
            (
                "/stops",
                json!([{ "offset":0,"color":{"r":1,"g":0,"b":0} }]),
            ),
        ] {
            let mut value = drawing("M0 0H10V10Z");
            let mut paint = gradient();
            *paint.pointer_mut(pointer).unwrap() = bad;
            value["children"][0]["fill"] = paint;
            value["children"][0]["opacity"] = json!(0);
            assert!(prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).is_err());
        }
        for radius in [0.0, -1.0, 1e20] {
            let mut value = drawing("M0 0H10V10Z");
            let mut paint = gradient();
            paint["$type"] = json!("Paint.radialGradient");
            paint["cx"] = json!(0);
            paint["cy"] = json!(0);
            paint["radius"] = json!(radius);
            value["children"][0]["fill"] = paint;
            assert!(prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).is_err());
        }
        let mut c = config();
        c.limits.gradient_stops = 0;
        assert!(
            prepare(&drawing("M0 0"), &Resolver { spoof: false }, &mut NoText, c)
                .unwrap_err()
                .message
                .contains("gradient_stops")
        );
    }
    #[test]
    fn invalid_gradients_are_rejected_even_when_hidden() {
        for (key, bad) in [
            ("x2", json!(0)),
            ("y1", json!(1e20)),
            ("stops", json!([])),
            (
                "stops",
                json!([
                    {"offset":0.8,"color":{"r":1,"g":0,"b":0}},
                    {"offset":0.2,"color":{"r":0,"g":0,"b":1}}
                ]),
            ),
        ] {
            let mut value = drawing("M0 0H10V10Z");
            value["children"][0]["opacity"] = json!(0);
            value["children"][0]["fill"] = gradient();
            value["children"][0]["fill"][key] = bad;
            let e = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap_err();
            assert!(e.element.contains("children/0"));
            assert!(e.message.contains("fill"));
        }
    }
    #[test]
    fn shared_colors_preserve_float32_channels_and_reject_invalid_late_elements() {
        let color = nx_value::color::make([0.123456789, 0.4, 0.8, 0.5]).unwrap();
        let mut value = drawing("M0 0H10V10Z");
        value["children"][0]["fill"]["color"] = serde_json::to_value(color).unwrap();
        let scene = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert_eq!(
            scene.root.children[0].fill,
            Some(Paint::Color {
                color: [0.123_456_79_f32, 0.4, 0.8, 0.5]
            })
        );
        let valid = value["children"][0].clone();
        value["children"].as_array_mut().unwrap().push(valid);
        for invalid in [
            json!("red"),
            json!({"r": 2.0, "g": 0.0, "b": 0.0, "a": 1.0}),
        ] {
            value["children"][1]["fill"]["color"] = invalid;
            let failure =
                prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap_err();
            assert_eq!(failure.code, "color-invalid-channel");
            assert!(failure.element.contains("children/1"));
        }
    }
    #[test]
    fn raw_color_records_use_the_default_alpha() {
        let mut value = drawing("M0 0H10V10Z");
        value["children"][0]["fill"]["color"] = json!({"r":1.0,"g":0.0,"b":0.0});
        let scene = prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert_eq!(
            scene.root.children[0].fill,
            Some(Paint::Color {
                color: [1.0, 0.0, 0.0, 1.0]
            })
        );
    }
    #[test]
    fn identity_and_late_path_validation() {
        assert!(prepare(
            &drawing("M0 0L10 10"),
            &Resolver { spoof: true },
            &mut NoText,
            config()
        )
        .is_err());
        for malformed in [
            "M0 0L10 10 garbage",
            "M0 0 A5 5 0 2 0 10 10",
            "M0 0 L",
            "M NaN 0",
        ] {
            assert!(
                prepare(
                    &drawing(malformed),
                    &Resolver { spoof: false },
                    &mut NoText,
                    config()
                )
                .is_err(),
                "{malformed}"
            );
        }
    }
    #[test]
    fn raw_viewports_and_geometry_children_retain_defensive_validation() {
        for dimension in ["width", "height"] {
            let mut value = drawing("");
            value["viewBox"][dimension] = json!(-1);
            let diagnostic =
                prepare(&value, &Resolver { spoof: false }, &mut NoText, config()).unwrap_err();
            assert!(diagnostic
                .message
                .contains("viewBox dimensions must be positive"));
        }
        struct GeometryChild;
        impl DeclarationResolver for GeometryChild {
            fn resolve(&self, path: &str) -> Option<Identity> {
                Some(Identity {
                    module: if path.is_empty() {
                        MODULE
                    } else {
                        "@nx/geometry/geometry.nx"
                    }
                    .into(),
                    version: "test".into(),
                    declaration: if path.is_empty() { "Drawing" } else { "Rect" }.into(),
                })
            }
        }
        let value = json!({"viewBox":{"width":100,"height":100},
            "children":[{"$type":"Rect","x":0,"y":0,"width":10,"height":10}]});
        let diagnostic = prepare(&value, &GeometryChild, &mut NoText, config()).unwrap_err();
        assert!(diagnostic
            .message
            .contains("unrecognized graphics declaration identity"));
    }

    #[test]
    fn raw_non_finite_fields_fail_before_json_conversion() {
        use nx_value::NxValue;
        for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            for field in ["width", "height"] {
                let mut value: NxValue = serde_json::from_value(drawing("")).unwrap();
                let NxValue::Record { properties, .. } = &mut value else {
                    panic!("drawing")
                };
                let NxValue::Record { properties, .. } = properties.get_mut("viewBox").unwrap()
                else {
                    panic!("viewBox")
                };
                properties.insert(field.into(), NxValue::Float(bad));
                let diagnostic =
                    prepare_value(&value, &Resolver { spoof: false }, &mut NoText, config())
                        .unwrap_err();
                assert_eq!(diagnostic.code, "graphics-non-finite");
                assert_eq!(diagnostic.element, format!("element/viewBox/{field}"));
            }
        }
    }

    #[test]
    fn fractional_font_weight_is_rejected_before_text_shaping() {
        struct TextResolver;
        impl DeclarationResolver for TextResolver {
            fn resolve(&self, path: &str) -> Option<Identity> {
                Some(Identity {
                    module: MODULE.into(),
                    version: "test".into(),
                    declaration: if path.is_empty() { "Drawing" } else { "Text" }.into(),
                })
            }
        }
        let value = json!({"viewBox":{"width":100,"height":100},
            "children":[{"fontFamily":"test","text":"hello","fontWeight":400.5}]});
        let diagnostic = prepare(&value, &TextResolver, &mut NoText, config()).unwrap_err();
        assert_eq!(diagnostic.message, "invalid font weight");
    }
    #[test]
    fn arcs_and_closed_subpaths() {
        let scene = prepare(
            &drawing("M10 50 A40 40 0 1 1 90 50 A40 40 0 1 1 10 50 Z M40 50L60 50"),
            &Resolver { spoof: false },
            &mut NoText,
            config(),
        )
        .unwrap();
        let elements = scene.root.children[0].path.as_ref().unwrap().elements();
        assert!(elements.iter().any(|e| matches!(e, PathEl::CurveTo(..))));
        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, PathEl::ClosePath))
                .count(),
            1
        );
    }
    #[test]
    fn transform_order_fit_and_stale_revision() {
        let mut v = drawing("M1 0 L2 2");
        v["children"][0]["transform"] = json!([{"$type":"Transform.translate","x":10,"y":0},{"$type":"Transform.scale","x":2,"y":2}]);
        let scene = prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert_eq!(
            scene.root.children[0].transform * Point::new(1.0, 0.0),
            Point::new(12.0, 0.0)
        );
        let repeated = prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        assert_eq!(scene.revision, repeated.revision);
        assert!(scene
            .inspect("previous", &scene.root.children[0].id)
            .is_err());
        for (fit, sx, sy) in [
            ("contain", 1.0, 1.0),
            ("cover", 2.0, 2.0),
            ("fill", 2.0, 1.0),
        ] {
            let mut v = drawing("");
            v["viewBox"]["width"] = json!(50);
            v["fit"] = json!(fit);
            let scene = prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
            assert_eq!(scene.root.transform.as_coeffs()[0], sx);
            assert_eq!(scene.root.transform.as_coeffs()[3], sy);
        }
    }
    #[test]
    fn limits_numbers_strokes_and_clips() {
        let mut v = drawing("M10 10L20 20");
        v["children"][0]["stroke"] = json!({"paint":{"$type":"Paint.color","color":{"r": 0.0, "g": 0.0, "b": 1.0, "a": 1.0}},"width":4,"dashArray":[1,2,3]});
        v["children"][0]["clipPath"] = json!("M0 0H5V5H0Z");
        let scene = prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).unwrap();
        let child = &scene.root.children[0];
        assert_eq!(child.stroke.as_ref().unwrap().dashes.len(), 6);
        assert!(child.painted.conservative);
        assert!(child.visible.rect.is_some());
        v["children"][0]["opacity"] = json!(2);
        assert!(prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).is_err());
        v["children"][0]["opacity"] = json!(1);
        let mut c = config();
        c.limits.nodes = 1;
        assert!(prepare(&v, &Resolver { spoof: false }, &mut NoText, c).is_err());
        v["children"][0]["transform"] = json!([{"$type":"Transform.scale","x":1e100}]);
        assert!(prepare(&v, &Resolver { spoof: false }, &mut NoText, config()).is_err());
    }
    #[derive(Default)]
    struct Mock {
        depth: usize,
        ended: bool,
        fail: bool,
        paths: usize,
    }
    impl Destination for Mock {
        fn begin(&mut self) -> Result<()> {
            Ok(())
        }
        fn end(&mut self) -> Result<()> {
            self.ended = true;
            assert_eq!(self.depth, 0);
            Ok(())
        }
        fn push(&mut self, _: Affine, _: Option<(&BezPath, bool)>, _: f64) -> Result<()> {
            self.depth += 1;
            Ok(())
        }
        fn pop(&mut self) -> Result<()> {
            self.depth -= 1;
            Ok(())
        }
        fn path(
            &mut self,
            _: &BezPath,
            _: bool,
            _: Option<&Paint>,
            _: Option<&Stroke>,
        ) -> Result<()> {
            self.paths += 1;
            if self.fail {
                Err(error("", "injected destination failure"))
            } else {
                Ok(())
            }
        }
        fn text(&mut self, _: &Text, _: &Paint) -> Result<()> {
            Ok(())
        }
    }
    #[test]
    fn direct_scopes_balance_on_failure() {
        let scene = prepare(
            &drawing("M0 0L10 10"),
            &Resolver { spoof: false },
            &mut NoText,
            config(),
        )
        .unwrap();
        for fail in [false, true] {
            let mut mock = Mock {
                fail,
                ..Mock::default()
            };
            assert_eq!(scene.draw(&mut mock).is_err(), fail);
            assert!(mock.ended);
            assert_eq!(mock.paths, 1);
        }
    }

    #[test]
    fn earlier_and_unknown_profiles_fail_before_destination_calls() {
        let mut scene = prepare(
            &drawing("M0 0L10 10"),
            &Resolver { spoof: false },
            &mut NoText,
            config(),
        )
        .unwrap();
        for profile in ["graphics-core-v3", "graphics-core-v999"] {
            scene.profile = profile.into();
            let mut destination = Mock::default();
            let diagnostic = scene.draw(&mut destination).unwrap_err();
            assert_eq!(diagnostic.message, "unsupported graphics profile");
            assert_eq!(destination.paths, 0);
            assert!(!destination.ended);
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use serde_json::json;
    struct Resolver;
    impl DeclarationResolver for Resolver {
        fn resolve(&self, p: &str) -> Option<Identity> {
            Some(Identity {
                module: MODULE.into(),
                version: "v".into(),
                declaration: if p.is_empty() { "Drawing" } else { "Rectangle" }.into(),
            })
        }
    }
    struct NoText;
    impl TextService for NoText {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            Err(error("", "no text"))
        }
    }
    #[test]
    fn preflight_boundaries_and_zero_strokes() {
        let v = json!({"viewBox":{"width":100,"height":100},"children":[{"width":10,"height":10,"fill":"none","stroke":{"paint":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}},"width":0}}]});
        let c = Config {
            graphics_version: "v".into(),
            ..Config::default()
        };
        let scene = prepare(&v, &Resolver, &mut NoText, c.clone()).unwrap();
        assert!(scene.root.children[0].stroke.is_none());
        assert!(scene.root.children[0].painted.rect.is_none());
        for limits in [
            Limits {
                nodes: 1,
                ..Limits::default()
            },
            Limits {
                depth: 0,
                ..Limits::default()
            },
            Limits {
                surface_pixels: 1.0,
                ..Limits::default()
            },
            Limits {
                path_commands: 1,
                ..Limits::default()
            },
        ] {
            let mut c = c.clone();
            c.limits = limits;
            assert!(prepare(&v, &Resolver, &mut NoText, c).is_err());
        }
        for width in [-1.0, 1e100, 1e-100] {
            let mut v = v.clone();
            v["children"][0]["width"] = json!(width);
            assert!(prepare(&v, &Resolver, &mut NoText, c.clone()).is_err());
        }
        for feature in ["shadows", "letterSpacing", "dominantBaseline", "image"] {
            let mut v = v.clone();
            v["children"][0][feature] = json!(1);
            assert!(prepare(&v, &Resolver, &mut NoText, c.clone()).is_err());
        }
        let mut v = v.clone();
        v["children"][0]["clipPath"] = json!("M1000 1000H1010V1010H1000Z");
        assert!(prepare(&v, &Resolver, &mut NoText, c)
            .unwrap()
            .root
            .children[0]
            .visible
            .rect
            .is_none());
    }
}

// Normalize SVG arcs to cubics at graphics construction tolerance instead of kurbo's fixed
// from_svg tolerance (0.1 logical units). Open subpaths remain open for stroke semantics.
#[derive(Debug)]
enum SvgError {
    Syntax(String),
    Limit,
}
impl From<&str> for SvgError {
    fn from(message: &str) -> Self {
        Self::Syntax(message.into())
    }
}
impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(message) => write!(f, "malformed SVG path: {message}"),
            Self::Limit => f.write_str("normalized path command limit exceeded"),
        }
    }
}
fn normalize_svg(data: &str, budget: usize) -> std::result::Result<BezPath, SvgError> {
    use svgtypes::PathSegment::*;
    let mut p = BezPath::new();
    let mut current = Point::ZERO;
    let mut start = Point::ZERO;
    let mut cubic: Option<Point> = None;
    let mut quadratic: Option<Point> = None;
    let mut begun = false;
    for segment in svgtypes::PathParser::from(data) {
        let segment = segment.map_err(|e| SvgError::Syntax(e.to_string()))?;
        if !begun && !matches!(segment, MoveTo { .. }) {
            return Err("path must start with moveto".into());
        }
        let at = |abs: bool, x: f64, y: f64| {
            if abs {
                Point::new(x, y)
            } else {
                current + kurbo::Vec2::new(x, y)
            }
        };
        let mut next_cubic = None;
        let mut next_quadratic = None;
        match segment {
            MoveTo { abs, x, y } => {
                current = at(abs, x, y);
                start = current;
                p.move_to(current);
                begun = true;
            }
            LineTo { abs, x, y } => {
                current = at(abs, x, y);
                p.line_to(current);
            }
            HorizontalLineTo { abs, x } => {
                current = Point::new(if abs { x } else { current.x + x }, current.y);
                p.line_to(current);
            }
            VerticalLineTo { abs, y } => {
                current = Point::new(current.x, if abs { y } else { current.y + y });
                p.line_to(current);
            }
            CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let a = at(abs, x1, y1);
                let b = at(abs, x2, y2);
                let end = at(abs, x, y);
                p.curve_to(a, b, end);
                current = end;
                next_cubic = Some(b);
            }
            SmoothCurveTo { abs, x2, y2, x, y } => {
                let a = cubic.map(|c| current + (current - c)).unwrap_or(current);
                let b = at(abs, x2, y2);
                let end = at(abs, x, y);
                p.curve_to(a, b, end);
                current = end;
                next_cubic = Some(b);
            }
            Quadratic { abs, x1, y1, x, y } => {
                let a = at(abs, x1, y1);
                let end = at(abs, x, y);
                p.quad_to(a, end);
                current = end;
                next_quadratic = Some(a);
            }
            SmoothQuadratic { abs, x, y } => {
                let a = quadratic
                    .map(|c| current + (current - c))
                    .unwrap_or(current);
                let end = at(abs, x, y);
                p.quad_to(a, end);
                current = end;
                next_quadratic = Some(a);
            }
            EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let end = at(abs, x, y);
                if end != current {
                    if rx == 0.0 || ry == 0.0 {
                        p.line_to(end);
                    } else {
                        let unit = rx.abs().min(ry.abs());
                        let input = kurbo::SvgArc {
                            from: Point::ZERO,
                            to: Point::new((end.x - current.x) / unit, (end.y - current.y) / unit),
                            radii: kurbo::Vec2::new(rx.abs() / unit, ry.abs() / unit),
                            x_rotation: x_axis_rotation.to_radians(),
                            large_arc,
                            sweep,
                        };
                        let arc =
                            kurbo::Arc::from_svg_arc(&input).ok_or("invalid nonzero SVG arc")?;
                        if ![
                            arc.center.x,
                            arc.center.y,
                            arc.radii.x,
                            arc.radii.y,
                            arc.start_angle,
                            arc.sweep_angle,
                        ]
                        .iter()
                        .all(|n| n.is_finite())
                        {
                            return Err("SVG arc outside supported numeric range".into());
                        }
                        let tolerance = 1e-6 * arc.radii.x.max(arc.radii.y);
                        let count = arc.append_iter(tolerance).count();
                        if count > budget.saturating_sub(p.elements().len()) {
                            return Err(SvgError::Limit);
                        }
                        let map =
                            |q: Point| Point::new(current.x + q.x * unit, current.y + q.y * unit);
                        arc.to_cubic_beziers(tolerance, |a, b, c| {
                            p.curve_to(map(a), map(b), map(c))
                        });
                    }
                }
                current = end;
            }
            ClosePath { .. } => {
                p.close_path();
                current = start;
            }
        }
        if p.elements().len() > budget {
            return Err(SvgError::Limit);
        }
        cubic = next_cubic;
        quadratic = next_quadratic;
    }
    Ok(p)
}

fn axis_rect(path: &BezPath) -> bool {
    let e = path.elements();
    if e.len() != 5 || !matches!(e[4], PathEl::ClosePath) {
        return false;
    }
    let points = match (e[0], e[1], e[2], e[3]) {
        (PathEl::MoveTo(a), PathEl::LineTo(b), PathEl::LineTo(c), PathEl::LineTo(d)) => {
            [a, b, c, d]
        }
        _ => return false,
    };
    let [a, b, c, d] = points;
    let horizontal =
        a.y == b.y && b.x == c.x && c.y == d.y && d.x == a.x && a.x != b.x && a.y != d.y;
    let vertical = a.x == b.x && b.y == c.y && c.x == d.x && d.y == a.y && a.y != b.y && a.x != d.x;
    horizontal || vertical
}

#[cfg(test)]
mod review_bounds_tests {
    use super::*;
    use serde_json::json;
    struct R;
    impl DeclarationResolver for R {
        fn resolve(&self, p: &str) -> Option<Identity> {
            Some(Identity {
                module: MODULE.into(),
                version: "v".into(),
                declaration: if p.is_empty() { "Drawing" } else { "Rectangle" }.into(),
            })
        }
    }
    struct T;
    impl TextService for T {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            Err(error("", "no text"))
        }
    }
    #[test]
    fn exact_visible_rectangles_and_unclamped_radius_default() {
        let mut v = json!({"viewBox":{"width":100,"height":100},"children":[{"x":10,"y":10,"width":20,"height":20}]});
        let c = Config {
            width: 100.0,
            height: 100.0,
            graphics_version: "v".into(),
            ..Config::default()
        };
        let scene = prepare(&v, &R, &mut T, c.clone()).unwrap();
        assert!(!scene.root.children[0].visible.conservative);
        v["children"][0] = json!({"width":10,"height":100,"rx":20});
        let scene = prepare(&v, &R, &mut T, c).unwrap();
        let endpoint = scene.root.children[0]
            .path
            .as_ref()
            .unwrap()
            .elements()
            .iter()
            .find_map(|e| {
                if let PathEl::CurveTo(_, _, p) = e {
                    Some(*p)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(endpoint, Point::new(10.0, 20.0));
    }
}

/// Prepares a generic runtime value without losing non-finite floats to JSON's null encoding.
/// Declaration identity still comes exclusively from the trusted linked-program resolver.
pub fn prepare_value(
    value: &nx_value::NxValue,
    resolver: &dyn DeclarationResolver,
    text: &mut dyn TextService,
    config: Config,
) -> Result<Scene> {
    fn finite(value: &nx_value::NxValue, path: &str) -> Result<()> {
        use nx_value::NxValue;
        match value {
            NxValue::Float(v) if !v.is_finite() => {
                let mut e = error(path, "non-finite graphics value");
                e.code = "graphics-non-finite".into();
                return Err(e);
            }
            NxValue::Float32(v) if !v.is_finite() => {
                let mut e = error(path, "non-finite graphics value");
                e.code = "graphics-non-finite".into();
                return Err(e);
            }
            NxValue::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    finite(value, &format!("{path}/{index}"))?;
                }
            }
            NxValue::Record { properties, .. } => {
                for (key, value) in properties {
                    finite(
                        value,
                        &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    finite(value, "").map_err(|mut e| {
        let mut path = e.element.strip_prefix("element").unwrap_or(&e.element);
        loop {
            if let Some(source) = resolver.provenance(path) {
                e.provenance = Some(source);
                break;
            }
            if let Some((parent, _)) = path.rsplit_once('/') {
                path = parent;
            } else {
                break;
            }
        }
        e
    })?;
    let json = serde_json::to_value(value).map_err(|e| error("", e.to_string()))?;
    prepare(&json, resolver, text, config)
}

#[cfg(test)]
mod folded_clip_tests {
    use super::*;
    use serde_json::json;
    struct R;
    impl DeclarationResolver for R {
        fn resolve(&self, p: &str) -> Option<Identity> {
            Some(Identity {
                module: MODULE.into(),
                version: "v".into(),
                declaration: if p.is_empty() { "Drawing" } else { "Rectangle" }.into(),
            })
        }
    }
    struct T;
    impl TextService for T {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            Err(error("", "no text"))
        }
    }
    #[test]
    fn folded_and_degenerate_clips_never_claim_exact_nonempty_bounds() {
        for data in [
            "M0 0L20 0L20 20L20 0Z",
            "M0 0L20 0L0 0L0 20Z",
            "M0 0L0 0L0 20L0 20Z",
        ] {
            let path = normalize_svg(data, 100_000).unwrap();
            assert!(!axis_rect(&path));
            let v = json!({"viewBox":{"width":30,"height":30},"children":[{"x":5,"y":5,"width":10,"height":10,"clipPath":data}]});
            let scene = prepare(
                &v,
                &R,
                &mut T,
                Config {
                    graphics_version: "v".into(),
                    ..Config::default()
                },
            )
            .unwrap();
            let visible = scene.root.children[0].visible;
            assert!(visible.rect.is_none() || visible.conservative);
        }
    }
}

#[cfg(test)]
mod layer_budget_tests {
    use super::*;

    struct Resolver {
        groups: usize,
    }
    impl DeclarationResolver for Resolver {
        fn resolve(&self, path: &str) -> Option<Identity> {
            let depth = path.matches("/children/").count();
            Some(Identity {
                module: MODULE.into(),
                version: "test".into(),
                declaration: if depth == 0 {
                    "Drawing"
                } else if depth <= self.groups {
                    "Group"
                } else {
                    "Path"
                }
                .into(),
            })
        }
    }
    fn config() -> Config {
        Config {
            width: 100.0,
            height: 100.0,
            graphics_version: "test".into(),
            limits: Limits {
                layer_pixels: 10_000.0,
                ..Limits::default()
            },
            ..Config::default()
        }
    }
    fn path() -> Value {
        serde_json::json!({"data":"M0 0H5V5Z","opacity":0.5,"fill":{"$type":"Paint.color","color":{"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}})
    }
    struct NoText;
    impl TextService for NoText {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            panic!("unexpected text")
        }
    }
    #[test]
    fn siblings_share_a_layer_budget_but_concurrent_nesting_is_bounded() {
        let value =
            serde_json::json!({"viewBox":{"width":100,"height":100},"children":vec![path();1000]});
        assert!(prepare(&value, &Resolver { groups: 0 }, &mut NoText, config()).is_ok());
        let mut nested = path();
        for _ in 0..4 {
            nested = serde_json::json!({"opacity":0.5,"children":[nested]});
        }
        let mut value =
            serde_json::json!({"viewBox":{"width":100,"height":100},"children":[nested]});
        assert!(
            prepare(&value, &Resolver { groups: 4 }, &mut NoText, config())
                .unwrap_err()
                .message
                .contains("layer")
        );
        value["children"][0]["opacity"] = serde_json::json!(0);
        assert!(prepare(&value, &Resolver { groups: 4 }, &mut NoText, config()).is_ok());
    }
}

#[cfg(test)]
mod full_review_tests {
    use super::*;
    use kurbo::ParamCurve;
    use serde_json::json;

    struct Resolver(Value);
    impl DeclarationResolver for Resolver {
        fn resolve(&self, path: &str) -> Option<Identity> {
            Some(Identity {
                module: MODULE.into(),
                version: "v".into(),
                declaration: self.0.pointer(path)?["$type"].as_str()?.into(),
            })
        }
    }
    struct NoText;
    impl TextService for NoText {
        fn shape(&mut self, _: &TextRequest) -> Result<TextRun> {
            panic!("unexpected text")
        }
    }
    fn scene(children: Value) -> Scene {
        let value =
            json!({"$type":"Drawing", "viewBox":{"width":100,"height":100}, "children":children});
        prepare(
            &value,
            &Resolver(value.clone()),
            &mut NoText,
            Config {
                width: 100.0,
                height: 100.0,
                graphics_version: "v".into(),
                ..Config::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn tiny_arcs_correct_radii_and_keep_their_curvature() {
        for data in [
            "M0 50 A0.000001 0.000001 0 0 1 10 50",
            "M0 0 A0.000008 0.000008 0 0 1 0.000016 0",
        ] {
            let path = normalize_svg(data, 100).unwrap();
            assert!(path
                .elements()
                .iter()
                .any(|el| matches!(el, PathEl::CurveTo(..))));
        }
        let p = normalize_svg("M0 50 A0.000001 0.000001 0 0 1 10 50", 100).unwrap();
        for segment in p.segments() {
            for i in 0..=20 {
                let point = segment.eval(i as f64 / 20.0);
                assert!(((point.x - 5.0).hypot(point.y - 50.0) - 5.0).abs() < 5e-5);
            }
        }
        assert!((p.bounding_box().y0 - 45.0).abs() < 1e-5);
        assert!(normalize_svg("M0 0 A0.000001 10 0 0 1 10 0", 100)
            .unwrap()
            .elements()
            .iter()
            .any(|el| matches!(el, PathEl::CurveTo(..))));
    }
    #[test]
    fn arc_expansion_respects_remaining_budget_before_appending() {
        let data = "M0 0 A0.00002 0.00002 0 0 1 9000000 0";
        assert!(normalize_svg(data, 2)
            .unwrap_err()
            .to_string()
            .contains("command limit"));
        assert!(normalize_svg(data, 20).unwrap().elements().len() <= 20);
        assert!(normalize_svg(
            &(data.to_owned() + " A0.00002 0.00002 0 0 1 -9000000 0"),
            10
        )
        .is_err());
    }
    #[test]
    fn group_bounds_include_only_visible_child_contributions() {
        let s = scene(json!([{"$type":"Group","children":[
            {"$type":"Rectangle","width":50,"height":50,"clipPath":"M0 0H10V10H0Z"},
            {"$type":"Rectangle","x":50,"width":10,"height":10,"opacity":0}
        ]}]));
        let group = &s.root.children[0];
        assert_eq!(group.painted.rect, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert_eq!(group.visible.rect, group.painted.rect);
        assert!(!group.visible.conservative);
    }
    #[test]
    fn closed_moveto_caps_have_bounds_and_zero_radii_do_not_paint() {
        let s = scene(json!([
            {"$type":"Path","data":"M30 50Z","fill":"none","stroke":{"paint":{"$type":"Paint.color","color":{"r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0}},"width":10,"lineCap":"round"}},
            {"$type":"Circle","cx":50,"cy":50,"r":0,"stroke":{"paint":{"$type":"Paint.color","color":{"r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0}},"width":8}},
            {"$type":"Ellipse","cx":50,"cy":50,"rx":0,"ry":30,"stroke":{"paint":{"$type":"Paint.color","color":{"r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0}},"width":8}}
        ]));
        assert!(s.root.children[0]
            .painted
            .rect
            .unwrap()
            .contains(Point::new(30.0, 50.0)));
        for child in &s.root.children[1..] {
            assert_eq!(child.visible.rect, None);
            assert!(child.path.as_ref().unwrap().is_empty());
        }
    }
    #[test]
    fn isolated_closed_movetos_only_contribute_to_cap_strokes() {
        let s = scene(
            json!([{"$type":"Group","children":[{"$type":"Path","data":"M0 0H10V10H0Z M90 90Z"}]}]),
        );
        for node in [&s.root.children[0], &s.root.children[0].children[0]] {
            assert_eq!(node.painted.rect, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
            assert_eq!(node.visible.rect, node.painted.rect);
            assert!(!node.visible.conservative);
        }
        let data = "M0 0 A0.00002 0.00002 0 0 1 9000000 0";
        let error = svg(data, "", &Limits::default(), 2).unwrap_err();
        assert_eq!(error.message, "normalized path command limit exceeded");
    }
    #[test]
    fn svg_reflection_close_relative_flags_and_transform_points() {
        let p = normalize_svg(
            "M10 10l10 0z l0 10 Q10 30 20 20 T30 20 C30 10 40 10 40 20 S50 30 50 20",
            100,
        )
        .unwrap();
        assert_eq!(p.elements()[3], PathEl::LineTo(Point::new(10.0, 20.0)));
        assert_eq!(
            p.elements()[5],
            PathEl::QuadTo(Point::new(30.0, 10.0), Point::new(30.0, 20.0))
        );
        assert_eq!(
            p.elements()[7],
            PathEl::CurveTo(
                Point::new(40.0, 30.0),
                Point::new(50.0, 30.0),
                Point::new(50.0, 20.0)
            )
        );
        let l = Limits::default();
        let rotate = transforms(
            &json!([{"$type":"rotate","degrees":90,"cx":10,"cy":10}]),
            "",
            &l,
        )
        .unwrap();
        assert!((rotate * Point::new(11.0, 10.0) - Point::new(10.0, 11.0)).hypot() < 1e-10);
        let skew = transforms(&json!([{"$type":"skewX","degrees":45}]), "", &l).unwrap();
        assert!((skew * Point::new(0.0, 2.0) - Point::new(2.0, 2.0)).hypot() < 1e-10);
        let clockwise = normalize_svg("M0 0 A5 5 0 0 1 10 0", 100).unwrap();
        let reverse = normalize_svg("M0 0 A5 5 0 0 0 10 0", 100).unwrap();
        assert!(clockwise.bounding_box().y0 < -4.99);
        assert!(reverse.bounding_box().y1 > 4.99);
        let small = normalize_svg("M0 0 A10 10 0 0 1 10 0", 100).unwrap();
        let large = normalize_svg("M0 0 A10 10 0 1 1 10 0", 100).unwrap();
        assert!(large.elements().len() > small.elements().len());
    }
    #[test]
    fn viewbox_alignment_translates_in_logical_viewport_coordinates() {
        for (fit, expected) in [
            ("contain", Point::new(0.0, 50.0)),
            ("cover", Point::new(-100.0, 0.0)),
            ("fill", Point::ZERO),
        ] {
            let value = json!({"$type":"Drawing","viewBox":{"width":20,"height":10},"fit":fit,"contentAlignment":{"x":"end","y":"end"}});
            let scene = prepare(
                &value,
                &Resolver(value.clone()),
                &mut NoText,
                Config {
                    width: 100.0,
                    height: 100.0,
                    graphics_version: "v".into(),
                    ..Config::default()
                },
            )
            .unwrap();
            assert!((scene.root.transform * Point::ZERO - expected).hypot() < 1e-10);
        }
        let extent = stroke_bounds(&normalize_svg("M0 0L1 1 M30 50Z", 100).unwrap(), "round")
            .rect
            .unwrap();
        assert_eq!((extent.x1, extent.y1), (30.0, 50.0));
        let value = json!({"$type":"Drawing","viewBox":{"width":100,"height":100}});
        let mut config = Config {
            graphics_version: "v".into(),
            ..Config::default()
        };
        config.limits.coordinate = 3e38;
        assert!(
            prepare(&value, &Resolver(value.clone()), &mut NoText, config)
                .unwrap_err()
                .message
                .contains("host limits")
        );
    }
    #[test]
    fn limits_and_text_requests_reject_unsupported_keys_and_invalid_fields() {
        assert!(serde_json::from_value::<Limits>(json!({"pathCommands":1})).is_err());
        assert!(serde_json::from_value::<TextRequest>(json!({"text":"a","family":"Fixture","size":12,"weight":400,"italic":false,"letterSpacing":5})).is_err());
        let request = TextRequest {
            text: "a".into(),
            family: "Fixture".into(),
            size: 12.0,
            weight: 400,
            italic: false,
        };
        for (request, message) in [
            (
                TextRequest {
                    size: 0.0,
                    ..request.clone()
                },
                "font size",
            ),
            (
                TextRequest {
                    weight: 0,
                    ..request.clone()
                },
                "font weight",
            ),
            (
                TextRequest {
                    text: "a\nb".into(),
                    ..request
                },
                "multiline",
            ),
        ] {
            assert!(validate_text_request(&request, "")
                .unwrap_err()
                .message
                .contains(message));
        }
    }
}
