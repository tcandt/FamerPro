use anyhow::Context;
use clap::{Parser, Subcommand};
use fp_adb::AdbRunner;
use fp_domain::DeviceDriver;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::WorkflowValidator;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Parser)]
#[command(name = "farmerpro-cli")]
#[command(about = "FarmerPro Headless Automation Diagnostic CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List connected Android devices via ADB
    Devices,

    /// Validate a workflow JSON file against FarmerPro schema
    Validate {
        #[arg(short, long)]
        file: Option<PathBuf>,

        #[arg(value_name = "FILE")]
        path: Option<PathBuf>,
    },

    /// Run a sample workflow headlessly against a mock or real device
    Run {
        #[arg(short, long)]
        workflow: Option<PathBuf>,

        #[arg(value_name = "WORKFLOW")]
        path: Option<PathBuf>,

        #[arg(long, default_value_t = false)]
        mock: bool,

        #[arg(short, long)]
        serial: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Devices => {
            println!("== FarmerPro ADB Discovery ==");
            let runner = AdbRunner::default();
            match runner.list_devices().await {
                Ok(devices) => {
                    if devices.is_empty() {
                        println!("No connected Android devices found via ADB.");
                    } else {
                        println!("Found {} device(s):", devices.len());
                        for d in devices {
                            println!(
                                "  - [{}] Serial: {} | Model: {:?} | State: {:?}",
                                d.id, d.adb_serial, d.model, d.connection_state
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run ADB discovery: {}", e);
                }
            }
        }

        Commands::Validate { file, path } => {
            let target_file = file
                .or(path)
                .context("Must provide a workflow JSON file via --file or positional argument")?;
            println!("Validating workflow file: {}", target_file.display());
            let content = fs::read_to_string(&target_file)?;
            match WorkflowValidator::parse_and_validate(&content) {
                Ok(doc) => {
                    println!(
                        "Validation SUCCESS: '{}' (schema v{}) with {} nodes and {} edges.",
                        doc.name,
                        doc.schema_version,
                        doc.graph.nodes.len(),
                        doc.graph.edges.len()
                    );
                }
                Err(e) => {
                    eprintln!("Validation FAILED:\n{}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Run {
            workflow,
            path,
            mock,
            serial,
        } => {
            let target_wf = workflow
                .or(path)
                .context("Must provide a workflow JSON file via --workflow or positional argument")?;
            println!("Loading workflow: {}", target_wf.display());
            let content = fs::read_to_string(&target_wf)?;
            let doc = WorkflowValidator::parse_and_validate(&content)?;

            let driver: Arc<dyn DeviceDriver> = match (mock, serial) {
                (false, Some(s)) => {
                    println!("Connecting to real ADB device: {}", s);
                    Arc::new(fp_adb::AdbDeviceDriver::new(s, AdbRunner::default()))
                }
                _ => {
                    println!("Using FakeDeviceDriver for deterministic headless execution.");
                    Arc::new(FakeDeviceDriver::new("fake-device-01", 1080, 1920))
                }
            };

            let (_tx, rx) = watch::channel(ExecutionSignal::Running);
            let mut executor = WorkflowExecutor::new(driver, rx);
            let mut ctx =
                ExecutionContext::new("cli-job-01".to_string(), "target-device".to_string());

            println!("Executing workflow '{}'...", doc.name);
            let state = executor
                .execute_workflow(
                    &doc,
                    &mut ctx,
                    Some(Arc::new(|event| {
                        println!(
                            "  [STEP EVENT] Node: {} | Type: {:?} | Time: {}",
                            event.node_id, event.event_type, event.at
                        );
                    })),
                )
                .await?;

            println!("Workflow execution finished with state: {:?}", state);
        }
    }

    Ok(())
}
