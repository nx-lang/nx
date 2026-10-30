use super::*;
use nx_graphics::{Config, DeclarationResolver, Identity};
use serde_json::{json, Value};

struct Identities(BTreeMap<String, String>);
impl DeclarationResolver for Identities {
    fn resolve(&self, path: &str) -> Option<Identity> {
        self.0.get(path).map(|name| Identity {
            module: nx_graphics::MODULE.into(),
            version: "test".into(),
            declaration: name.clone(),
        })
    }
}
// A deliberately small test-only SVG reader for the versioned corpus, not a production backend.
fn read_svg(node: roxmltree::Node<'_, '_>, path: &str, ids: &mut Identities) -> Value {
    let name = node.tag_name().name();
    let kind = match name {
        "svg" => "Drawing",
        "g" => "Group",
        "rect" => "Rect",
        "path" => "Path",
        _ => panic!("unexpected corpus SVG tag {name}"),
    };
    ids.0.insert(path.into(), kind.into());
    let mut v = json!({"$type":kind});
    let number = |key: &str, default: f64| {
        node.attribute(key)
            .map(|s| s.parse::<f64>().unwrap())
            .unwrap_or(default)
    };
    if name == "svg" {
        let values = node
            .attribute("viewBox")
            .unwrap()
            .split_whitespace()
            .map(|s| s.parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        v["viewBox"] = json!({"x":values[0],"y":values[1],"width":values[2],"height":values[3]});
    }
    if name == "rect" {
        for key in ["x", "y", "width", "height"] {
            v[key] = json!(number(key, 0.0));
        }
    }
    if name == "path" {
        v["data"] = json!(node.attribute("d").unwrap());
        v["fillRule"] = json!(node.attribute("fill-rule").unwrap_or("nonzero"));
    }
    if let Some(color) = node.attribute("fill") {
        v["fill"] = if color == "none" {
            json!("none")
        } else {
            json!({"$type":"Paint.color","color":nx_value::color::parse(color).unwrap()})
        };
    }
    if let Some(color) = node.attribute("stroke") {
        v["stroke"] = json!({"paint":{"$type":"Paint.color","color":nx_value::color::parse(color).unwrap()},"width":number("stroke-width",1.0)});
    }
    if node.attribute("opacity").is_some() {
        v["opacity"] = json!(number("opacity", 1.0));
    }
    if let Some(transform) = node.attribute("transform") {
        v["transform"] = Value::Array(
            svgtypes::TransformListParser::from(transform)
                .map(|t| match t.unwrap() {
                    svgtypes::TransformListToken::Translate { tx, ty } => {
                        json!({"$type":"Transform.translate","x":tx,"y":ty})
                    }
                    svgtypes::TransformListToken::Scale { sx, sy } => {
                        json!({"$type":"Transform.scale","x":sx,"y":sy})
                    }
                    other => panic!("unexpected corpus transform {other:?}"),
                })
                .collect(),
        );
    }
    if name == "svg" || name == "g" {
        v["children"] = Value::Array(
            node.children()
                .filter(|n| n.is_element())
                .enumerate()
                .map(|(i, n)| read_svg(n, &format!("{path}/children/{i}"), ids))
                .collect(),
        );
    }
    v
}
#[test]
fn versioned_svg_entries_render_as_nx_with_their_analytic_and_pixel_expectations() {
    let expectations: Value =
        serde_json::from_str(include_str!("../fixtures/svg-v1/expectations.json")).unwrap();
    for fixture in expectations["references"].as_array().unwrap() {
        let svg = match fixture["file"].as_str().unwrap() {
            "opacity.svg" => include_str!("../fixtures/svg-v1/opacity.svg"),
            "transforms.svg" => include_str!("../fixtures/svg-v1/transforms.svg"),
            "ring.svg" => include_str!("../fixtures/svg-v1/ring.svg"),
            _ => panic!("missing corpus entry"),
        };
        let doc = roxmltree::Document::parse(svg).unwrap();
        let root = doc.root_element();
        let width = root.attribute("width").unwrap().parse::<f64>().unwrap();
        let height = root.attribute("height").unwrap().parse::<f64>().unwrap();
        let mut ids = Identities(BTreeMap::new());
        let value = read_svg(root, "", &mut ids);
        let fonts = Fonts::fixture().unwrap();
        let scene = nx_graphics::prepare(
            &value,
            &ids,
            &mut Fonts::fixture().unwrap(),
            Config {
                width,
                height,
                graphics_version: "test".into(),
                ..Config::default()
            },
        )
        .unwrap();
        let mut surface = sk::surfaces::raster_n32_premul((width as i32, height as i32)).unwrap();
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
        for sample in fixture["samples"].as_array().unwrap() {
            let mut pixel = [0u8; 4];
            assert!(surface.canvas().read_pixels(
                &info,
                &mut pixel,
                4,
                (
                    sample[0].as_i64().unwrap() as i32,
                    sample[1].as_i64().unwrap() as i32
                )
            ));
            if sample[2] == "nonzero" {
                assert!(pixel[3] > 0);
            } else {
                assert!(
                    (pixel[3] as i64 - sample[2].as_i64().unwrap()).abs() <= 1,
                    "{}: {sample}: {pixel:?}",
                    fixture["file"]
                );
            }
        }
        if fixture["file"] == "transforms.svg" {
            assert_eq!(
                scene.root.children[0].transform * kurbo::Point::new(1.0, 0.0),
                kurbo::Point::new(12.0, 0.0)
            );
        }
    }
}

#[test]
fn small_text_ink_scales_through_the_viewbox_and_local_transform() {
    let mut fonts = Fonts::fixture().unwrap();
    let ids = Identities(BTreeMap::from([
        ("".into(), "Drawing".into()),
        ("/children/0".into(), "Text".into()),
    ]));
    let config = Config {
        width: 800.0,
        height: 200.0,
        graphics_version: "test".into(),
        ..Config::default()
    };
    let mut v = json!({"viewBox":{"width":40,"height":10},"children":[{"text":"Hello","fontFamily":"Fixture","fontSize":2,"x":2,"y":5}]});
    let small = nx_graphics::prepare(&v, &ids, &mut fonts, config.clone()).unwrap();
    v["children"][0]["fontSize"] = json!(20);
    v["children"][0]["x"] = json!(20);
    v["children"][0]["y"] = json!(50);
    v["children"][0]["transform"] = json!([{"$type":"Transform.scale","x":0.1}]);
    let large = nx_graphics::prepare(&v, &ids, &mut fonts, config).unwrap();
    let a = small.root.children[0].painted.rect.unwrap();
    let b = large.root.children[0].painted.rect.unwrap();
    for (a, b) in [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)] {
        assert!((a - b).abs() < 0.02, "{a} vs {b}");
    }
    assert!(render_png(&small, &fonts).is_ok());
    assert!(render_png(&large, &fonts).is_ok());
}

