use std::ffi::{CStr, CString};
use std::io;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

use crate::core::model::{Channel, DriftKind, Mood, TraceStatus};
use crate::persist::snapshot::{
    assemble_archive, assemble_axiom, assemble_drift, assemble_mood, assemble_trace,
    channel_token, drift_token, layer_token, parse_layer_token, profile_from_params,
    profile_params_line, status_token, Snapshot,
};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;

#[repr(C)]
struct Sqlite3 {
    _private: [u8; 0],
}

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open(filename: *const c_char, pp_db: *mut *mut Sqlite3) -> c_int;
    fn sqlite3_close(db: *mut Sqlite3) -> c_int;
    fn sqlite3_exec(
        db: *mut Sqlite3,
        sql: *const c_char,
        cb: Option<unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int>,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(p: *mut c_void);
    fn sqlite3_busy_timeout(db: *mut Sqlite3, ms: c_int) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut Sqlite3,
        zsql: *const c_char,
        nbyte: c_int,
        ppstmt: *mut *mut SqliteStmt,
        pztail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut SqliteStmt,
        idx: c_int,
        val: *const c_char,
        n: c_int,
        destructor: *mut c_void,
    ) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut SqliteStmt, idx: c_int, val: i64) -> c_int;
    fn sqlite3_bind_double(stmt: *mut SqliteStmt, idx: c_int, val: f64) -> c_int;
    fn sqlite3_bind_null(stmt: *mut SqliteStmt, idx: c_int) -> c_int;
    fn sqlite3_step(stmt: *mut SqliteStmt) -> c_int;
    fn sqlite3_reset(stmt: *mut SqliteStmt) -> c_int;
    fn sqlite3_finalize(stmt: *mut SqliteStmt) -> c_int;
}

#[repr(C)]
struct SqliteStmt {
    _private: [u8; 0],
}

const SQLITE_DONE: c_int = 101;
const SQLITE_TRANSIENT: *mut c_void = -1isize as *mut c_void;

struct Db {
    raw: *mut Sqlite3,
}

impl Db {
    fn open(path: &Path) -> io::Result<Self> {
        let c = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut raw = ptr::null_mut();
        let rc = unsafe { sqlite3_open(c.as_ptr(), &mut raw) };
        if rc != 0 {
            if !raw.is_null() {
                unsafe { sqlite3_close(raw) };
            }
            return Err(io::Error::new(io::ErrorKind::Other, format!("sqlite open {rc}")));
        }
        unsafe { sqlite3_busy_timeout(raw, 5000) };
        Ok(Self { raw })
    }

    fn prepare(&self, sql: &str) -> io::Result<Stmt> {
        let c = CString::new(sql).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut raw = ptr::null_mut();
        let rc = unsafe { sqlite3_prepare_v2(self.raw, c.as_ptr(), -1, &mut raw, ptr::null_mut()) };
        if rc != 0 || raw.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("sqlite prepare {rc}")));
        }
        Ok(Stmt { raw })
    }

    fn exec(&self, sql: &str) -> io::Result<()> {
        let c = CString::new(sql).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut err = ptr::null_mut();
        let rc = unsafe { sqlite3_exec(self.raw, c.as_ptr(), None, ptr::null_mut(), &mut err) };
        if rc != 0 {
            let msg = if err.is_null() {
                format!("sqlite {rc}")
            } else {
                let s = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
                unsafe { sqlite3_free(err as *mut c_void) };
                s
            };
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        Ok(())
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sqlite3_close(self.raw) };
        }
    }
}

struct Stmt {
    raw: *mut SqliteStmt,
}

