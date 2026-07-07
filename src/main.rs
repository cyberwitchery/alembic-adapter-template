//! example alembic external adapter.
//!
//! an external adapter is a standalone binary that the alembic CLI spawns as a
//! subprocess. it reads one JSON request on stdin and writes one JSON response
//! on stdout. the `alembic_external_main!` macro wires up that protocol for you;
//! all you implement is the `ExternalAdapter` trait below.
//!
//! to turn this template into a real adapter:
//!   1. rename the crate (Cargo.toml `name`) and the `ExampleAdapter` type.
//!   2. put your backend's connection/config on the struct and parse it in `setup`.
//!   3. implement `read` (observe backend state) and `write` (apply ops).
//!   4. if your backend needs schema provisioned first, implement `ensure_schema`
//!      (and `preview_schema`, so `alembic plan` can show the schema work).
//!
//! protocol reference:
//! <https://github.com/cyberwitchery/alembic/blob/main/docs/external-adapters.md>

use alembic_core::{Schema, TypeName};
use alembic_engine::{
    alembic_external_main, AppliedOp, ApplyReport, ExternalAdapter, ExternalObject, Op, StateData,
};
use anyhow::Result;

// generates `fn main()`, which runs the stdin/stdout protocol against our adapter.
alembic_external_main!(ExampleAdapter::new());

/// an example read+write backend adapter.
///
/// replace this with your backend's client and state. the fields are populated
/// from the `setup:` block of the backend config yaml (see `examples/backend.yaml`).
pub struct ExampleAdapter {
    /// base url of the backend, configurable via `setup.host`.
    host: String,
}

impl Default for ExampleAdapter {
    fn default() -> Self {
        Self {
            host: "http://localhost:8080".into(),
        }
    }
}

impl ExampleAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExternalAdapter for ExampleAdapter {
    /// called once per request with the `setup:` block from the backend config.
    /// parse your connection details and options here.
    fn setup(&mut self, configuration: &serde_yaml::Value) -> Result<()> {
        if let Some(host) = config_str(configuration, "host") {
            self.host = host.to_string();
        }
        Ok(())
    }

    /// observe current backend state for the requested `types`.
    ///
    /// the engine diffs what you return here against the desired inventory to
    /// build a plan, so return one object per backend record: its natural `key`,
    /// observed `attrs` (using ir field names), and `backend_id` when known.
    /// `state` carries the engine's existing uid -> backend_id mappings.
    ///
    /// an emit-only adapter (one that just writes artifacts and keeps no backend
    /// state) can return an empty vec, which makes every desired object a create.
    fn read(
        &mut self,
        _schema: &Schema,
        _types: &[TypeName],
        _state: &StateData,
    ) -> Result<Vec<ExternalObject>> {
        // TODO: query `self.host` and map each record into an `ExternalObject`.
        Ok(vec![])
    }

    /// apply plan operations to the backend.
    ///
    /// each `Op` is a create, update, or delete. perform the side effect, then
    /// push an `AppliedOp` so the engine can persist the uid -> backend_id
    /// mapping in its state store. set `backend_id` to the id your backend
    /// assigns on create (this keeps identities stable across renames); leave it
    /// `None` if you have nothing to map.
    fn write(&mut self, _schema: &Schema, ops: &[Op], _state: &StateData) -> Result<ApplyReport> {
        let mut report = ApplyReport::default();
        for op in ops {
            match op {
                Op::Create {
                    uid,
                    type_name,
                    desired,
                } => {
                    // TODO: create `desired` on the backend, capture its id.
                    let _ = desired;
                    report.applied.push(AppliedOp {
                        uid: *uid,
                        type_name: type_name.clone(),
                        backend_id: None,
                    });
                }
                Op::Update {
                    uid,
                    type_name,
                    desired,
                    changes,
                    backend_id,
                } => {
                    // TODO: patch the record identified by `backend_id` with `changes`.
                    let _ = (desired, changes);
                    report.applied.push(AppliedOp {
                        uid: *uid,
                        type_name: type_name.clone(),
                        backend_id: backend_id.clone(),
                    });
                }
                Op::Delete {
                    uid,
                    type_name,
                    backend_id,
                    ..
                } => {
                    // TODO: delete the record identified by `backend_id`.
                    report.applied.push(AppliedOp {
                        uid: *uid,
                        type_name: type_name.clone(),
                        backend_id: backend_id.clone(),
                    });
                }
            }
        }
        Ok(report)
    }

    // optional: provision backend schema (custom fields, types, ...) on apply.
    // the trait's default returns an empty report; uncomment and implement if
    // your backend needs schema set up before objects can be written.
    //
    // fn ensure_schema(&mut self, _schema: &Schema) -> Result<alembic_engine::ProvisionReport> {
    //     Ok(alembic_engine::ProvisionReport::default())
    // }

    // optional: preview what `ensure_schema` would provision, writing nothing,
    // so `alembic plan` can show the schema work up front. the trait's default
    // returns `None` ("this adapter cannot preview"); if you implement
    // `ensure_schema`, implement this too and return `Some(report)`.
    //
    // fn preview_schema(
    //     &mut self,
    //     _schema: &Schema,
    // ) -> Result<Option<alembic_engine::ProvisionReport>> {
    //     Ok(None)
    // }
}

/// read an optional string field from a `setup:` config value.
fn config_str<'a>(cfg: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
    cfg.get(key).and_then(serde_yaml::Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alembic_core::Object;

    #[test]
    fn setup_reads_host() {
        let mut adapter = ExampleAdapter::default();
        let cfg: serde_yaml::Value =
            serde_yaml::from_str("host: https://backend.example.com").unwrap();
        adapter.setup(&cfg).unwrap();
        assert_eq!(adapter.host, "https://backend.example.com");
    }

    #[test]
    fn setup_keeps_default_when_unset() {
        let mut adapter = ExampleAdapter::default();
        adapter.setup(&serde_yaml::Value::default()).unwrap();
        assert_eq!(adapter.host, "http://localhost:8080");
    }

    #[test]
    fn read_returns_no_state() {
        let mut adapter = ExampleAdapter::default();
        let observed = adapter
            .read(&Schema::default(), &[], &StateData::default())
            .unwrap();
        assert!(observed.is_empty());
    }

    #[test]
    fn write_reports_each_op_applied() {
        let mut adapter = ExampleAdapter::default();

        let device: Object = serde_json::from_value(serde_json::json!({
            "uid": "7b8f7a92-8fd0-4667-9a4b-9f3b5c9a4aaa",
            "type": "dcim.device",
            "key": { "name": "leaf01" },
            "attrs": { "name": "leaf01", "primary_ip": "198.51.100.1", "site": "site-a" }
        }))
        .unwrap();
        let ops = vec![Op::Create {
            uid: device.uid,
            type_name: device.type_name.clone(),
            desired: device,
        }];

        let report = adapter
            .write(&Schema::default(), &ops, &StateData::default())
            .unwrap();

        assert_eq!(report.applied.len(), 1);
    }
}
