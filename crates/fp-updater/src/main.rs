use anyhow::Result;
use clap::{Parser, Subcommand};
use fp_bundle_contract::RuntimeCompatibilityProfile;
use fp_updater::{
    FaultInjectionPoint, MiniToolUpdater, UpdateExecutionOptions,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "fp-updater",
    about = "FarmerPro External Crash-Safe Updater & Version Manager (P5.2)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a candidate release directory against PE RT_RCDATA #7777 + Ed25519 + 5-dim Compatibility Gate
    CheckCompat {
        #[arg(long)]
        release_dir: PathBuf,
    },
    /// Install an initial versioned Mini Tool release into <install-root>
    InstallInitial {
        #[arg(long)]
        install_root: PathBuf,
        #[arg(long)]
        release_dir: PathBuf,
        #[arg(long)]
        skip_child_verify_exec: bool,
    },
    /// Crash-safe upgrade of an existing Mini Tool install root to a newer SemVer release
    Upgrade {
        #[arg(long)]
        install_root: PathBuf,
        #[arg(long)]
        release_dir: PathBuf,
        #[arg(long)]
        pause_at: Option<String>,
        #[arg(long, default_value_t = 30000)]
        pause_ms: u64,
        #[arg(long)]
        crash_at: Option<String>,
        #[arg(long)]
        simulate_migration_failure: bool,
        #[arg(long)]
        skip_child_verify_exec: bool,
    },
    /// Roll back an upgraded Mini Tool installation to its previous verified version
    Rollback {
        #[arg(long)]
        install_root: PathBuf,
        #[arg(long)]
        restore_backup: bool,
        #[arg(long)]
        skip_child_verify_exec: bool,
    },
    /// Reconcile an interrupted update journal after OS crash or forced termination
    Reconcile {
        #[arg(long)]
        install_root: PathBuf,
        #[arg(long)]
        skip_child_verify_exec: bool,
    },
    /// Revoke a compromised or retired publisher Ed25519 key in local + external Trust Anchor
    RevokeKey {
        #[arg(long)]
        install_root: PathBuf,
        #[arg(long)]
        pubkey_hex: String,
        #[arg(long, default_value = "manual_revocation")]
        reason: String,
    },
    /// Audit entire installation security: Trust Triad + External Anchor + PE/Bundle/Manifests + All versions/
    VerifySecurity {
        #[arg(long)]
        install_root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::CheckCompat { release_dir } => {
            let verified = MiniToolUpdater::verify_candidate_release_dir(
                &release_dir,
                &RuntimeCompatibilityProfile::default(),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "COMPATIBLE",
                    "slug": verified.manifest.project.slug,
                    "version": verified.manifest.project.version,
                    "exeName": verified.manifest.project.exe_name,
                    "pePubkeyHex": verified.pe_pubkey_hex,
                    "compatibility": verified.manifest.compatibility,
                }))?
            );
        }
        Commands::InstallInitial {
            install_root,
            release_dir,
            skip_child_verify_exec,
        } => {
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.install_initial(
                &release_dir,
                &UpdateExecutionOptions {
                    skip_child_verify_exec,
                    ..Default::default()
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Upgrade {
            install_root,
            release_dir,
            pause_at,
            pause_ms,
            crash_at,
            simulate_migration_failure,
            skip_child_verify_exec,
        } => {
            let pause = match pause_at {
                Some(ref raw) => Some((FaultInjectionPoint::parse(raw)?, pause_ms)),
                None => None,
            };
            let crash = match crash_at {
                Some(ref raw) => Some(FaultInjectionPoint::parse(raw)?),
                None => None,
            };
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.upgrade_from_release(
                &release_dir,
                &UpdateExecutionOptions {
                    pause_after_stage_ms: pause,
                    crash_after_stage: crash,
                    simulate_migration_failure,
                    skip_child_verify_exec,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Rollback {
            install_root,
            restore_backup,
            skip_child_verify_exec,
        } => {
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.rollback_to_previous(restore_backup, skip_child_verify_exec)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Reconcile {
            install_root,
            skip_child_verify_exec,
        } => {
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.reconcile_incomplete_journal(skip_child_verify_exec)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::RevokeKey {
            install_root,
            pubkey_hex,
            reason,
        } => {
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.revoke_pubkey(&pubkey_hex, &reason)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::VerifySecurity { install_root } => {
            let updater = MiniToolUpdater::new(&install_root);
            let res = updater.verify_install_security_audit()?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
    }
    Ok(())
}
