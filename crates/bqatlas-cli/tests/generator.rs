use std::{fs, process::Command};
use uuid::Uuid;
#[test]
fn starter_is_portable_and_directory_wiring_is_reviewable() {
    let base = std::env::temp_dir().join(format!("bqatlas-generator-{}", Uuid::new_v4().simple()));
    let app = base.join("my-erp");
    fs::create_dir(&base).expect("temp");
    let cli = env!("CARGO_BIN_EXE_bqatlas");
    assert!(
        Command::new(cli)
            .args(["new", app.to_str().expect("path")])
            .status()
            .expect("new")
            .success()
    );
    for name in [
        "Cargo.toml",
        "AGENTS.md",
        "docs/ARCHITECTURE.md",
        "docs/HANDOFF.md",
        "docs/EXTENDING-RESOURCE.md",
        "samples/crm/web/vendor/manifest.json",
        "crates/bqatlas-auth/src/oidc.rs",
        "dev/oidc-roles.json",
    ] {
        assert!(app.join(name).is_file());
    }
    for name in [".env", "target", "node_modules", ".git", ".local-mail"] {
        assert!(!app.join(name).exists());
    }
    assert!(
        !Command::new(cli)
            .args(["new", app.to_str().expect("path")])
            .status()
            .expect("existing")
            .success()
    );
    let spec = base.join("warehouses.json");
    fs::write(&spec,r#"{"module":"inventory","resource":"warehouses","entity":"Warehouse","title":"Warehouses"}"#).expect("spec");
    assert!(
        Command::new(cli)
            .args([
                "directory",
                "--spec",
                spec.to_str().expect("spec"),
                "--app",
                app.to_str().expect("app"),
                "--wire"
            ])
            .status()
            .expect("directory")
            .success()
    );
    let lib =
        fs::read_to_string(app.join("samples/crm/modules/inventory/src/lib.rs")).expect("module");
    assert!(lib.contains("inventory.warehouses.read"));
    assert!(!lib.contains("crm.customers"));
    assert!(!lib.contains("CustomerDirectory"));
    assert!(
        fs::read_to_string(app.join("samples/crm/server/src/lib.rs"))
            .expect("host")
            .contains("bqatlas_inventory_warehouses::migration")
    );
    fs::remove_dir_all(base).expect("cleanup own test directory");
}
