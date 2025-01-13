// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use config::utils::file::set_permission;
use infra::{file_list as infra_file_list, table};

use crate::{
    cli::data::{
        cli::{args as dataArgs, Cli as dataCli},
        export, import, Context,
    },
    common::{infra::config::USERS, meta, migration},
    service::{compact, db, file_list, users},
};

pub async fn cli() -> Result<bool, anyhow::Error> {
    let app = clap::Command::new("openobserve")
        .version(env!("GIT_VERSION"))
        .about(clap::crate_description!())
        .subcommands(&[
            clap::Command::new("reset")
                .about("reset openobserve data")
                .arg(
                    clap::Arg::new("component")
                        .short('c')
                        .long("component")
                        .help(
                            "reset data of the component: root, user, alert, dashboard, function, stream-stats",
                        ),
                ),
            clap::Command::new("import")
                .about("import openobserve data").args(dataArgs()),
            clap::Command::new("export")
                .about("export openobserve data").args(dataArgs()),
            clap::Command::new("view")
                .about("view openobserve data")
                .arg(
                    clap::Arg::new("component")
                        .short('c')
                        .long("component")
                        .help("view data of the component: version, user"),
                ),
            clap::Command::new("init-dir")
                .about("init openobserve data dir")
                .arg(
                    clap::Arg::new("path")
                        .short('p')
                        .long("path")
                        .help("init this path as data root dir"),
                ),
            clap::Command::new("migrate-file-list")
                .about("migrate file-list")
                .args([
                    clap::Arg::new("prefix")
                        .short('p')
                        .long("prefix")
                        .value_name("prefix")
                        .required(false)
                        .help("only migrate specified prefix, default is all"),
                    clap::Arg::new("from")
                        .short('f')
                        .long("from")
                        .value_name("from")
                        .required(true)
                        .help("migrate from: sled, sqlite, etcd, mysql, postgresql"),
                    clap::Arg::new("to")
                        .short('t')
                        .long("to")
                        .value_name("to")
                        .required(true)
                        .help("migrate to: sqlite, mysql, postgresql"),
                ]),
            clap::Command::new("migrate-meta")
                .about("migrate meta")
                .args([
                    clap::Arg::new("from")
                        .short('f')
                        .long("from")
                        .value_name("from")
                        .required(true)
                        .help("migrate from: sled, sqlite, etcd, mysql, postgresql"),
                    clap::Arg::new("to")
                        .short('t')
                        .long("to")
                        .value_name("to")
                        .required(true)
                        .help("migrate to: sqlite, etcd, mysql, postgresql"),
                ]),
            clap::Command::new("migrate-dashboards").about("migrate-dashboards"),
            clap::Command::new("migrate-pipeline").about("migrate pipelines")
                .arg(
                    clap::Arg::new("drop-table")
                        .long("drop-table")
                        .required(false)
                        .num_args(0)
                        .help("Drop existing Pipeline table first before migrating")
                ),
            clap::Command::new("delete-parquet")
                .about("delete parquet files from s3 and file_list")
                .arg(
                    clap::Arg::new("file")
                        .short('f')
                        .long("file")
                        .value_name("file")
                        .help("the parquet file name"),
                ),
            clap::Command::new("migrate-schemas").about("migrate from single row to row per schema version"),
            clap::Command::new("seaorm-rollback").about("rollback SeaORM migration steps")
                .subcommand(
                    clap::Command::new("all")
                    .about("rollback all SeaORM migration steps")
                )
                .subcommand(
                    clap::Command::new("last")
                    .about("rollback last N SeaORM migration steps")
                    .arg(clap::Arg::new("N").help("number of migration steps to rollback (default is 1)").value_parser(clap::value_parser!(u32)))
                ),
        ])
        .get_matches();

    if app.subcommand().is_none() {
        return Ok(false);
    }
    let cfg = config::get_config();
    #[cfg(not(feature = "tokio-console"))]
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("INFO"));

    let (name, command) = app.subcommand().unwrap();
    if name == "init-dir" {
        match command.get_one::<String>("path") {
            Some(path) => {
                set_permission(path, 0o777)?;
                println!("init dir {} successfully", path);
            }
            None => {
                return Err(anyhow::anyhow!("please set data path"));
            }
        }
        return Ok(true);
    }

    // init infra, create data dir & tables
    infra::init().await.expect("infra init failed");
    match name {
        "reset" => {
            let component = command.get_one::<String>("component").unwrap();
            match component.as_str() {
                "root" => {
                    let _ = users::update_user(
                        meta::organization::DEFAULT_ORG,
                        cfg.auth.root_user_email.as_str(),
                        false,
                        cfg.auth.root_user_email.as_str(),
                        meta::user::UpdateUser {
                            change_password: true,
                            old_password: None,
                            new_password: Some(cfg.auth.root_user_password.clone()),
                            role: Some(crate::common::meta::user::UserRoleRequest {
                                role: config::meta::user::UserRole::Root.to_string(),
                                custom: None,
                            }),
                            first_name: Some("root".to_owned()),
                            last_name: Some("".to_owned()),
                            token: None,
                        },
                    )
                    .await?;
                }
                "user" => {
                    db::user::reset().await?;
                }
                "alert" => {
                    db::alerts::alert::reset().await?;
                }
                "dashboard" => {
                    table::dashboards::delete_all().await?;
                }
                "report" => {
                    db::dashboards::reports::reset().await?;
                }
                "function" => {
                    db::functions::reset().await?;
                }
                "stream-stats" => {
                    // reset stream stats update offset
                    db::compact::stats::set_offset(0, None).await?;
                    // reset stream stats table data
                    infra_file_list::reset_stream_stats().await?;
                    // load stream list
                    db::schema::cache().await?;
                    // update stats from file list
                    compact::stats::update_stats_from_file_list()
                        .await
                        .expect("file list remote calculate stats failed");
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "unsupported reset component: {}",
                        component
                    ));
                }
            }
        }
        "view" => {
            let component = command.get_one::<String>("component").unwrap();
            match component.as_str() {
                "version" => {
                    println!("version: {}", db::version::get().await?);
                }
                "user" => {
                    db::user::cache().await?;
                    db::org_users::cache().await?;
                    let mut id = 0;
                    for user in USERS.iter() {
                        id += 1;
                        println!("{id}\t{:?}\n{:?}", user.key(), user.value());
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!("unsupported reset component: {component}"));
                }
            }
        }
        "migrate-file-list" => {
            let from = match command.get_one::<String>("from") {
                Some(from) => from.to_string(),
                None => "".to_string(),
            };
            let to = match command.get_one::<String>("to") {
                Some(to) => to.to_string(),
                None => "".to_string(),
            };
            println!("Running migration file_list from {} to {}", from, to);
            migration::file_list::run(&from, &to).await?;
        }
        "migrate-meta" => {
            let from = match command.get_one::<String>("from") {
                Some(from) => from.to_string(),
                None => "".to_string(),
            };
            let to = match command.get_one::<String>("to") {
                Some(to) => to.to_string(),
                None => "".to_string(),
            };
            println!("Running migration metadata from {} to {}", from, to);
            migration::meta::run(&from, &to).await?
        }
        "migrate-dashboards" => {
            println!("Running migration dashboard");
            migration::dashboards::run().await?
        }
        "migrate-pipeline" => {
            println!("Running migration pipeline");
            let drop_table = command.get_flag("drop-table");
            migration::pipeline_func::run(drop_table).await?;
        }
        "delete-parquet" => {
            let file = command.get_one::<String>("file").unwrap();
            match file_list::delete_parquet_file(file, true).await {
                Ok(_) => {
                    println!("delete parquet file {} successfully", file);
                }
                Err(e) => {
                    println!("delete parquet file {} failed, error: {}", file, e);
                }
            }
        }
        "import" => {
            import::Import::operator(dataCli::arg_matches(command.clone())).await?;
        }
        "export" => {
            export::Export::operator(dataCli::arg_matches(command.clone())).await?;
        }
        "migrate-schemas" => {
            println!("Running schema migration to row per schema version");
            #[allow(deprecated)]
            migration::schema::run().await?
        }
        "seaorm-rollback" => match command.subcommand() {
            Some(("all", _)) => {
                println!("Rolling back all");
                infra::table::down(None).await?
            }
            Some(("last", sub_matches)) => {
                let n = sub_matches
                    .get_one::<u32>("N")
                    .map(|n| n.to_owned())
                    .unwrap_or(1);
                println!("Rolling back {n}");
                infra::table::down(Some(n)).await?
            }
            Some((name, _)) => {
                return Err(anyhow::anyhow!("unsupported sub command: {name}"));
            }
            None => {
                return Err(anyhow::anyhow!("missing sub command"));
            }
        },
        _ => {
            return Err(anyhow::anyhow!("unsupported sub command: {name}"));
        }
    }

    // flush db
    let db = infra::db::get_db().await;
    if let Err(e) = db.close().await {
        log::error!("waiting for db close failed, error: {}", e);
    }

    println!("command {name} execute successfully");
    Ok(true)
}
