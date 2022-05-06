use workflow_model::model::{Phase, PluginInvocation, PluginResult};

fn main() {
    workflow_model::plugin::main(Box::new(run));
}

fn run(invocation: PluginInvocation) -> anyhow::Result<PluginResult> {
    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: format!("Success, have {} parameters", invocation.parameters.len()),
        outputs: Default::default(),
    })
}
