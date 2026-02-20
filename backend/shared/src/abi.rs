use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractSpec {
    #[serde(rename = "type")]
    pub spec_type: String,
    pub name: String,
    pub inputs: Vec<InputSpec>,
    pub outputs: Vec<OutputSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputSpec {
    pub name: String,
    pub value: TypeValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputSpec {
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeValue {
    #[serde(rename = "type")]
    pub type_name: String,
}

pub fn extract_abi(wasm_path: &str) -> Result<Vec<ContractSpec>> {
    let output = Command::new("soroban")
        .args(["contract", "bindings", "json", "--wasm", wasm_path])
        .output()
        .context("Failed to run soroban bindings")?;

    if !output.status.success() {
        anyhow::bail!("soroban bindings failed");
    }

    serde_json::from_slice(&output.stdout).context("Failed to parse spec")
}

pub fn generate_markdown(specs: &[ContractSpec], name: &str) -> String {
    let mut md = format!("# {}\n\n## Functions\n\n", name);

    for spec in specs.iter().filter(|s| s.spec_type == "function") {
        md.push_str(&format!("### `{}`\n\n", spec.name));

        if let Some(doc) = &spec.doc {
            md.push_str(&format!("{}\n\n", doc));
        }

        md.push_str("**Parameters:**\n");
        if spec.inputs.is_empty() {
            md.push_str("- None\n");
        } else {
            for input in &spec.inputs {
                md.push_str(&format!(
                    "- `{}`: `{}`\n",
                    input.name, input.value.type_name
                ));
            }
        }

        md.push_str("\n**Returns:** ");
        if spec.outputs.is_empty() {
            md.push_str("`void`");
        } else {
            let types: Vec<&str> = spec.outputs.iter().map(|o| o.type_name.as_str()).collect();
            md.push_str(&format!("`{}`", types.join(", ")));
        }

        md.push_str("\n\n**Example:**\n```rust\n");
        md.push_str(&format!("contract.{}(", spec.name));
        for (i, input) in spec.inputs.iter().enumerate() {
            if i > 0 {
                md.push_str(", ");
            }
            md.push_str(&input.name);
        }
        md.push_str(");\n```\n\n---\n\n");
    }

    md
}
