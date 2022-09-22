use std::convert::TryInto;
use std::panic::PanicInfo;

use clap::{crate_version, Arg, Command};
use ko_backend::BackendImpl;
use ko_context::ContextMgr;
use ko_protocol::{log, secp256k1::SecretKey, tokio, KoResult, Logger, ProjectDeps};
use ko_rpc::RpcServerRuntime;
use ko_rpc_client::RpcClient;

const PROJECT_TYPE_ARGS_TOML: &str = ".project_type_args.toml";

#[tokio::main]
async fn main() -> KoResult<()> {
    // initail Command line options
    let matches = Command::new("knside-out")
        .version(crate_version!())
        .arg(
            Arg::new("config_path")
                .short('c')
                .long("config")
                .help("Knside-out config path")
                .required(true)
                .takes_value(true),
        )
        .subcommand(Command::new("run").about("Run knside-out process"))
        .get_matches();

    // initail log system
    log::set_boxed_logger(Box::new(Logger))
        .map(|_| log::set_max_level(log::LevelFilter::Info))
        .expect("logger");

    let config_path = matches.value_of("config_path").unwrap();
    let config = ko_config::load_file(config_path)?;
    let config_type_args = ko_config::load_type_args_file(PROJECT_TYPE_ARGS_TOML)?;
    let project_deps: &ProjectDeps = &config.as_ref().try_into().expect("config");

    // initail CKB rcp client
    let rpc_client = RpcClient::new(&config.ckb_url, &config.ckb_indexer_url);

    // initail driver context manager
    let manager_private_key =
        SecretKey::from_slice(config.project_manager_privkey.as_bytes()).expect("private key");
    let mut context_mgr = ContextMgr::new(
        &rpc_client,
        &manager_private_key,
        project_deps,
        &config.drive_settings,
    );
    context_mgr.recover_contexts(config_type_args.into()).await;

    // backup loop for persisting contexts status into project_type_args toml file
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(config.persist_interval_sec)) => {
                    let contexts_status = ContextMgr::<RpcClient>::dump_contexts_status().await;
                    ko_config::save_type_args_file(contexts_status, PROJECT_TYPE_ARGS_TOML)
                        .expect("save config");
                }
            }
        }
    });

    // initail rpc backend
    let backend = BackendImpl::new(&rpc_client, context_mgr);

    // start rpc server
    RpcServerRuntime::run(&config.rpc_endpoint, backend, project_deps).await?;

    // wait abort signal to exit
    let ctrl_c_handler = tokio::spawn(async {
        #[cfg(windows)]
        let _ = tokio::signal::ctrl_c().await;
        #[cfg(unix)]
        {
            use tokio::signal::unix as os_impl;
            let mut sigtun_int = os_impl::signal(os_impl::SignalKind::interrupt()).unwrap();
            let mut sigtun_term = os_impl::signal(os_impl::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigtun_int.recv() => {}
                _ = sigtun_term.recv() => {}
            };
        }
    });

    // register channel of panic
    let (panic_sender, mut panic_receiver) = tokio::sync::mpsc::channel(1);

    std::panic::set_hook(Box::new(move |info: &PanicInfo| {
        panic_sender
            .try_send(info.to_string())
            .expect("panic_receiver is droped");
    }));

    tokio::select! {
        _ = ctrl_c_handler => {
            log::warn!("ctrl-c is pressed, quit knside-out")
        }
        Some(panic_info) = panic_receiver.recv() => {
            log::warn!("child thread paniced: {}", panic_info)
        }
    }

    Ok(())
}
