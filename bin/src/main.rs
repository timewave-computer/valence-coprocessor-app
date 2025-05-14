use std::{fs, net::SocketAddr, path::PathBuf, process::Command as Cmd};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use clap::{arg, command, Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Socket address of the co-processor.
    #[arg(short, long, value_name = "SOCKET", default_value = "127.0.0.1:37281")]
    socket: SocketAddr,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the co-processor service
    Coprocessor,

    /// Deploys definitions to the co-processor
    #[command(subcommand)]
    Deploy(CmdDeploy),

    /// Submits a proof request to the co-processor
    Prove {
        /// ID of the deployed program
        #[arg(value_name = "PROGRAM")]
        program: String,

        /// Optional JSON argument to be passed to the program
        #[arg(short, long, value_name = "JSON")]
        json: Option<String>,

        /// Path to store the proof on the virtual filesystem
        #[arg(
            short,
            long,
            value_name = "PATH",
            default_value = "/var/share/proof.bin"
        )]
        path: PathBuf,
    },

    /// Reads a file from the storage, returning its base64 data
    Storage {
        /// ID of the deployed program
        #[arg(value_name = "PROGRAM")]
        program: String,

        /// Path to the file on the virtual filesystem
        #[arg(
            short,
            long,
            value_name = "PATH",
            default_value = "/var/share/proof.bin"
        )]
        path: PathBuf,
    },

    /// Returns the VK of a program
    Vk {
        /// ID of the deployed program
        #[arg(value_name = "PROGRAM")]
        program: String,
    },
}

#[derive(Subcommand)]
enum CmdDeploy {
    /// Deploys the domain definition to the co-processor
    Domain {
        /// Name of the domain to be deployed
        #[arg(short, long, value_name = "NAME")]
        name: String,
    },

    /// Deploys the program definition to the co-processor
    Program {
        /// Nonce of the deployed program. Used to compute the ID
        #[arg(short, long, default_value_t = 0)]
        nonce: u64,
    },
}

