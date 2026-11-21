use anyhow::Context;
use clap::{Parser, Subcommand};
use fp_adb::AdbRunner;
use fp_builder::{
    orchestrator::run_concurrent_two_device_verification, seed_two_poc_projects, MiniToolPackager,
};
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
#[command(about = "FarmerPro Headless Automation Diagnostic & Builder CLI", long_about = None)]
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

    /// Export the two PoC Mini Auto-Tool packages using fp-builder and external Ed25519 producer keystore
    ExportPoc {
        #[arg(long)]
        repo_root: Option<PathBuf>,
    },

    /// Run multi-device concurrent lease & generation fencing verification
    VerifyMultiDevice,
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

        Commands::ExportPoc { repo_root } => {
            let root = repo_root.unwrap_or_else(|| std::env::current_dir().unwrap());
            let runtime_exe = root.join("target").join("release").join("fp-mini-runtime.exe");
            let bin_dir = root.join("bin");
            let (jx2_proj, slg_proj) = seed_two_poc_projects(&root)?;

            let jx2_out = root.join("releases").join("poc").join("Auto_JX2_Mobile");
            let slg_out = root.join("releases").join("poc").join("Auto_Kingdom_SLG");

            let rep1 = MiniToolPackager::build_and_export_release(
                &jx2_proj,
                &runtime_exe,
                Some(&bin_dir),
                &jx2_out,
                None,
            )?;
            let rep2 = MiniToolPackager::build_and_export_release(
                &slg_proj,
                &runtime_exe,
                Some(&bin_dir),
                &slg_out,
                None,
            )?;

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "command": "export-poc",
                    "status": "PASS",
                    "releases": [rep1, rep2]
                }))?
            );
        }

        Commands::VerifyMultiDevice => {
            let report = run_concurrent_two_device_verification().await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}
