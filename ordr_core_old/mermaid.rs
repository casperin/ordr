use crate::{
    graph::{Ctx, Er, Graph},
    job::Job,
};

/// Creates a mermaid diagram.
#[must_use]
pub(crate) fn mermaid<C: Ctx, E: Er>(graph: &Graph<C, E>, job: &Job<C, E>) -> String {
    let mut lines = vec![
        "flowchart LR".into(),
        "classDef target   fill:#fff,color:#000,stroke-width:2px,stroke:#f0a".into(),
        "classDef given    fill:#fff,color:#000,stroke-width:2px,stroke:#073".into(),
        "classDef inactive fill:#eee,color:#bbb,stroke-width:2px,stroke:#eee".into(),
    ];

    let pending = job.pending(graph);

    for (i, node) in graph.nodes.iter().enumerate() {
        let role = if job.targets.contains(&node.id) {
            "target"
        } else if job.inputs.contains_key(&node.id) {
            "given"
        } else if pending.contains(&i) {
            "active"
        } else {
            "inactive"
        };
        let line = format!("v{i}[{}]:::{role}", node.name);
        lines.push(line);
    }

    for (i, deps) in graph.adj.iter().enumerate() {
        if deps.is_empty() {
            continue;
        }
        let input = deps
            .iter()
            .map(|i| format!("v{i}"))
            .collect::<Vec<_>>()
            .join(" & ");

        lines.push(format!("{input} --> v{i}"));
    }

    lines.join("\n    ")
}
