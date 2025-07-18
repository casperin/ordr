use crate::{Job, State};

/// Builds a simple mermaid diagram of the nodes that will be executed when running this job.
pub fn mermaid<S: State>(job: &Job<S>) -> String {
    let mut lines = vec!["flowchart LR".into()];
    for (id, deps) in &job.adj {
        let name = job.name(id);
        let dep_names: Vec<_> = deps.iter().map(|dep| job.name(dep)).collect();
        lines.push(format!("{name} --> {}", dep_names.join(" & ")));
    }
    lines.join("\n    ")
}
