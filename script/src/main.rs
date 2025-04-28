use std::{
    fs,
    io::{self, Read as _, Write as _},
    net::SocketAddr,
    path::PathBuf,
    process::Command,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as Base64};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use sp1_sdk::{Prover as _, ProverClient, SP1ProofWithPublicValues, SP1VerifyingKey};
use valence_coprocessor::ProgramData;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Co-processor endpoint RPC address.
    #[arg(short, long, default_value = "127.0.0.1:37281")]
    rpc: SocketAddr,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploys the application to the provided endpoint
    Deploy,

    /// Proves a deployed program.
    Prove {
        /// JSON arguments.
        #[arg(short, long)]
        args: Option<String>,
    },

    /// Verifies a proof.
    Verify {
        /// JSON arguments.
        #[arg(short, long)]
        args: Option<String>,

        /// zkVM mode.
        #[arg(short, long)]
        mode: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { rpc, command } = Cli::parse();

    let url = format!("http://{rpc}/api");
    let client = reqwest::Client::new();

    match command {
        Commands::Deploy => {
            let lib = build_lib()?;
            let circuit = build_circuit()?;

            let lib = Base64.encode(lib);
            let circuit = Base64.encode(circuit);

            let program = client
                .post(format!("{url}/registry/program"))
                .json(&json!({
                    "lib": lib,
                    "circuit": circuit
                }))
                .send()
                .await?
                .json::<Value>()
                .await?["program"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("inconsistent reponse"))?
                .to_string();

            eprintln!("registered program `{program}`...");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "program": program
                }))?
            );
        }

        Commands::Prove { args } => {
            let program = get_program()?;
            let args = get_args(&args)?;

            let proof = client
                .post(format!("{url}/registry/program/{program}/prove"))
                .json(&json!({
                    "args": args
                }))
                .send()
                .await?
                .json::<Value>()
                .await?;

            println!("{}", serde_json::to_string_pretty(&proof)?);
        }

        Commands::Verify { args, mode } => {
            let program = get_program()?;
            let args = get_args(&args)?;
            let log = args.get("log").cloned().unwrap_or_default();

            let vk = client
                .get(format!("{url}/registry/program/{program}/vk"))
                .send()
                .await?
                .json::<Value>()
                .await?["base64"]
                .as_str()
                .map(|vk| Base64.decode(vk))
                .ok_or_else(|| anyhow::anyhow!("failed to get base64"))??;

            let proof = args
                .get("proof")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("proof argument not provided"))?;
            let proof = Base64.decode(proof)?;
            let mut proof: SP1ProofWithPublicValues = bincode::deserialize(&proof)?;
            let vk: SP1VerifyingKey = bincode::deserialize(&vk)?;

            match mode.as_deref() {
                Some("mock") | None => {
                    ProverClient::builder().mock().build().verify(&proof, &vk)?
                }
                Some("cpu") => ProverClient::builder().cpu().build().verify(&proof, &vk)?,
                Some("cuda") => ProverClient::builder().cuda().build().verify(&proof, &vk)?,
                Some("network") => ProverClient::builder()
                    .network()
                    .build()
                    .verify(&proof, &vk)?,
                v => anyhow::bail!("unknown mode {v:?}"),
            }

            let output: String = proof.public_values.read();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "output": output,
                    "log": log,
                }))?
            )
        }
    }

    Ok(())
}

fn build_lib() -> anyhow::Result<Vec<u8>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("error navigating to manifest"))?
        .canonicalize()?;

    eprintln!("building library...");

    let build = Command::new("cargo")
        .current_dir(&manifest)
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--package",
            "valence-coprocessor-app-lib",
        ])
        .output()?;

    io::stderr().write_all(&build.stderr)?;
    io::stderr().write_all(&build.stdout)?;
    anyhow::ensure!(build.status.success(), "failed to build library!");

    let lib = manifest
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("valence_coprocessor_app_lib.wasm");

    anyhow::ensure!(lib.is_file(), "lib not found!");

    Ok(fs::read(lib)?)
}

fn build_circuit() -> anyhow::Result<Vec<u8>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("error navigating to manifest"))?
        .canonicalize()?;

    let assets = manifest.join("assets");
    let zkvm = manifest.join("zkvm").join("circuit");
    let script = manifest.join("zkvm").join("script").join("Cargo.toml");
    let circuit = assets.join("demo.elf");

    eprintln!("building circuit...");

    let build = Command::new("cargo")
        .current_dir(&zkvm)
        .args(["prove", "build"])
        .output()?;

    io::stderr().write_all(&build.stderr)?;
    io::stderr().write_all(&build.stdout)?;
    anyhow::ensure!(build.status.success(), "failed to build circuit!");

    let build = Command::new("cargo")
        .current_dir(&manifest)
        .args([
            "run",
            "--manifest-path",
            format!("{}", script.display()).as_str(),
            "--",
            format!("{}", circuit.display()).as_str(),
        ])
        .output()?;

    io::stderr().write_all(&build.stderr)?;
    io::stderr().write_all(&build.stdout)?;
    anyhow::ensure!(build.status.success(), "failed to build pk!");

    anyhow::ensure!(circuit.is_file(), "circuit not found!");

    Ok(fs::read(circuit)?)
}

fn get_program() -> anyhow::Result<String> {
    let circuit = build_circuit()?;
    let program = ProgramData::identifier_from_parts(&circuit, 0);

    Ok(hex::encode(program))
}

fn get_args(args: &Option<String>) -> anyhow::Result<Value> {
    match args {
        Some(a) => Ok(serde_json::from_str(a)?),
        None => {
            let mut args = String::new();

            io::stdin().read_to_string(&mut args)?;

            Ok(serde_json::from_str(&args)?)
        }
    }
}
