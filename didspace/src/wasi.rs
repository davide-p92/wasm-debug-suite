use wasmparser::{Parser, Payload, ComponentType, ComponentValType, ComponentExternalKind};
use anyhow::Result;

pub fn detect_wasi_imports(wasm_bytes: &[u8]) -> Result<(bool, String)> {
    let mut parser = Parser::new(0);
    let mut wasi_found = false;
    let mut out = String::new();
    out.push_str("WASI Imports\n");
    out.push_str("===========\n");

    for payload in parser.parse_all(wasm_bytes) {
        match payload? {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import?;
                    if import.module.starts_with("wasi") {
                        out.push_str(&format!(
                            "Found WASI import: module='{}', name='{}'\n",
                            import.module,
                            import.name
                        ));
                        println!(
                            "Found WASI import: module='{}', name='{}'",
                            import.module, import.name
                        );
                        wasi_found = true;
                    }
                }
            }
            _ => {}
        }
    }
    
    if !wasi_found {
        out.push_str("No WASI imports found.\n");
    }

    Ok((wasi_found, out))
}

pub fn detect_component_model(wasm_bytes: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(0);
    let mut component_found = false;

    for payload in parser.parse_all(wasm_bytes) {
        match payload? {
            Payload::ComponentSection { .. } => {
                println!("Found Component Model section!");
                component_found = true;
            }
            Payload::ComponentTypeSection(_) => println!("Found Component Type section"),
            Payload::ComponentImportSection(_) => println!("Found Component Import section"),
            Payload::ComponentExportSection(_) => println!("Found Component Export section"),
            _ => {}
        }
    }

    Ok(component_found)
}

pub fn analyze_component(wasm_bytes: &[u8]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    let mut parser = Parser::new(0);
    let mut comp_types = Vec::new();
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut func_count = 0;

    println!("Component Analysis:");
    for payload in parser.parse_all(wasm_bytes) {
        match payload? {
            // Component Type Section
            Payload::ComponentTypeSection(reader) => {
                //println!("• Type Section:");
                for ty in reader {
                    let ty = ty?;
                    match ty {
                        ComponentType::Func(func_ty) => {
                            let params: Vec<String> = func_ty
                                .params
                                .iter()
                                .map(|(name, val_ty)| {
                                    let ty_str = match val_ty {
                                        wasmparser::ComponentValType::Primitive(p) => format!("{:?}", p),
                                        wasmparser::ComponentValType::Type(idx) => format!("type{}", idx),
//                                        wasmparser::ComponentValType::Borrow(idx) => format!("borrow{}", idx),
//                                        wasmparser::ComponentValType::Own(idx) => format!("own{}", idx),
                                    };

                                    format!("{}: {}", name, ty_str)
                                })
                                .collect();

                            // Format res
                            let result = match func_ty.result {
                                Some(val_ty) => {
                                    let ty_str = match val_ty {
                                        wasmparser::ComponentValType::Primitive(p) => format!("{:?}", p),
                                        wasmparser::ComponentValType::Type(idx) => format!("{}", idx),
//                                        wasmparser::ComponentValType::Borrow(idx) => format!("borrow{}", idx),
//                                        wasmparser::ComponentValType::Own(idx) => format!("own{}", idx),
                                    };
                                    format!(" -> {}", ty_str)
                                }
                                None => String::new(),
                            };
                            comp_types.push(format!("  - func({}){}", params.join(", "), result));
                            func_count += 1;
                        },
                        _ => {
                            println!("  - other type: {:?}", ty);
                            comp_types.push(format!("  - other type: {:?}", ty));
                        }
                    }
                }
            }

            // Component Import Section
            Payload::ComponentImportSection(reader) => {
                //println!("• Import Section:");
                for import in reader {
                    let import = import?;
                    //println!("  - name={:?}, kind={:?}", import.name, import.ty);
                    imports.push(format!("{:?}", import.name));
                }
            }

            // Component Export Section
            Payload::ComponentExportSection(reader) => {
                //println!("• Export Section:");
                for export in reader {
                    let export = export?;
                    //println!("  - name={:?}, kind={:?}", export.name, export.kind);
                    exports.push(format!("{:?}", export.name));
                }
            }

            _ => {}
        }
    }

    let mut out = String::new();
    out.push_str("Component Analysis:\n");
    out.push_str("✅ Component Model detected\n\n");
    out.push_str("Types:\n");
    println!("Component Analysis:");
    println!("✅ Component Model detected");
    println!("Types:");
    for ctp in comp_types {
        println!("  {}", ctp);
        out.push_str(&format!("{}\n", ctp));
    }
    println!("Imports:");
    out.push_str("\nImports:\n");
    for imp in imports {
        println!("  {}\n", imp);
        out.push_str(&format!("  {}\n", imp));
    }
    println!("Exports:");
    out.push_str("\nExports:\n");
    for exp in exports {
        println!("  {}", exp);
        out.push_str(&format!("  {}\n", exp));
    }
    out.push_str("\nTypes Summary:\n");
    out.push_str(&format!("  • Functions: {}\n", func_count));
    println!("Types Summary:");
    println!("  • Functions: {}", func_count);

    Ok(out)
}

