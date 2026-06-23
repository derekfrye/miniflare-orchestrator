use crate::docs_model::InstructionsDocument;
use maud::{DOCTYPE, Markup, html};
use serde_json::to_string_pretty;

pub const DOCS_CSS: &str = include_str!("docs.css");

#[must_use]
pub fn render(instructions: &InstructionsDocument) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Worker Runtime Host" }
                link rel="stylesheet" href="/docs.css";
            }
            body {
                header {
                    h1 { "Worker Runtime Host" }
                    p.muted { "Machine-readable and human-readable runtime host instructions." }
                    p { "Docs version: " (instructions.version) }
                    p { "Bootstrap mode: " code { (instructions.bootstrap_mode) } }
                    p { "Host root: " code { (instructions.host_root) } }
                    p { "Lease root: " code { (instructions.lease_root) } }
                    p { "Lease port range: " code { (instructions.lease_port_range) } }
                }
                section {
                    h2 { "Deploy Flow" }
                    ol { @for step in &instructions.deploy_flow { li { (step) } } }
                }
                section {
                    h2 { "Lease Flow" }
                    ol { @for step in &instructions.lease_flow { li { (step) } } }
                }
                section {
                    h2 { "Endpoints" }
                    table {
                        tr { th { "Method" } th { "Path" } th { "Description" } th { "Content Types" } }
                        @for endpoint in &instructions.endpoints {
                            tr {
                                td { code { (endpoint.method) } }
                                td { code { (endpoint.path) } }
                                td { (endpoint.description) }
                                td { (endpoint.content_types.join(", ")) }
                            }
                        }
                    }
                }
                section {
                    h2 { "Client Contract" }
                    @for example in &instructions.client_contract {
                        article {
                            h3 { (example.title) }
                            p.muted { (example.purpose) }
                            p { code { (example.method) } " " code { (example.path) } }
                            h4 { "Request Example" }
                            pre { code { (to_string_pretty(&example.request_example).unwrap_or_else(|_| example.request_example.to_string())) } }
                            h4 { "Response Example" }
                            pre { code { (to_string_pretty(&example.response_example).unwrap_or_else(|_| example.response_example.to_string())) } }
                        }
                    }
                }
                section {
                    h2 { "Projects" }
                    table {
                        tr {
                            th { "Name" } th { "Runtime" } th { "Static" } th { "State" }
                            th { "Logs" } th { "Port" } th { "Health URL" }
                        }
                        @for project in &instructions.projects {
                            tr {
                                td { code { (project.name) } }
                                td { code { (project.runtime_dir) } }
                                td { code { (project.static_dir) } }
                                td { code { (project.state_dir) } }
                                td { code { (project.log_dir) } }
                                td { code { (project.port) } }
                                td { code { (project.health_url) } }
                            }
                        }
                    }
                }
                section {
                    h2 { "Notes" }
                    ul { @for note in &instructions.notes { li { (note) } } }
                }
                section {
                    h2 { "Lease Binaries" }
                    ul {
                        li { "Worker binary: " code { (instructions.lease_worker_bin) } }
                        li { "Wrangler binary: " code { (instructions.lease_wrangler_bin) } }
                    }
                }
            }
        }
    }
}
