use workflow_model::{
    model::{Parameter, Phase, PluginInvocation, PluginResult},
    plugin::ArtifactManager,
};

fn main() {
    workflow_model::plugin::main(Box::new(run));
}

fn run(invocation: PluginInvocation, _: ArtifactManager) -> anyhow::Result<PluginResult> {
    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: format!("Success, have {} parameters", invocation.parameters.len()),
        outputs: workflow_model::model::Outputs {
            artifacts: Default::default(),
            parameters: vec![Parameter {
                name: "question".into(),
                value: "Schnapspraline?".into(),
            }],
        },
    })
}
