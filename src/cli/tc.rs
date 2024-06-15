use std::fs;
use std::path::Path;

use anyhow::Result;
use clap::Parser;
use convert_case::{Case, Casing};
use dotenvy::dotenv;
use inquire::{Confirm, Select, Text};
use lazy_static::lazy_static;
use stripmargin::StripMargin;

use super::command::{Cli, Command};
use super::update_checker;
use crate::cli::fmt::Fmt;
use crate::cli::server::Server;
use crate::cli::{self, CLIError};
use crate::core::blueprint::Blueprint;
use crate::core::config::reader::ConfigReader;
use crate::core::generator::Generator;
use crate::core::http::API_URL_PREFIX;
use crate::core::print_schema;
use crate::core::rest::{EndpointSet, Unchecked};
const FILE_NAME: &str = ".tailcallrc.graphql";
const YML_FILE_NAME: &str = ".graphqlrc.yml";
const JSON_FILE_NAME: &str = ".tailcallrc.schema.json";

lazy_static! {
    static ref TRACKER: tailcall_tracker::Tracker = tailcall_tracker::Tracker::default();
}
pub async fn run() -> Result<()> {
    if let Ok(path) = dotenv() {
        tracing::info!("Env file: {:?} loaded", path);
    }
    let cli = Cli::parse();
    update_checker::check_for_update().await;
    let runtime = cli::runtime::init(&Blueprint::default());
    let config_reader = ConfigReader::init(runtime.clone());

    // Initialize ping event every 60 seconds
    let _ = TRACKER
        .init_ping(tokio::time::Duration::from_secs(60))
        .await;

    // Dispatch the command as an event
    let _ = TRACKER
        .dispatch(cli.command.to_string().to_case(Case::Snake).as_str())
        .await;
    match cli.command {
        Command::Start { file_paths } => {
            let config_module = config_reader.read_all(&file_paths).await?;
            log_endpoint_set(&config_module.extensions.endpoint_set);
            Fmt::log_n_plus_one(false, &config_module.config);
            let server = Server::new(config_module);
            server.fork_start().await?;
            Ok(())
        }
        Command::Check { file_paths, n_plus_one_queries, schema, format } => {
            let config_module = (config_reader.read_all(&file_paths)).await?;
            log_endpoint_set(&config_module.extensions.endpoint_set);
            if let Some(format) = format {
                Fmt::display(format.encode(&config_module)?);
            }
            let blueprint = Blueprint::try_from(&config_module).map_err(CLIError::from);

            match blueprint {
                Ok(blueprint) => {
                    tracing::info!("Config {} ... ok", file_paths.join(", "));
                    Fmt::log_n_plus_one(n_plus_one_queries, &config_module.config);
                    // Check the endpoints' schema
                    let _ = config_module
                        .extensions
                        .endpoint_set
                        .into_checked(&blueprint, runtime)
                        .await?;
                    if schema {
                        display_schema(&blueprint);
                    }
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        }
        Command::Init { folder_path } => init(&folder_path).await,
        Command::Gen { paths, input, output, query } => {
            let generator = Generator::init(runtime);
            let cfg = generator
                .read_all(input, paths.as_ref(), query.as_str())
                .await?;

            let config = output.unwrap_or_default().encode(&cfg)?;
            Fmt::display(config);
            Ok(())
        }
    }
}

pub async fn init(folder_path: &str) -> Result<()> {
    // 0. some common tasks
    let hello_world_graphql_config = r#"
schema @server(showcase: true) {
  query: Query
}

type User {
  id: Int!
  name: String!
}

type Query {
  Hello_World: User
    @http(
      path: "/users/1"
      baseURL: "http://jsonplaceholder.typicode.com"
    )
}
    "#;

    let hello_world_yaml_config = r#"
server:
  showcase: true
schema:
  query: Query
types:
  User:
    fields:
      id:
        type: Int
        required: true
      name:
        type: Int
        required: true
  Query:
    fields:
      Hello_World:
        type: User
        http:
          path: /users/1
          baseURL: http://jsonplaceholder.typicode.com
    "#;

    let hello_world_json_config = r#"
{
  "server": {
    "showcase": true
  },
  "schema": {
    "query": "Query"
  },
  "types": {
    "User": {
      "fields": {
        "id": {
          "type": "Int",
          "required": true
        },
        "name": {
          "type": "String",
          "required": true
        }
      }
    },
    "Query": {
      "fields": {
        "HelloWorld": {
          "type": "User",
          "http": {
            "path": "/users/1",
            "baseURL": "http://jsonplaceholder.typicode.com"
          }
        }
      }
    }
  }
}
    "#;

    // 1. Ask the project name
    let project_name = Text::new("Please enter the project name : ")
        .with_default(folder_path)
        .prompt()?;

    // 2. Ask the file format
    let file_format_options = vec!["Graphql", "Json", "Yaml"];
    let file_format =
        Select::new("Please select the file format : ", file_format_options).prompt()?;

    fn create_project_folder(project_name: String) -> Result<bool> {
        let folder_exists = fs::metadata(project_name.clone()).is_ok();

        if !folder_exists {
            let confirm = Confirm::new(&format!(
                "Do you want to create the folder {}?",
                project_name.clone()
            ))
            .with_default(false)
            .prompt()?;

            if confirm {
                fs::create_dir_all(project_name.clone())?;
                tracing::info!("Created project folder : '{}'", project_name.clone());
                return Ok(true);
            } else {
                return Ok(false);
            };
        } else {
            tracing::info!("project folder : '{}' already exists. Exiting.", project_name.clone());
            return Ok(false);
        }
    }

    // 3. generate helloworld config file
    match file_format {
        "Graphql" => {
            // Generate project_name.graphql in project_name
            if !create_project_folder(project_name.clone())? {
                return Ok(());
            }
            let hello_world_config_path =
                Path::new(project_name.clone().as_str()).join(project_name.clone() + ".graphql");
            fs::write(hello_world_config_path, hello_world_graphql_config)?;
            tracing::info!(
                "Generated file : '{}.graphql' in '{}'",
                project_name.clone(),
                project_name.clone()
            );
        }
        "Json" => {
            // Generate project_name.json in project_name
            if !create_project_folder(project_name.clone())? {
                return Ok(());
            }
            let hello_world_config_path =
                Path::new(project_name.clone().as_str()).join(project_name.clone() + ".json");
            fs::write(hello_world_config_path, hello_world_json_config)?;
            tracing::info!(
                "Generated file : '{}.json' in '{}'",
                project_name.clone(),
                project_name.clone()
            );
        }
        "Yaml" => {
            // Generate project_name.yml in project_name
            if !create_project_folder(project_name.clone())? {
                return Ok(());
            }
            let hello_world_config_path =
                Path::new(project_name.clone().as_str()).join(project_name.clone() + ".yaml");
            fs::write(hello_world_config_path, hello_world_yaml_config)?;
            tracing::info!(
                "Generated file : '{}.yaml' in '{}'",
                project_name.clone(),
                project_name.clone()
            );
        }
        _ => {}
    }

    let tailcallrc = include_str!("../../generated/.tailcallrc.graphql");
    let tailcallrc_json: &str = include_str!("../../generated/.tailcallrc.schema.json");

    let file_path = Path::new(&project_name.clone()).join(project_name.clone() + FILE_NAME);
    let json_file_path =
        Path::new(&project_name.clone()).join(project_name.clone() + JSON_FILE_NAME);
    let yml_file_path = Path::new(&project_name.clone()).join(project_name.clone() + YML_FILE_NAME);

    let tailcall_exists = fs::metadata(&file_path).is_ok();

    if tailcall_exists {
        // confirm overwrite
        let confirm = Confirm::new(&format!("Do you want to overwrite the file {}?", FILE_NAME))
            .with_default(false)
            .prompt()?;

        if confirm {
            fs::write(&file_path, tailcallrc.as_bytes())?;
            tracing::info!(
                "Generated file : '{}.tailcallrc.graphql' in '{}'",
                project_name.clone(),
                project_name.clone()
            );

            fs::write(&json_file_path, tailcallrc_json.as_bytes())?;
            tracing::info!(
                "Generated file : '{}.tailcallrc.schema.json' in '{}'",
                project_name.clone(),
                project_name.clone()
            );
        }
    } else {
        fs::write(&file_path, tailcallrc.as_bytes())?;
        tracing::info!(
            "Generated file : '{}.tailcallrc.graphql' in '{}'",
            project_name.clone(),
            project_name.clone()
        );

        fs::write(&json_file_path, tailcallrc_json.as_bytes())?;
        tracing::info!(
            "Generated file : '{}.tailcallrc.schema.json' in '{}'",
            project_name.clone(),
            project_name.clone()
        );
    }

    let yml_exists = fs::metadata(&yml_file_path).is_ok();

    if !yml_exists {
        fs::write(&yml_file_path, "")?;

        let graphqlrc = r"|schema:
         |- './.tailcallrc.graphql'
    "
        .strip_margin();

        fs::write(&yml_file_path, graphqlrc)?;
        tracing::info!(
            "Generated file : '{}.graphqlrc.yaml' in '{}'",
            project_name.clone(),
            project_name.clone()
        );
    }

    let graphqlrc = fs::read_to_string(&yml_file_path)?;

    let file_path = file_path.to_str().unwrap();

    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&graphqlrc)?;

    if let Some(mapping) = yaml.as_mapping_mut() {
        let schema = mapping
            .entry("schema".into())
            .or_insert(serde_yaml::Value::Sequence(Default::default()));
        if let Some(schema) = schema.as_sequence_mut() {
            if !schema
                .iter()
                .any(|v| v == &serde_yaml::Value::from("./.tailcallrc.graphql"))
            {
                let confirm =
                    Confirm::new(&format!("Do you want to add {} to the schema?", file_path))
                        .with_default(false)
                        .prompt()?;

                if confirm {
                    schema.push(serde_yaml::Value::from("./.tailcallrc.graphql"));
                    let updated = serde_yaml::to_string(&yaml)?;
                    fs::write(yml_file_path, updated)?;
                    tracing::info!(
                        "Modified file : '{}.graphqlrc.yaml' in '{}'",
                        project_name.clone(),
                        project_name.clone()
                    );
                }
            }
        }
    }

    Ok(())
}

fn log_endpoint_set(endpoint_set: &EndpointSet<Unchecked>) {
    let mut endpoints = endpoint_set.get_endpoints().clone();
    endpoints.sort_by(|a, b| {
        let method_a = a.get_method();
        let method_b = b.get_method();
        if method_a.eq(method_b) {
            a.get_path().as_str().cmp(b.get_path().as_str())
        } else {
            method_a.to_string().cmp(&method_b.to_string())
        }
    });
    for endpoint in endpoints {
        tracing::info!(
            "Endpoint: {} {}{} ... ok",
            endpoint.get_method(),
            API_URL_PREFIX,
            endpoint.get_path().as_str()
        );
    }
}

pub fn display_schema(blueprint: &Blueprint) {
    Fmt::display(Fmt::heading("GraphQL Schema:\n"));
    let sdl = blueprint.to_schema();
    Fmt::display(format!("{}\n", print_schema::print_schema(sdl)));
}
