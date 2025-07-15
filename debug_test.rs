#[cfg(test)]
mod tests {
    use crate::Config;
    use clap::Parser;

    #[test]
    fn debug_copy_with_output_conflict() {
        let config = Config::parse_from(["code-digest", "src", "--copy", "-o", "out.md"]);
        println!("Config: copy={}, output_file={:?}", config.copy, config.output_file);
        let result = config.validate();
        println!("Result: {:?}", result);
        if let Err(e) = result {
            println!("Error string: '{}'", e.to_string());
            println!("Contains 'Cannot specify both': {}", e.to_string().contains("Cannot specify both"));
        }
    }
}