impl Stmt {
    fn bind_text(&self, i: c_int, s: &str) -> io::Result<()> {
        let c = CString::new(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let rc = unsafe { sqlite3_bind_text(self.raw, i, c.as_ptr(), -1, SQLITE_TRANSIENT) };
        if rc != 0 {
            Err(io::Error::new(io::ErrorKind::Other, format!("bind_text {rc}")))
        } else {
            Ok(())
        }
    }
    fn bind_opt_text(&self, i: c_int, s: Option<&str>) -> io::Result<()> {
        match s {
            Some(s) => self.bind_text(i, s),
            None => self.bind_null(i),
        }
    }
    fn bind_i64(&self, i: c_int, v: i64) -> io::Result<()> {
        let rc = unsafe { sqlite3_bind_int64(self.raw, i, v) };
        if rc != 0 {
            Err(io::Error::new(io::ErrorKind::Other, format!("bind_i64 {rc}")))
        } else {
            Ok(())
        }
    }
    fn bind_f64(&self, i: c_int, v: f64) -> io::Result<()> {
        let rc = unsafe { sqlite3_bind_double(self.raw, i, v) };
        if rc != 0 {
            Err(io::Error::new(io::ErrorKind::Other, format!("bind_f64 {rc}")))
        } else {
            Ok(())
        }
    }
    fn bind_null(&self, i: c_int) -> io::Result<()> {
        let rc = unsafe { sqlite3_bind_null(self.raw, i) };
        if rc != 0 {
            Err(io::Error::new(io::ErrorKind::Other, format!("bind_null {rc}")))
        } else {
            Ok(())
        }
    }
    fn step_done(&self) -> io::Result<()> {
        let rc = unsafe { sqlite3_step(self.raw) };
        if rc != SQLITE_DONE {
            return Err(io::Error::new(io::ErrorKind::Other, format!("sqlite step {rc}")));
        }
        unsafe { sqlite3_reset(self.raw) };
        Ok(())
    }
}

impl Drop for Stmt {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sqlite3_finalize(self.raw) };
        }
    }
}