#[test]
fn shared_gradient_scenes_cover_placement_transforms_strokes_and_compositing() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/gradients/scenes.json"
    );
    let mut cases: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let update = std::env::var_os("NX_UPDATE_GRADIENT_SCENES").is_some();
    fn identities(value: &Value, path: &str, ids: &mut Identities) {
        ids.0
            .insert(path.into(), value["$type"].as_str().unwrap().into());
        if let Some(children) = value["children"].as_array() {
            for (i, child) in children.iter().enumerate() {
                identities(child, &format!("{path}/children/{i}"), ids);
            }
        }
    }
    for case in cases.as_array_mut().unwrap() {
        let mut ids = Identities(BTreeMap::new());
        identities(&case["value"], "", &mut ids);
        let mut fonts = Fonts::fixture().unwrap();
        let scene = nx_graphics::prepare(
            &case["value"],
            &ids,
            &mut fonts,
            Config {
                width: 100.0,
                height: 100.0,
                graphics_version: "test".into(),
                ..Config::default()
            },
        )
        .unwrap();
        let prepared = serde_json::to_value(&scene).unwrap();
        if update {
            case["prepared"] = prepared;
        } else {
            assert_eq!(case["prepared"], prepared, "{}", case["name"]);
        }
        let info = sk::ImageInfo::new(
            (100, 100),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Premul,
            sk::ColorSpace::new_srgb(),
        );
        let mut surface = sk::surfaces::raster(&info, None, None).unwrap();
        surface.canvas().clear(sk::Color::TRANSPARENT);
        scene
            .draw(&mut SkiaDestination::new(surface.canvas(), &fonts))
            .unwrap();
        let pixel_info = sk::ImageInfo::new(
            (1, 1),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Premul,
            sk::ColorSpace::new_srgb(),
        );
        for sample in case["samples"].as_array().unwrap() {
            let mut actual = [0u8; 4];
            assert!(surface.canvas().read_pixels(
                &pixel_info,
                &mut actual,
                4,
                (
                    sample["x"].as_i64().unwrap() as i32,
                    sample["y"].as_i64().unwrap() as i32
                )
            ));
            for i in 0..4 {
                assert!(
                    (actual[i] as f64 / 255.0 - sample["expected"][i].as_f64().unwrap()).abs()
                        <= 2.0 / 255.0,
                    "{}: {actual:?} expected {}",
                    case["name"],
                    sample["expected"]
                );
            }
        }
    }
    if update {
        std::fs::write(file, serde_json::to_string_pretty(&cases).unwrap() + "\n").unwrap();
    }
}

#[test]
fn pinned_skia_collapses_tiny_local_gradients_even_when_enlarged() {
    // Probe the upstream behavior motivating neutral preparation's minimum span guard.
    // These deliberately bypass preparation; magnification does not prevent Skia's collapse.
    let stops = json!([
        {"offset":0,"color":[1,0,0,1]},
        {"offset":1,"color":[0,0,1,1]}
    ]);
    for value in [
        json!({"kind":"linearGradient","x1":0,"y1":0,"x2":1.0/32768.0,"y2":0,"stops":stops}),
        json!({"kind":"radialGradient","cx":0,"cy":0,"radius":1.0/32768.0,"stops":stops}),
        json!({"kind":"linearGradient","x1":0,"y1":0,"x2":0.00002,"y2":0,"stops":stops}),
        json!({"kind":"radialGradient","cx":0,"cy":0,"radius":0.00002,"stops":stops}),
    ] {
        let gradient: nx_graphics::Paint = serde_json::from_value(value).unwrap();
        let info = sk::ImageInfo::new(
            (40, 40),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Premul,
            sk::ColorSpace::new_srgb(),
        );
        let mut surface = sk::surfaces::raster(&info, None, None).unwrap();
        surface.canvas().clear(sk::Color::TRANSPARENT);
        surface.canvas().scale((480000.0, 480000.0));
        surface
            .canvas()
            .draw_rect(sk::Rect::from_wh(0.001, 0.001), &paint(&gradient).unwrap());
        let info = sk::ImageInfo::new(
            (1, 1),
            sk::ColorType::RGBA8888,
            sk::AlphaType::Premul,
            sk::ColorSpace::new_srgb(),
        );
        let mut pixel = [0u8; 4];
        assert!(surface.canvas().read_pixels(&info, &mut pixel, 4, (2, 2)));
        assert_eq!(pixel, [0, 0, 255, 255]);
    }
}
