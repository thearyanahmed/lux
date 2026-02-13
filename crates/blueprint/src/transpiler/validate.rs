use super::error::TranspileError;
use super::ir::Phase;
use crate::transpiler::ir::Blueprint;
use std::collections::{HashMap, HashSet};

/// validate the IR for structural correctness
pub fn validate_blueprint(bp: &Blueprint) -> Result<(), TranspileError> {
    validate_phase_dependencies(bp)?;
    validate_unique_phase_names(bp)?;
    Ok(())
}

fn validate_unique_phase_names(bp: &Blueprint) -> Result<(), TranspileError> {
    let mut seen = HashSet::new();
    for phase in &bp.phases {
        if !seen.insert(&phase.name) {
            return Err(TranspileError::new(format!(
                "duplicate phase name: '{}'",
                phase.name
            )));
        }
    }
    Ok(())
}

/// check for cycles in phase dependency graph using topological sort
fn validate_phase_dependencies(bp: &Blueprint) -> Result<(), TranspileError> {
    let phase_names: HashSet<&str> = bp.phases.iter().map(|p| p.name.as_str()).collect();

    // validate all dependencies reference existing phases
    for phase in &bp.phases {
        for dep in &phase.depends_on {
            if !phase_names.contains(dep.as_str()) {
                return Err(TranspileError::new(format!(
                    "phase '{}' depends on unknown phase '{}'",
                    phase.name, dep
                )));
            }
        }
    }

    // topological sort to detect cycles
    topological_sort(&bp.phases)?;

    Ok(())
}

/// topological sort using kahn's algorithm — returns ordered phase names or error on cycle
pub fn topological_sort(phases: &[Phase]) -> Result<Vec<String>, TranspileError> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for phase in phases {
        in_degree.entry(phase.name.as_str()).or_insert(0);
        adjacency.entry(phase.name.as_str()).or_default();
    }

    for phase in phases {
        for dep in &phase.depends_on {
            adjacency
                .entry(dep.as_str())
                .or_default()
                .push(phase.name.as_str());
            *in_degree.entry(phase.name.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();
    // sort for deterministic order
    queue.sort();

    let mut result = Vec::new();

    while let Some(name) = queue.pop() {
        result.push(name.to_string());
        if let Some(neighbors) = adjacency.get(name) {
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                        queue.sort();
                    }
                }
            }
        }
    }

    if result.len() != phases.len() {
        return Err(TranspileError::new("cycle detected in phase dependencies"));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::grammar::parse;
    use crate::transpiler::resolve::transpile;

    fn bp_from_str(input: &str) -> Result<Blueprint, String> {
        let ast = parse(input).map_err(|e| format!("parse error: {e}"))?;
        transpile(&ast).map_err(|e| format!("transpile error: {e}"))
    }

    #[test]
    fn test_valid_dependencies() {
        let bp = bp_from_str(
            r#"
blueprint "Test" {
    phase "a" {
        step "s1" {
            probe tcp 4221
            expect { connected: true }
        }
    }
    phase "b" {
        depends-on: "a"
        step "s2" {
            probe tcp 4222
            expect { connected: true }
        }
    }
    phase "c" {
        depends-on: "b"
        step "s3" {
            probe tcp 4223
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(validate_blueprint(&bp).is_ok());
    }

    #[test]
    fn test_unknown_dependency() {
        let bp = bp_from_str(
            r#"
blueprint "Test" {
    phase "a" {
        step "s1" {
            probe tcp 4221
            expect { connected: true }
        }
    }
    phase "b" {
        depends-on: "nonexistent"
        step "s2" {
            probe tcp 4222
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let result = validate_blueprint(&bp);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("unknown phase"),
            "error was: {}",
            err.message
        );
    }

    #[test]
    fn test_topological_sort_linear() {
        let bp = bp_from_str(
            r#"
blueprint "Test" {
    phase "a" {
        step "s1" {
            probe tcp 4221
            expect { connected: true }
        }
    }
    phase "b" {
        depends-on: "a"
        step "s2" {
            probe tcp 4222
            expect { connected: true }
        }
    }
    phase "c" {
        depends-on: "b"
        step "s3" {
            probe tcp 4223
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let order = topological_sort(&bp.phases);
        assert!(order.is_ok());
        let order = order.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_topological_sort_no_deps() {
        let bp = bp_from_str(
            r#"
blueprint "Test" {
    phase "x" {
        step "s1" {
            probe tcp 4221
            expect { connected: true }
        }
    }
    phase "y" {
        step "s2" {
            probe tcp 4222
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let order = topological_sort(&bp.phases);
        assert!(order.is_ok());
        let order = order.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_duplicate_phase_names() {
        let bp = bp_from_str(
            r#"
blueprint "Test" {
    phase "a" {
        step "s1" {
            probe tcp 4221
            expect { connected: true }
        }
    }
    phase "a" {
        step "s2" {
            probe tcp 4222
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let result = validate_blueprint(&bp);
        assert!(result.is_err());
    }
}
