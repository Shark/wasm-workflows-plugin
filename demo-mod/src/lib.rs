wit_bindgen_rust::export!("../src/app/wasm/workflow.wit");

struct Workflow;

impl workflow::Workflow for Workflow {
    fn invoke(ctx: workflow::Invocation) -> workflow::Node {
        workflow::Node {
            phase: "Succeeded".to_string(),
            message: "Hello from Wasm!".to_string(),
        }
    }
}
