use context_creator::core::semantic::resolver::{ModuleResolver, PythonModuleResolver};
use std::path::PathBuf;

fn main() {
    let resolver = PythonModuleResolver;
    
    // Test case from failing test
    let module_path = "src.models.user";
    let from_file = PathBuf::from("/tmp/test_project/src/services/user_service.py");
    let base_dir = PathBuf::from("/tmp/test_project");
    
    println!("Testing Python import resolution:");
    println!("  Module: {}", module_path);
    println!("  From file: {}", from_file.display());
    println!("  Base dir: {}", base_dir.display());
    
    match resolver.resolve_import(module_path, &from_file, &base_dir) {
        Ok(resolved) => {
            println!("\nResolved successfully!");
            println!("  Path: {}", resolved.path.display());
            println!("  Is external: {}", resolved.is_external);
            println!("  Confidence: {}", resolved.confidence);
        }
        Err(e) => {
            println!("\nFailed to resolve: {:?}", e);
        }
    }
    
    // Also test what the resolver is doing step by step
    println!("\n--- Step by step resolution ---");
    
    // Check if it's external
    if resolver.is_external_module(module_path) {
        println!("Module is considered external");
    } else {
        println!("Module is not external, attempting to resolve...");
        
        // Split the module path
        let parts: Vec<&str> = module_path.split('.').collect();
        println!("Module parts: {:?}", parts);
        
        // Try from base directory
        let mut path = base_dir.clone();
        for part in &parts {
            path = path.join(part);
        }
        let py_file = path.with_extension("py");
        println!("Trying: {}", py_file.display());
        if py_file.exists() {
            println!("  -> EXISTS!");
        } else {
            println!("  -> Not found");
        }
    }
}