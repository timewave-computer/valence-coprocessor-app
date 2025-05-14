use std::{fs, net::SocketAddr, path::PathBuf, process::Command as Cmd};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use clap::{arg, command, value_parser, Command};
use serde_json::{json, Value};

fn main() -> anyhow::Result<()> {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            arg!([SOCKET])
                .value_parser(value_parser!(SocketAddr))
                .default_value("127.0.0.1:37281"),
        )
        .subcommand(
            Command::new("deploy")
                .about("Deploys definitions to the co-processor")
                .subcommand(
                    Command::new("domain")
                        .about("Deploys the domain definition to the co-processor")
                        .arg(arg!([NAME])),
                )
                .subcommand(
                    Command::new("program")
                        .about("Deploys the program definition to the co-processor")
                        .arg(
                            arg!([NONCE])
                                .value_parser(value_parser!(u64))
                                .default_value("0"),
                        ),
                ),
        )
        .subcommand(
            Command::new("prove")
                .about("submits a proof request to the co-processor.")
                .arg(arg!([PROGRAM]))
                .arg(arg!([JSON]))
                .arg(arg!([PATH])),
        )
        .subcommand(
            Command::new("storage")
                .about("reads a file from the storage, returning its base64 data.")
                .arg(arg!([PROGRAM]))
                .arg(arg!([PATH])),
        )
        .subcommand(
            Command::new("vk")
                .about("returns the VK of a program")
                .arg(arg!([PROGRAM])),
        )
        .get_matches();

    let socket = matches.get_one::<SocketAddr>("SOCKET").unwrap();

    match matches.subcommand() {
        Some(("deploy", m)) => match m.subcommand() {
            Some(("domain", m)) => {
                let name = m.get_one::<String>("NAME").unwrap();

                anyhow::ensure!(Cmd::new("make").arg("domain").status()?.success());

                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
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

            Some(("program", m)) => {
                let nonce = m.get_one::<u64>("NONCE").unwrap();

                anyhow::ensure!(Cmd::new("make").arg("program").status()?.success());
                anyhow::ensure!(Cmd::new("make").arg("circuit").status()?.success());

                let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("docker")
                    .join("build");

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

            _ => unreachable!(),
        },

        Some(("prove", m)) => {
            let program = m.get_one::<String>("PROGRAM").unwrap();
            let args = m.get_one::<String>("JSON").unwrap();
            let path = m.get_one::<String>("PATH").unwrap();
            let args: Value = serde_json::from_str(&args)?;
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

        Some(("storage", m)) => {
            let program = m.get_one::<String>("PROGRAM").unwrap();
            let path = m.get_one::<String>("PATH").unwrap();
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

        Some(("vk", m)) => {
            let program = m.get_one::<String>("PROGRAM").unwrap();
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

        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }

    Ok(())
}
