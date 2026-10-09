//! Explicit generation settings and read-only snapshot reuse for paired runs.
use super::json::{self, Json};
use super::sha256;
use std::{fs, path::Path};

pub fn validate(raw: &str) -> Result<String, String> {
    let effort = raw.trim().to_ascii_lowercase();
    match effort.as_str() {
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" => Ok(effort),
        _ => Err("reasoning must be an explicit effort: none, minimal, low, medium, high or xhigh; provider/model support still applies".into()),
    }
}

pub fn validate_source(source: &Path, protocol: &Json, protocol_hash: &str) -> Result<(), String> {
    let raw = fs::read_to_string(source.join("manifest.json")).map_err(|e| format!("cannot read memory-source manifest: {e}"))?;
    let manifest = json::parse(&raw)?;
    if manifest.get("protocol_sha256")?.string()? != protocol_hash
        || manifest.get("dataset_sha256")?.string()? != protocol.get("dataset_sha256")?.string()?
        || manifest.get("sample_id")?.string()? != protocol.get("sample_id")?.string()? {
        return Err("memory source must use the same protocol, dataset and sample".into());
    }
    if manifest.get("organ_revision")?.string()? != super::ORGAN_REVISION {
        return Err("memory source must use the frozen organ revision".into());
    }
    let hashes = manifest.get("memory_snapshot_sha256")?;
    let status = manifest.get("status")?.string()?;
    if !(status.starts_with("complete") || status.starts_with("technical rehearsal complete")) {
        return Err("memory source must be a completed run".into());
    }
    for name in ["full_a", "full_b", "full_null_a", "full_null_b", "nosleep_a", "nosleep_b"] {
        let bytes = fs::read(source.join(format!("{name}.selmem"))).map_err(|e| format!("missing source snapshot {name}: {e}"))?;
        if hashes.get(name)?.string()? != sha256::hex(&bytes) {
            return Err(format!("source snapshot checksum mismatch: {name}"));
        }
    }
    Ok(())
}

pub fn copy_snapshot(source: &Path, name: &str, target: &Path) -> Result<(), String> {
    fs::copy(source.join(format!("{name}.selmem")), target).map_err(|e| e.to_string())?;
    // Carry forward audits as historical preparation evidence, never regenerate
    // them or feed them to runtime recall. No sleep/encoding happens in reuse.
    for suffix in ["book.json", "audit.jsonl"] {
        let old = source.join(format!("{name}.{suffix}"));
        if old.is_file() {
            fs::copy(old, target.with_extension(suffix)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::json::{obj, text};
    #[test]
    fn explicit_effort_has_no_silent_provider_default() {
        assert_eq!(validate("none").unwrap(), "none");
        assert_eq!(validate(" low ").unwrap(), "low");
        assert!(validate("off").is_err());
        assert!(validate("").is_err());
    }
    #[test]
    fn reuse_requires_matching_protocol_and_preserves_snapshot_bytes() {
        let root = std::env::temp_dir().join(format!("selmem-reasoning-reuse-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let protocol = obj([("dataset_sha256", text("dataset")), ("sample_id", text("sample"))]);
        let mut manifest = obj([("protocol_sha256", text("protocol")), ("dataset_sha256", text("dataset")), ("sample_id", text("sample")), ("status", text("complete")), ("organ_revision", text(super::super::ORGAN_REVISION))]);
        fs::write(root.join("manifest.json"), manifest.encode()).unwrap();
        assert!(validate_source(&root, &protocol, "wrong-protocol").is_err());
        assert!(validate_source(&root, &protocol, "protocol").is_err());
        for name in ["full_a", "full_b", "full_null_a", "full_null_b", "nosleep_a", "nosleep_b"] {
            fs::write(root.join(format!("{name}.selmem")), format!("snapshot:{name}")).unwrap();
        }
        let hashes = ["full_a", "full_b", "full_null_a", "full_null_b", "nosleep_a", "nosleep_b"]
            .into_iter().map(|name| (name.to_string(), text(sha256::hex(&fs::read(root.join(format!("{name}.selmem"))).unwrap())))).collect();
        manifest.put("memory_snapshot_sha256", Json::Object(hashes));
        fs::write(root.join("manifest.json"), manifest.encode()).unwrap();
        validate_source(&root, &protocol, "protocol").unwrap();
        assert!(!root.join("selection_a.selmem").exists());
        let mut outdated = manifest.clone();
        outdated.put("organ_revision", text("old-organ"));
        fs::write(root.join("manifest.json"), outdated.encode()).unwrap();
        assert!(validate_source(&root, &protocol, "protocol").is_err());
        fs::write(root.join("manifest.json"), manifest.encode()).unwrap();
        let target = root.join("copy.selmem");
        copy_snapshot(&root, "full_a", &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), fs::read(root.join("full_a.selmem")).unwrap());
        let mut checked = manifest;
        let hashes = ["full_a", "full_b", "full_null_a", "full_null_b", "nosleep_a", "nosleep_b"]
            .into_iter().map(|name| (name.to_string(), text(sha256::hex(&fs::read(root.join(format!("{name}.selmem"))).unwrap())))).collect();
        checked.put("memory_snapshot_sha256", Json::Object(hashes));
        fs::write(root.join("manifest.json"), checked.encode()).unwrap();
        validate_source(&root, &protocol, "protocol").unwrap();
        fs::write(root.join("full_a.selmem"), "changed snapshot").unwrap();
        assert!(validate_source(&root, &protocol, "protocol").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn copied_book_has_identical_read_only_context() {
        use selmem::{EncodeInput, EntityProfile, SelectiveMemory};
        let root = std::env::temp_dir().join(format!("selmem-reasoning-context-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut memory = SelectiveMemory::new(EntityProfile::new("Claire")).detach_clock();
        memory.clock.origin_real = 4_102_444_800;
        memory.clock.scale = 1;
        let mut event = EncodeInput::new("Caroline said: The support group made me feel accepted.");
        event.permanence = 1.0;
        assert!(memory.live_with(event).kept);
        let source = root.join("full_a.selmem");
        memory.path = Some(source.clone());
        memory.save().unwrap();
        let copy = root.join("copy.selmem");
        let before = fs::read(&source).unwrap();
        copy_snapshot(&root, "full_a", &copy).unwrap();
        let query = "What effect did the support group have on Caroline?";
        let a = super::super::memory::readout(&source, query, None).unwrap();
        let b = super::super::memory::readout(&copy, query, None).unwrap();
        assert_eq!(a.encode(), b.encode());
        assert_eq!(fs::read(&source).unwrap(), before);
        assert_eq!(fs::read(&copy).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

}
