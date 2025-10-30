//! Debug test to understand what's happening with type checking

use nx_types::check_str;

#[test]
fn debug_undefined_identifier() {
    let source = r#"
        let <Test /> = undefined_var
    "#;

    let result = check_str(source, "undefined.nx");

    println!("=== MODULE ===");
    if let Some(module) = &result.module {
        println!("Items: {}", module.items().len());
        for (i, item) in module.items().iter().enumerate() {
            println!("  Item {}: {:?}", i, item);
        }
    } else {
        println!("Module is None");
    }

    println!("\n=== DIAGNOSTICS ===");
    println!("Total: {}", result.diagnostics.len());
    for (i, diag) in result.diagnostics.iter().enumerate() {
        println!("  Diagnostic {}: {:?}", i, diag.message());
        println!("    Severity: {:?}", diag.severity());
    }

    println!("\n=== ERRORS ===");
    let errors = result.errors();
    println!("Total errors: {}", errors.len());
    for (i, err) in errors.iter().enumerate() {
        println!("  Error {}: {}", i, err.message());
    }
}

#[test]
fn debug_type_mismatch() {
    let source = r#"
        let <Test /> = 42 + "string"
    "#;

    let result = check_str(source, "mismatch.nx");

    println!("=== MODULE ===");
    if let Some(module) = &result.module {
        println!("Items: {}", module.items().len());
        for (i, item) in module.items().iter().enumerate() {
            match item {
                nx_hir::Item::Function(func) => {
                    println!("  Item {}: Function {}", i, func.name);
                    println!("    Body expr: {:?}", func.body);
                    // Try to look up the expression
                    let body_expr = module.expr(func.body);
                    println!("    Body: {:?}", body_expr);
                }
                _ => println!("  Item {}: {:?}", i, item),
            }
        }
    }

    println!("\n=== DIAGNOSTICS ===");
    println!("Total: {}", result.diagnostics.len());
    for (i, diag) in result.diagnostics.iter().enumerate() {
        println!("  Diagnostic {}: {}", i, diag.message());
    }

    println!("\n=== TYPE ENVIRONMENT ===");
    println!("Bindings: {}", result.type_env.len());
    for (name, ty) in result.type_env.bindings() {
        println!("  {} -> {}", name, ty);
    }
}
