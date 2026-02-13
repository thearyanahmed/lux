use color_eyre::eyre::{Result, WrapErr};

use crate::{oops, say};

/// handle `luxctl export <file.bp> [--format json]`
/// offline tool: reads .bp file, parses, transpiles, prints JSON to stdout.
/// no auth required.
pub fn export(file: &str, format: &str) -> Result<()> {
    if format != "json" {
        oops!("unsupported format '{}' — only 'json' is supported", format);
        return Ok(());
    }

    let source = std::fs::read_to_string(file)
        .wrap_err_with(|| format!("failed to read '{}'", file))?;

    let ast = blueprint::parser::parse(&source).map_err(|e| {
        color_eyre::eyre::eyre!(
            "parse error at line {}:{}: {}",
            e.line,
            e.col,
            e.message
        )
    })?;

    let bp = blueprint::transpiler::transpile(&ast).map_err(|e| {
        let msg = match e.context {
            Some(ctx) => format!("{}: {}", ctx, e.message),
            None => e.message,
        };
        color_eyre::eyre::eyre!("transpile error: {}", msg)
    })?;

    let json = serde_json::to_string_pretty(&bp)
        .wrap_err("failed to serialize blueprint to JSON")?;

    println!("{}", json);

    say!("exported {} phases, {} steps",
        bp.phases.len(),
        bp.phases.iter().map(|p| p.steps.len()).sum::<usize>()
    );

    Ok(())
}
