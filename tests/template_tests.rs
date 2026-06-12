use rustbasic_core::template::TemplateEngine;
use serde_json::json;

#[test]
fn test_text_rendering() {
    let engine = TemplateEngine::new();
    let ctx = json!({});
    let res = engine.render("Hello World!", &ctx).unwrap();
    assert_eq!(res, "Hello World!");
}

#[test]
fn test_variable_interpolation() {
    let engine = TemplateEngine::new();
    let ctx = json!({ "name": "RustBasic" });
    let res = engine.render("Hello {{ name }}!", &ctx).unwrap();
    assert_eq!(res, "Hello RustBasic!");
}

#[test]
fn test_nested_path_interpolation() {
    let engine = TemplateEngine::new();
    let ctx = json!({
        "user": {
            "profile": {
                "first_name": "Heris"
            }
        }
    });
    let res = engine.render("Hello {{ user.profile.first_name }}!", &ctx).unwrap();
    assert_eq!(res, "Hello Heris!");
}

#[test]
fn test_conditional_if_true() {
    let engine = TemplateEngine::new();
    let ctx = json!({ "is_logged_in": true });
    let res = engine.render(
        "{% if is_logged_in %}Welcome back!{% else %}Please log in.{% endif %}",
        &ctx
    ).unwrap();
    assert_eq!(res, "Welcome back!");
}

#[test]
fn test_conditional_if_false() {
    let engine = TemplateEngine::new();
    let ctx = json!({ "is_logged_in": false });
    let res = engine.render(
        "{% if is_logged_in %}Welcome back!{% else %}Please log in.{% endif %}",
        &ctx
    ).unwrap();
    assert_eq!(res, "Please log in.");
}

#[test]
fn test_for_loop() {
    let engine = TemplateEngine::new();
    let ctx = json!({ "items": ["apple", "banana", "orange"] });
    let res = engine.render(
        "{% for item in items %}- {{ item }}\n{% endfor %}",
        &ctx
    ).unwrap();
    assert_eq!(res, "- apple\n- banana\n- orange\n");
}

#[test]
fn test_comparison_operator() {
    let engine = TemplateEngine::new();
    let ctx = json!({ "score": 85 });
    let res1 = engine.render(
        "{% if score >= 80 %}Pass{% else %}Fail{% endif %}",
        &ctx
    ).unwrap();
    assert_eq!(res1, "Pass");

    let ctx_fail = json!({ "score": 50 });
    let res2 = engine.render(
        "{% if score >= 80 %}Pass{% else %}Fail{% endif %}",
        &ctx_fail
    ).unwrap();
    assert_eq!(res2, "Fail");
}

#[test]
fn test_default_filter_tojson() {
    let engine = TemplateEngine::new();
    let ctx = json!({
        "data": { "id": 1, "status": "active" }
    });
    let res = engine.render("{{ data | tojson }}", &ctx).unwrap();
    assert_eq!(res, "{\"id\":1,\"status\":\"active\"}");
}

#[test]
fn test_custom_filter() {
    let mut engine = TemplateEngine::new();
    engine.add_filter("upper", |val, _| {
        if let serde_json::Value::String(s) = val {
            serde_json::Value::String(s.to_uppercase())
        } else {
            val.clone()
        }
    });

    let ctx = json!({ "name": "rustbasic" });
    let res = engine.render("Hello {{ name | upper }}!", &ctx).unwrap();
    assert_eq!(res, "Hello RUSTBASIC!");
}

#[test]
fn test_chained_filters() {
    let mut engine = TemplateEngine::new();
    engine.add_filter("upper", |val, _| {
        if let serde_json::Value::String(s) = val {
            serde_json::Value::String(s.to_uppercase())
        } else {
            val.clone()
        }
    });

    let ctx = json!({
        "data": "hello"
    });
    let res = engine.render("{{ data | upper | tojson }}", &ctx).unwrap();
    assert_eq!(res, "\"HELLO\"");
}
