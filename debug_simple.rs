use context_creator::core::semantic::query_engine::QueryEngine;
use tree_sitter::Parser;

fn main() {
    let rust_content = r#"
use crate::models::User;
use shared::types::ApiResponse;

pub fn handle_user_request(user_id: u32) -> ApiResponse<User> {
    let user = User::find(user_id);
    ApiResponse::ok(user)
}
"#;

    let language = tree_sitter_rust::language();
    let query_engine = QueryEngine::new(language, "rust").unwrap();

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();

    let result = query_engine.analyze_with_parser(&mut parser, rust_content).unwrap();

    println!("Type references found: {}", result.type_references.len());
    for type_ref in &result.type_references {
        println!("  - {}: {:?} (definition_path: {:?})", 
                type_ref.name, type_ref.module, type_ref.definition_path);
    }
}