pub fn save(path: &Path, profile: &EntityProfile, mood: &Mood, store: &MemoryStore) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let db = Db::open(path)?;
    db.exec(
        "CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS archives(id TEXT PRIMARY KEY, created INTEGER, source TEXT, verbatim TEXT);
         CREATE TABLE IF NOT EXISTS traces(
           id TEXT PRIMARY KEY, gist TEXT, core TEXT, valence REAL, arousal REAL, disgust REAL,
           self_relevance REAL, schema TEXT, channel TEXT, archive_id TEXT,
           created INTEGER, last_recalled INTEGER, last_consolidated INTEGER,
           fidelity REAL, permanence REAL, rehearsals INTEGER, access REAL,
           status TEXT, salience REAL, embedding TEXT, anchor REAL, detach_strikes INTEGER);
         CREATE TABLE IF NOT EXISTS cues(trace_id TEXT, cue TEXT);
         CREATE TABLE IF NOT EXISTS drifts(
           trace_id TEXT, kind TEXT, at INTEGER, note TEXT,
           fidelity_delta REAL, valence_delta REAL, disgust_delta REAL);
         CREATE TABLE IF NOT EXISTS axioms(
           id TEXT PRIMARY KEY, statement TEXT, valence REAL, strength REAL,
           created INTEGER, superseded_by TEXT, schema TEXT, layer TEXT);
         CREATE TABLE IF NOT EXISTS axiom_support(axiom_id TEXT, trace_id TEXT);
         CREATE TABLE IF NOT EXISTS edges(a TEXT, b TEXT);",
    )?;
    let _ = db.exec("ALTER TABLE traces ADD COLUMN core TEXT;");
    let _ = db.exec("ALTER TABLE traces ADD COLUMN anchor REAL;");
    let _ = db.exec("ALTER TABLE traces ADD COLUMN detach_strikes INTEGER;");
    let _ = db.exec("ALTER TABLE axioms ADD COLUMN layer TEXT;");
    db.exec("BEGIN IMMEDIATE;")?;
    db.exec(
        "DELETE FROM meta; DELETE FROM archives; DELETE FROM traces;
         DELETE FROM cues; DELETE FROM drifts; DELETE FROM axioms;
         DELETE FROM axiom_support; DELETE FROM edges;",
    )?;

    {
        let st = db.prepare("INSERT INTO meta(k,v) VALUES (?1,?2)")?;
        let deep = store
            .last_deep_at
            .map(|ts| ts.to_string())
            .unwrap_or_else(|| "-".into());
        for (k, v) in [
            ("name", profile.name.as_str()),
            ("params", &profile_params_line(profile)),
            ("mood", &format!("{} {} {}", mood.valence, mood.arousal, mood.disgust)),
            ("last_deep", deep.as_str()),
        ] {
            st.bind_text(1, k)?;
            st.bind_text(2, v)?;
            st.step_done()?;
        }
    }
    {
        let st = db.prepare("INSERT INTO archives(id,created,source,verbatim) VALUES (?1,?2,?3,?4)")?;
        for a in store.archives.values() {
            st.bind_text(1, &a.id)?;
            st.bind_i64(2, a.created_at as i64)?;
            st.bind_text(3, &a.source)?;
            st.bind_text(4, &a.verbatim)?;
            st.step_done()?;
        }
    }
    let q_tr = db.prepare(
        "INSERT INTO traces(id,gist,core,valence,arousal,disgust,self_relevance,schema,channel,archive_id,created,last_recalled,last_consolidated,fidelity,permanence,rehearsals,access,status,salience,embedding,anchor,detach_strikes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
    )?;
    let q_cue = db.prepare("INSERT INTO cues(trace_id,cue) VALUES (?1,?2)")?;
    let q_dr = db.prepare(
        "INSERT INTO drifts(trace_id,kind,at,note,fidelity_delta,valence_delta,disgust_delta) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )?;
    for t in store.traces.values() {
        q_tr.bind_text(1, &t.id)?;
        q_tr.bind_text(2, &t.gist)?;
        q_tr.bind_text(3, &t.core)?;
        q_tr.bind_f64(4, t.valence as f64)?;
        q_tr.bind_f64(5, t.arousal as f64)?;
        q_tr.bind_f64(6, t.disgust as f64)?;
        q_tr.bind_f64(7, t.self_relevance as f64)?;
        q_tr.bind_opt_text(8, t.schema.as_deref())?;
        q_tr.bind_text(9, channel_token(t.channel))?;
        q_tr.bind_opt_text(10, t.archive_id.as_deref())?;
        q_tr.bind_i64(11, t.created_at as i64)?;
        match t.last_recalled_at {
            Some(v) => q_tr.bind_i64(12, v as i64)?,
            None => q_tr.bind_null(12)?,
        }
        match t.last_consolidated_at {
            Some(v) => q_tr.bind_i64(13, v as i64)?,
            None => q_tr.bind_null(13)?,
        }
        q_tr.bind_f64(14, t.fidelity as f64)?;
        q_tr.bind_f64(15, t.permanence as f64)?;
        q_tr.bind_i64(16, t.rehearsals as i64)?;
        q_tr.bind_f64(17, t.access as f64)?;
        q_tr.bind_text(18, status_token(t.status))?;
        q_tr.bind_f64(19, t.salience_at_encode as f64)?;
        q_tr.bind_text(20, &pack_emb(&t.embedding))?;
        q_tr.bind_f64(21, t.anchor as f64)?;
        q_tr.bind_i64(22, t.detach_strikes as i64)?;
        q_tr.step_done()?;
        for c in &t.cues {
            q_cue.bind_text(1, &t.id)?;
            q_cue.bind_text(2, c)?;
            q_cue.step_done()?;
        }
        for d in &t.drifts {
            q_dr.bind_text(1, &t.id)?;
            q_dr.bind_text(2, drift_token(d.kind))?;
            q_dr.bind_i64(3, d.at as i64)?;
            q_dr.bind_text(4, &d.note)?;
            q_dr.bind_f64(5, d.fidelity_delta as f64)?;
            q_dr.bind_f64(6, d.valence_delta as f64)?;
            q_dr.bind_f64(7, d.disgust_delta as f64)?;
            q_dr.step_done()?;
        }
    }
    let q_ax = db.prepare(
        "INSERT INTO axioms(id,statement,valence,strength,created,superseded_by,schema,layer) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )?;
    let q_sup = db.prepare("INSERT INTO axiom_support(axiom_id,trace_id) VALUES (?1,?2)")?;
    for a in store.axioms.values() {
        q_ax.bind_text(1, &a.id)?;
        q_ax.bind_text(2, &a.statement)?;
        q_ax.bind_f64(3, a.valence as f64)?;
        q_ax.bind_f64(4, a.strength as f64)?;
        q_ax.bind_i64(5, a.created_at as i64)?;
        q_ax.bind_opt_text(6, a.superseded_by.as_deref())?;
        q_ax.bind_opt_text(7, a.schema.as_deref())?;
        q_ax.bind_text(8, layer_token(a.layer))?;
        q_ax.step_done()?;
        for tid in &a.support_trace_ids {
            q_sup.bind_text(1, &a.id)?;
            q_sup.bind_text(2, tid)?;
            q_sup.step_done()?;
        }
    }
    let q_ed = db.prepare("INSERT INTO edges(a,b) VALUES (?1,?2)")?;
    for (src, dsts) in &store.edges {
        for dst in dsts {
            if src < dst {
                q_ed.bind_text(1, src)?;
                q_ed.bind_text(2, dst)?;
                q_ed.step_done()?;
            }
        }
    }
    db.exec("COMMIT;")?;
    Ok(())
}

struct Rows {
    cells: Vec<Vec<String>>,
}

unsafe extern "C" fn collect_cb(
    arg: *mut c_void,
    n: c_int,
    vals: *mut *mut c_char,
    _cols: *mut *mut c_char,
) -> c_int {
    let rows = &mut *(arg as *mut Rows);
    let mut rec = Vec::with_capacity(n as usize);
    for i in 0..n {
        let p = *vals.add(i as usize);
        if p.is_null() {
            rec.push(String::new());
        } else {
            rec.push(CStr::from_ptr(p).to_string_lossy().into_owned());
        }
    }
    rows.cells.push(rec);
    0
}

fn query(db: &Db, sql: &str) -> io::Result<Vec<Vec<String>>> {
    let c = CString::new(sql).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let mut rows = Rows { cells: Vec::new() };
    let mut err = ptr::null_mut();
    let rc = unsafe {
        sqlite3_exec(
            db.raw,
            c.as_ptr(),
            Some(collect_cb),
            &mut rows as *mut Rows as *mut c_void,
            &mut err,
        )
    };
    if rc != 0 {
        let msg = if err.is_null() {
            format!("sqlite query {rc}")
        } else {
            let s = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
            unsafe { sqlite3_free(err as *mut c_void) };
            s
        };
        return Err(io::Error::new(io::ErrorKind::Other, msg));
    }
    Ok(rows.cells)
}

pub fn load(path: &Path) -> io::Result<Snapshot> {
    let db = Db::open(path)?;
    let meta = query(&db, "SELECT k,v FROM meta")?;
    let mut name = String::new();
    let mut params_s = String::new();
    let mut mood_s = String::new();
    let mut last_deep_s = String::new();
    for row in meta {
        if row.len() < 2 {
            continue;
        }
        match row[0].as_str() {
            "name" => name = row[1].clone(),
            "params" => params_s = row[1].clone(),
            "mood" => mood_s = row[1].clone(),
            "last_deep" => last_deep_s = row[1].clone(),
            _ => {}
        }
    }
    let profile = parse_profile(&name, &params_s)?;
    let mood = parse_mood(&mood_s)?;
    let mut store = MemoryStore::new();
    if !last_deep_s.is_empty() && last_deep_s != "-" {
        store.last_deep_at = last_deep_s.parse().ok();
    }

    for row in query(&db, "SELECT id,created,source,verbatim FROM archives")? {
        if row.len() < 4 {
            continue;
        }
        let a = assemble_archive(
            row[0].clone(),
            row[1].parse().unwrap_or(0),
            row[2].clone(),
            row[3].clone(),
        );
        store.archives.insert(a.id.clone(), a);
    }

    for row in query(
        &db,
        "SELECT id,gist,core,valence,arousal,disgust,self_relevance,schema,channel,archive_id,created,last_recalled,last_consolidated,fidelity,permanence,rehearsals,access,status,salience,embedding,anchor,detach_strikes FROM traces",
    )? {
        if row.len() < 20 {
            continue;
        }
        let t = assemble_trace(
            row[0].clone(),
            row[1].clone(),
            row[2].clone(),
            row[3].parse().unwrap_or(0.0),
            row[4].parse().unwrap_or(0.0),
            row[5].parse().unwrap_or(0.0),
            row[6].parse().unwrap_or(0.0),
            empty_none(&row[7]),
            parse_ch(&row[8]),
            empty_none(&row[9]),
            row[10].parse().unwrap_or(0),
            parse_opt_u64(&row[11]),
            parse_opt_u64(&row[12]),
            row[13].parse().unwrap_or(1.0),
            row[14].parse().unwrap_or(0.0),
            row[15].parse().unwrap_or(0),
            row[16].parse().unwrap_or(1.0),
            parse_st(&row[17]),
            row[18].parse().unwrap_or(0.0),
            unpack_emb(&row[19]),
            row.get(20).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            row.get(21).and_then(|s| s.parse().ok()).unwrap_or(0),
            Vec::new(),
            Vec::new(),
        );
        store.traces.insert(t.id.clone(), t);
    }

    for row in query(&db, "SELECT trace_id,cue FROM cues")? {
        if row.len() < 2 {
            continue;
        }
        if let Some(t) = store.traces.get_mut(&row[0]) {
            t.cues.push(row[1].clone());
        }
    }
    for row in query(
        &db,
        "SELECT trace_id,kind,at,note,fidelity_delta,valence_delta,disgust_delta FROM drifts",
    )? {
        if row.len() < 7 {
            continue;
        }
        if let Some(t) = store.traces.get_mut(&row[0]) {
            t.drifts.push(assemble_drift(
                parse_dk(&row[1]),
                row[2].parse().unwrap_or(0),
                row[3].clone(),
                row[4].parse().unwrap_or(0.0),
                row[5].parse().unwrap_or(0.0),
                row[6].parse().unwrap_or(0.0),
            ));
        }
    }
    for row in query(
        &db,
        "SELECT id,statement,valence,strength,created,superseded_by,schema,layer FROM axioms",
    )? {
        if row.len() < 6 {
            continue;
        }
        let a = assemble_axiom(
            row[0].clone(),
            row[1].clone(),
            row[2].parse().unwrap_or(0.0),
            row[3].parse().unwrap_or(0.0),
            row[4].parse().unwrap_or(0),
            empty_none(&row[5]),
            row.get(6).and_then(|s| empty_none(s)),
            parse_layer_token(row.get(7).map(|s| s.as_str()).unwrap_or("belief")),
            Vec::new(),
        );
        store.axioms.insert(a.id.clone(), a);
    }
    for row in query(&db, "SELECT axiom_id,trace_id FROM axiom_support")? {
        if row.len() < 2 {
            continue;
        }
        if let Some(a) = store.axioms.get_mut(&row[0]) {
            a.support_trace_ids.push(row[1].clone());
        }
    }
    for row in query(&db, "SELECT a,b FROM edges")? {
        if row.len() < 2 {
            continue;
        }
        store.link(&row[0], &row[1]);
    }

    crate::persist::prune_orphaned_archives(&mut store);
    crate::persist::bump_id_counter(&store);
    Ok(Snapshot {
        profile,
        mood,
        store,
    })
}

fn parse_profile(name: &str, rest: &str) -> io::Result<EntityProfile> {
    let n: Vec<f32> = rest.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    profile_from_params(name, &n)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "params sqlite incomplets"))
}

