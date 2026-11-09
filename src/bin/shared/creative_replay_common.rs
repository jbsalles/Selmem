// Shared replay input/output and deterministic sampling helpers.
type Result<T> = std::result::Result<T, String>;
const CONDITIONS: [&str; 6] = [
    "empty",
    "empty_two_step",
    "rag",
    "before_sleep_guarded",
    "after_no_recon",
    "after_full_guarded",
];
fn s<'a>(v: &'a Json, k: &str) -> Result<&'a str> {
    v.get(k)?.string()
}
fn n(v: &Json, k: &str) -> Result<f64> {
    v.get(k)?.number()
}
fn array<'a>(v: &'a Json, k: &str) -> Result<&'a [Json]> {
    v.get(k)?.array().map(|a| &a[..])
}
fn boolean(v: &Json, k: &str) -> Result<bool> {
    match v.get(k)? {
        Json::Bool(b) => Ok(*b),
        _ => Err(format!("boolean required: {k}")),
    }
}
fn arr(xs: impl IntoIterator<Item = Json>) -> Json {
    Json::Array(xs.into_iter().collect())
}
fn require(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(msg.into())
    }
}
fn load(p: &Path) -> Result<Json> {
    json::parse(&fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?)
}
fn hash(p: &Path) -> Result<String> {
    Ok(sha256::hex(
        &fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
    ))
}
fn words(s: &str) -> usize {
    s.split_whitespace().count()
}
fn commit(p: &Path, v: &Json) -> Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("replay.tmp");
    let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    f.write_all(format!("{}\n", v.encode()).as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(tmp, p).map_err(|e| e.to_string())
}
fn fresh(p: &Path, v: &Json) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(p)
        .map_err(|e| format!("fresh output {}: {e}", p.display()))?;
    f.write_all(format!("{}\n", v.encode()).as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}
fn append(p: &Path, v: &Json) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(p)
        .map_err(|e| e.to_string())?;
    f.write_all(format!("{}\n", v.encode()).as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}
fn lines(p: &Path) -> Result<Vec<Json>> {
    if !p.exists() {
        return Ok(vec![]);
    }
    fs::read_to_string(p)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(json::parse)
        .collect()
}
fn shuffle<T>(xs: &mut [T], mut seed: u64) {
    for i in (1..xs.len()).rev() {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        xs.swap(i, ((seed >> 32) as usize) % (i + 1));
    }
}