fn main() -> anyhow::Result<()> {
    let Cli { socket, cmd } = Cli::parse();

    match cmd {
        Commands::Coprocessor => {
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .canonicalize()?;

            anyhow::ensure!(Cmd::new("docker")
                .current_dir(&base)
                .args(["build", "-t", "coprocessor:0.1.0", "./docker/coprocessor"])
                .status()?
                .success());

            anyhow::ensure!(Cmd::new("docker")
                .current_dir(&base)
                .args([
                    "run",
                    "--rm",
                    "-it",
                    "--init",
                    "-p",
                    "37281:37281",
                    "coprocessor:0.1.0"
                ])
                .status()?
                .success());
        }

        Commands::Deploy(c) => match c {
            CmdDeploy::Domain { name } => {
                let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .canonicalize()?;

                anyhow::ensure!(Cmd::new("docker")
                    .current_dir(&base)
                    .args([
                        "build",
                        "-t",
                        "valence-coprocessor-app:0.1.0",
                        "./docker/deploy"
                    ])
                    .status()?
                    .success());

                anyhow::ensure!(Cmd::new("docker")
                    .current_dir(&base)
                    .args([
                        "run",
                        "--rm",
                        "-it",
                        "-v",
                        format!("{}:/usr/src/app", base.display()).as_str(),
                        "valence-coprocessor-app:0.1.0",
                        "cargo",
                        "build",
                        "--target",
                        "wasm32-unknown-unknown",
                        "--release",
                        "--manifest-path",
                        "./docker/build/domain-wasm/Cargo.toml"
                    ])
                    .status()?
                    .success());

                let path = base
                    .join("docker")
                    .join("build")
                    .join("domain-wasm")
                    .join("target")
                    .join("wasm32-unknown-unknown")
                    .join("release")
                    .join("valence_coprocessor_app_domain_wasm.wasm");

                let bytes = fs::read(path)?;
                let lib = Base64.encode(bytes);
                let uri = format!("http://{socket}/api/registry/domain");

                let response = reqwest::blocking::Client::new()
                    .post(uri)
                    .json(&json!({
                        "name": name,
                        "lib": lib,
                    }))
                    .send()?
                    .json::<Value>()?
                    .get("domain")
                    .ok_or_else(|| anyhow::anyhow!("no data received"))?
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
                    .to_string();

                println!("{response}");
            }

            CmdDeploy::Program { nonce } => {
                let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .canonicalize()?;

                anyhow::ensure!(Cmd::new("docker")
                    .current_dir(&base)
                    .args([
                        "build",
                        "-t",
                        "valence-coprocessor-app:0.1.0",
                        "./docker/deploy"
                    ])
                    .status()?
                    .success());

                anyhow::ensure!(Cmd::new("docker")
                    .current_dir(&base)
                    .args([
                        "run",
                        "--rm",
                        "-it",
                        "-v",
                        format!("{}:/usr/src/app", base.display()).as_str(),
                        "valence-coprocessor-app:0.1.0",
                        "cargo",
                        "build",
                        "--target",
                        "wasm32-unknown-unknown",
                        "--release",
                        "--manifest-path",
                        "./docker/build/program-wasm/Cargo.toml"
                    ])
                    .status()?
                    .success());

                anyhow::ensure!(Cmd::new("docker")
                    .current_dir(&base)
                    .args([
                        "run",
                        "--rm",
                        "-it",
                        "-v",
                        format!("{}:/usr/src/app", base.display()).as_str(),
                        "valence-coprocessor-app:0.1.0"
                    ])
                    .status()?
                    .success());

                let build = base.join("docker").join("build");

                let wasm = build
                    .join("program-wasm")
                    .join("target")
                    .join("wasm32-unknown-unknown")
                    .join("release")
                    .join("valence_coprocessor_app_program_wasm.wasm");

                let elf = build
                    .join("program-circuit")
                    .join("target")
                    .join("program.elf");

                let wasm = fs::read(wasm)?;
                let elf = fs::read(elf)?;
                let uri = format!("http://{socket}/api/registry/program");

                let lib = Base64.encode(wasm);
                let circuit = Base64.encode(elf);

                let response = reqwest::blocking::Client::new()
                    .post(uri)
                    .json(&json!({
                        "lib": lib,
                        "circuit": circuit,
                        "nonce": nonce,
                    }))
                    .send()?
                    .json::<Value>()?
                    .get("program")
                    .ok_or_else(|| anyhow::anyhow!("no data received"))?
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
                    .to_string();

                println!("{response}");
            }
        },

        Commands::Prove {
            program,
            json,
            path,
        } => {
            let args: Value = match json {
                Some(a) => serde_json::from_str(&a)?,
                None => Value::Null,
            };
            let uri = format!("http://{socket}/api/registry/program/{program}/prove");

            let response = reqwest::blocking::Client::new()
                .post(uri)
                .json(&json!({
                    "args": args,
                    "payload": {
                        "cmd": "store",
                        "path": path
                    }
                }))
                .send()?
                .text()?;

            println!("{response}");
        }

        Commands::Storage { program, path } => {
            let uri = format!("http://{socket}/api/registry/program/{program}/storage/fs");

            let response = reqwest::blocking::Client::new()
                .post(uri)
                .json(&json!({
                    "path": path
                }))
                .send()?
                .json::<Value>()?
                .get("data")
                .ok_or_else(|| anyhow::anyhow!("no data received"))?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
                .to_string();

            println!("{response}");
        }

        Commands::Vk { program } => {
            let uri = format!("http://{socket}/api/registry/program/{program}/vk");

            let response = reqwest::blocking::Client::new()
                .get(uri)
                .send()?
                .json::<Value>()?
                .get("base64")
                .ok_or_else(|| anyhow::anyhow!("no data received"))?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
                .to_string();

            println!("{response}");
        }
    }

    Ok(())
}