fn parse_mood(s: &str) -> io::Result<Mood> {
    let p: Vec<f32> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    if p.len() < 3 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "mood sqlite"));
    }
    Ok(assemble_mood(p[0], p[1], p[2]))
}

fn pack_emb(v: &[f32]) -> String {
    v.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(",")
}

fn unpack_emb(s: &str) -> Vec<f32> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',').filter_map(|x| x.parse().ok()).collect()
}

fn empty_none(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn parse_opt_u64(s: &str) -> Option<u64> {
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

fn parse_ch(s: &str) -> Channel {
    match s {
        "world" => Channel::World,
        "log" => Channel::Log,
        _ => Channel::Selfhood,
    }
}
fn parse_st(s: &str) -> TraceStatus {
    match s {
        "cold" => TraceStatus::Cold,
        "myth" => TraceStatus::Myth,
        "latent" => TraceStatus::Latent,
        _ => TraceStatus::Active,
    }
}
fn parse_dk(s: &str) -> DriftKind {
    match s {
        "disgust" => DriftKind::AmplifyDisgust,
        "fade" => DriftKind::Fade,
        "merge" => DriftKind::Merge,
        "weather" => DriftKind::Weather,
        "rewrite" => DriftKind::Rewrite,
        "reinterpret" => DriftKind::Reinterpret,
        "ground" => DriftKind::Ground,
        "color" => DriftKind::Color,
        _ => DriftKind::Embellish,
    }
}

