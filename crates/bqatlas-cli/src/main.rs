//! Portable source starter and a first directory-resource scaffold.
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
include!(concat!(env!("OUT_DIR"), "/starter.rs"));
#[derive(Parser)]
#[command(
    version,
    about = "Create bqAtlas Rust applications and directory resources"
)]
struct Options {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Create an independent full-stack application. Existing paths are never overwritten.
    New {
        directory: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    /// Scaffold a code/name/contact-email/active directory from a resource specification.
    Directory {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long, default_value = ".")]
        app: PathBuf,
        #[arg(long)]
        wire: bool,
    },
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DirectorySpec {
    module: String,
    resource: String,
    entity: String,
    title: String,
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().enumerate().all(|(i, c)| {
            if i == 0 {
                c.is_ascii_lowercase()
            } else {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'
            }
        })
}
fn file(path: &str) -> Result<&'static [u8], String> {
    FILES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, data)| *data)
        .ok_or_else(|| format!("Starter file missing: {path}"))
}
fn source(path: &str) -> Result<String, String> {
    String::from_utf8(file(path)?.to_vec()).map_err(|_| "Invalid template source".into())
}
fn new_app(target: &Path, name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    if target.exists() {
        return Err("Target already exists; choose a new directory".into());
    }
    let name = name
        .or_else(|| {
            target
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
        })
        .ok_or("Project name is required")?;
    if name.len() > 64
        || name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("Use a project name containing letters, digits, hyphens or underscores".into());
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".bqatlas-staging-{}", Uuid::new_v4().simple()));
    fs::create_dir(&staging)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        for (name, data) in FILES {
            let path = staging.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, data)?;
        }
        let pkg = staging.join("samples/crm/web/package.json");
        let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&pkg)?)?;
        json["name"] = serde_json::json!(format!("{}-web", name.to_lowercase()));
        fs::write(pkg, serde_json::to_vec_pretty(&json)?)?;
        let lock = staging.join("samples/crm/web/package-lock.json");
        let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&lock)?)?;
        json["name"] = serde_json::json!(format!("{}-web", name.to_lowercase()));
        json["packages"][""]["name"] = json["name"].clone();
        fs::write(lock, serde_json::to_vec_pretty(&json)?)?;
        let compose = staging.join("dev/compose.yaml");
        fs::write(
            &compose,
            fs::read_to_string(&compose)?.replace(
                "name: bqatlas-rust-dev",
                &format!("name: {}-dev", name.to_lowercase()),
            ),
        )?;
        let readme = staging.join("README.md");
        fs::write(
            &readme,
            fs::read_to_string(&readme)?.replacen("# bqAtlas Rust", &format!("# {name}"), 1),
        )?;
        fs::rename(&staging, target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;
    println!(
        "Created {}. Copy .env.example to .env, then follow README.md. No services or accounts were created.",
        target.display()
    );
    Ok(())
}
fn replace_once(text: String, old: &str, new: &str) -> Result<String, String> {
    if text.matches(old).count() != 1 {
        return Err("Application composition differs from this template; wire the generated module manually".into());
    }
    Ok(text.replacen(old, new, 1))
}
fn directory(app: &Path, spec_path: &Path, wire: bool) -> Result<(), Box<dyn std::error::Error>> {
    let spec: DirectorySpec = serde_json::from_slice(&fs::read(spec_path)?)?;
    if !identifier(&spec.module)
        || !identifier(&spec.resource)
        || spec.entity.is_empty()
        || !spec.entity.bytes().enumerate().all(|(i, c)| {
            if i == 0 {
                c.is_ascii_uppercase()
            } else {
                c.is_ascii_alphanumeric()
            }
        })
        || spec.title.trim().is_empty()
        || spec.title.len() > 100
        || spec.title.chars().any(char::is_control)
    {
        return Err("Invalid directory resource specification".into());
    }
    if matches!(
        spec.module.as_str(),
        "identity" | "crm" | "sales" | "engagement" | "bqatlas"
    ) {
        return Err("Choose a new module ID; existing modules are not overwritten".into());
    }
    let target = app.join("samples/crm/modules").join(&spec.module);
    if target.exists() {
        return Err("Module already exists".into());
    }
    let package = format!(
        "bqatlas-{}-{}",
        spec.module.replace('_', "-"),
        spec.resource.replace('_', "-")
    );
    let alias = package.replace('-', "_");
    let transform = |text: String| {
        text.replace(
            "crm.customers",
            &format!("{}.{}", spec.module, spec.resource),
        )
        .replace(
            "/crm/customers",
            &format!("/{}/{}", spec.module, spec.resource),
        )
        .replace(
            "crm.customers",
            &format!("{}.{}", spec.module, spec.resource),
        )
        .replace("crm.", &format!("{}.", spec.module))
        .replace("[crm]", &format!("[{}]", spec.module))
        .replace("SCHEMA crm", &format!("SCHEMA {}", spec.module))
        .replace("'crm'", &format!("'{}'", spec.module))
        .replace("\"crm\"", &format!("\"{}\"", spec.module))
        .replace("Customer", &spec.entity)
        .replace("customers", &spec.resource)
        .replace("Customers", &spec.title)
    };
    let mut lib = source("samples/crm/modules/crm/src/lib.rs")?;
    lib = lib.replace(
        "#[async_trait]\nimpl CustomerDirectory for CustomerService",
        "impl CustomerService",
    );
    lib = lib.replace("use async_trait::async_trait;\n", "").replace(
        "use bqatlas_sample_crm_contracts::{CUSTOMER_LOOKUP, CustomerDirectory, CustomerSummary};",
        &format!(
            "const CUSTOMER_LOOKUP: &str = {:?};\n#[derive(serde::Serialize)] struct CustomerSummary {{id:Uuid,code:String,name:String,active:bool}}",
            format!("{}.{}.lookup", spec.module, spec.resource)
        ),
    );
    lib = transform(lib);
    lib = lib.replace(
        "//! CRM customer module. Its persistence is private; consumers use CustomerDirectory.",
        &format!(
            "//! {} directory module. Extend its domain rules for your application.",
            spec.title
        ),
    );
    // Human-facing title and the fixed resource set URL are independent of the singular entity name.
    lib = lib.replace(
        &format!("title: {:?}.into()", format!("{}s", spec.entity)),
        &format!("title: {:?}.into()", spec.title),
    );
    lib = lib.replace(
        &format!("/odata/{}s", spec.entity),
        &format!("/odata/{}", spec.title.replace(' ', "")),
    );
    let mut manifest = source("samples/crm/modules/crm/Cargo.toml")?.replace(
        "name = \"bqatlas-sample-crm\"",
        &format!("name = {:?}", package),
    );
    manifest = manifest
        .lines()
        .filter(|line| {
            !line.starts_with("bqatlas-sample-crm-contracts")
                && !line.starts_with("async-trait.workspace")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut changes = vec![];
    if wire {
        let host = app.join("samples/crm/server/src/lib.rs");
        let mut text = fs::read_to_string(&host)?;
        text = replace_once(
            text,
            "        bqatlas_sample_crm::migration(),",
            &format!("        bqatlas_sample_crm::migration(),\n        {alias}::migration(),"),
        )?;
        text = replace_once(
            text,
            "    registry.add_module(bqatlas_sample_crm::definition())?;",
            &format!(
                "    registry.add_module(bqatlas_sample_crm::definition())?;\n    registry.add_module({alias}::definition())?;\n    registry.add_resource({alias}::descriptor())?;"
            ),
        )?;
        text = replace_once(
            text,
            "    permissions\n}",
            &format!(
                "    permissions.extend({alias}::PERMISSIONS.into_iter().map(str::to_owned));\n    permissions\n}}"
            ),
        )?;
        text = replace_once(
            text,
            ".merge(bqatlas_sample_crm::router().with_state(CustomerService::new(db.clone())))",
            &format!(
                ".merge(bqatlas_sample_crm::router().with_state(CustomerService::new(db.clone())))\n            .merge({alias}::router().with_state({alias}::{}Service::new(db.clone())))",
                spec.entity
            ),
        )?;
        changes.push((host, text));
        let host_manifest = app.join("samples/crm/server/Cargo.toml");
        let text = fs::read_to_string(&host_manifest)?;
        changes.push((
            host_manifest,
            replace_once(
                text,
                "[dependencies]",
                &format!(
                    "[dependencies]\n{package} = {{ path = \"../modules/{}\" }}",
                    spec.module
                ),
            )?,
        ));
        let web = app.join("samples/crm/web/src/main.ts");
        let text = fs::read_to_string(&web)?;
        let feature = format!(
            "this.crud.register({{resource:{:?},title:{:?},icon:\"users\",writePermission:{:?},deletePermission:{:?},defaults:{{code:\"\",name:\"\",email:\"\",active:true}}}});\n    this.menus.register(",
            format!("{}.{}", spec.module, spec.resource),
            spec.title,
            format!("{}.{}.write", spec.module, spec.resource),
            format!("{}.{}.delete", spec.module, spec.resource)
        );
        let text = replace_once(text, "this.menus.register(", &feature)?;
        let menu = format!(
            "this.menus.register(\n      {{id:{:?},kind:\"resource\",label:{:?},resource:{:?}}},",
            format!("{}.{}", spec.module, spec.resource),
            spec.title,
            format!("{}.{}", spec.module, spec.resource)
        );
        changes.push((web, replace_once(text, "this.menus.register(", &menu)?));
    }
    fs::create_dir_all(target.join("src"))?;
    fs::write(target.join("src/lib.rs"), lib)?;
    fs::write(target.join("Cargo.toml"), manifest)?;
    for engine in ["postgresql", "sqlserver"] {
        let dir = target.join("migrations").join(engine);
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join("0001.sql"),
            transform(source(&format!(
                "samples/crm/modules/crm/migrations/{engine}/0001.sql"
            ))?),
        )?;
    }
    fs::write(
        target.join("RESOURCE.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"module":spec.module,"resource":spec.resource,"entity":spec.entity,"title":spec.title}),
        )?,
    )?;
    for (path, text) in changes {
        fs::write(path, text)?;
    }
    println!(
        "Created {}. Extend code/name/email/active rules, then run migrate. New permissions require deliberate account provisioning; existing accounts are preserved.",
        target.display()
    );
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Options::parse().command {
        Command::New { directory, name } => new_app(&directory, name),
        Command::Directory { spec, app, wire } => directory(&app, &spec, wire),
    }
}
