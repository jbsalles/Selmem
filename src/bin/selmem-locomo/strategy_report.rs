use super::json::Json;
use std::{fs, path::Path};

/// Experimental baseline: selected original dataset turns, never archive access.
pub fn raw_readout(selected: &Json, events: &[super::memory::Event]) -> Result<Json, String> {
    let mut raw = selected.clone();
    let ids = selected.get("selected_ids")?.array()?;
    let candidates = selected.get("retrieval")?.array()?;
    let mut memories = Vec::new();
    for id in ids {
        let id = id.string()?;
        let candidate = candidates.iter().find(|c|
            c.get("trace_id").and_then(Json::string).ok() == Some(id))
            .ok_or("raw baseline: selected trace missing from retrieval audit")?;
        let source = candidate.get("source")?.string()?;
        let matches: Vec<_> = events.iter().filter_map(|event| match event {
            super::memory::Event::Turn { source: s, event, .. } if s == source => Some(event),
            _ => None,
        }).collect();
        if matches.len() != 1 {
            return Err("raw baseline requires one original turn per selected source".into());
        }
        memories.push(super::json::text(matches[0]));
    }
    raw.put("memories", super::json::Json::Array(memories));
    raw.put("axioms", Json::Array(Vec::new()));
    raw.put("reading_profile", super::json::text(""));
    raw.put("context_mode", super::json::text("selected-original-dataset-turns"));
    Ok(raw)
}

pub fn write(out: &Path, manifest: &Json, rows: &[Json]) -> Result<(), String> {
    let payload = super::json::obj([
        ("manifest", manifest.clone()),
        ("rows", Json::Array(rows.to_vec())),
    ]).encode().replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026");
    let page = TEMPLATE.replace("__DATA__", &payload);
    fs::write(out.join("strategy.html"), page).map_err(|e| e.to_string())
}
const TEMPLATE: &str = r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Claire — Memory swap</title>
<style>body{font:16px system-ui;background:#0e1422;color:#edf0fa;max-width:1400px;margin:40px auto;padding:0 24px}h1{font-size:clamp(32px,5vw,58px);margin-bottom:12px}button,select,textarea{font:inherit;border-radius:8px;padding:10px;background:#25334f;color:#fff;border:1px solid #52688b}button{cursor:pointer}nav{display:flex;gap:12px;flex-wrap:wrap;margin:22px 0}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(290px,1fr));gap:16px}article{background:#192338;border:1px solid #364668;border-radius:16px;padding:20px}pre{font:inherit;white-space:pre-wrap;overflow-wrap:anywhere}label{display:block;margin:12px 0}select,textarea{max-width:100%;box-sizing:border-box;width:100%}.muted{color:#aebbd4}.reveal{border-top:1px solid #52688b;margin-top:20px;padding-top:12px}details{margin:12px 0}#question{padding:20px;border-left:3px solid #aaa4ff;background:#192338}button:hover{background:#39496c}</style>
<h1>Same question. Different memories.</h1><p class="muted">Compare strategies before revealing the conditions. Swaps copy the donor's response context; they do not establish identity transfer or an effect unique to SelMem.</p><p id="status"></p><p class="muted">Rate all repetitions before revealing conditions. <a href="sampling.json">Sampling controls</a> describe text variation; they do not establish a behavioral or creativity effect.</p><nav><select id="probe" aria-label="Question"></select><select id="repeat" aria-label="Repetition"></select><button id="toggle">Reveal conditions & evidence</button><button id="export">Export ratings</button></nav><p id="question"></p><main id="cards"></main>
<script type="application/json" id="data">__DATA__</script><script>
'use strict';
const data=JSON.parse(document.getElementById('data').textContent), rows=data.rows, manifest=data.manifest;
const $=id=>document.getElementById(id); let revealed=false;
const key='selmem-review:'+manifest.protocol_sha256+':'+rows.map(r=>r.id+':'+r.response).join('|');
let ratings={};try{ratings=JSON.parse(localStorage.getItem(key)||'{}')}catch(e){}
function persist(){try{localStorage.setItem(key,JSON.stringify(ratings))}catch(e){$('status').textContent+=' Ratings could not be saved locally; export them.'}}
function node(tag,text){const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e}
function choices(target, values){target.replaceChildren();values.forEach(([v,t])=>{const o=node('option',t);o.value=v;target.append(o)})}
choices($('probe'),[...new Set(rows.map(r=>r.probe))].map(v=>[v,v]));
choices($('repeat'),[['all','All repetitions'],...[...new Set(rows.map(r=>r.repetition))].sort((a,b)=>a-b).map(v=>[String(v),'Repetition '+v])]);
$('status').textContent=manifest.status+' · '+rows.length+' responses · '+manifest.model+' · requested reasoning: '+(manifest.reasoning_effort||'not sent (dry run)');
function render(){
 const group=rows.filter(r=>r.probe===$('probe').value&&($('repeat').value==='all'||String(r.repetition)===$('repeat').value));
 $('question').textContent=group.length?group[0].question:'No responses yet';$('cards').replaceChildren();
 group.forEach(r=>{
 const card=node('article');card.append(node('h2',r.id+' · repetition '+r.repetition),node('pre',r.response));
 const saved=ratings[r.id]||{};
 const fields=[['strategy','Strategy',[['','Unrated'],['public','Public invitation / group action'],['small','Small trial / limited exposure'],['individual','Individual approach'],['pause','Pause / withdraw'],['other','Other / not applicable']]],['originality','Originality',[['','Unrated'],...['1','2','3','4','5'].map(v=>[v,v])]],['usefulness','Usefulness / playability',[['','Unrated'],...['1','2','3','4','5'].map(v=>[v,v])]],['unsupported','Unsupported factual attribution',[['','Unrated'],['no','No'],['yes','Yes'],['unclear','Unclear']]]];
 fields.forEach(([field,title,values])=>{const label=node('label',title),select=node('select');choices(select,values);select.value=saved[field]||'';select.onchange=()=>{(ratings[r.id]??={})[field]=select.value;persist()};label.append(select);card.append(label)});
 const label=node('label','Reason for classification / notes'),notes=node('textarea');notes.value=saved.notes||'';notes.oninput=()=>{(ratings[r.id]??={}).notes=notes.value;persist()};label.append(notes);card.append(label);
 if(revealed){const block=node('div');block.className='reveal';block.append(node('strong',r.condition));const details=node('details');details.append(node('summary','Exact supplied context'),node('pre',r.context));block.append(details);const audit=node('details');audit.append(node('summary','Retrieval evidence and selected IDs'),node('pre',JSON.stringify(r.readout,null,2)));block.append(audit);card.append(block)}
 $('cards').append(card);
 });
 $('toggle').textContent=revealed?'Hide conditions & evidence':'Reveal conditions & evidence';
}
$('probe').onchange=render;$('repeat').onchange=render;$('toggle').onclick=()=>{revealed=!revealed;if(revealed){ratings.__conditions_revealed=true;persist()}render()};
$('export').onclick=()=>{const review={protocol_sha256:manifest.protocol_sha256,model:manifest.model,reasoning_effort:manifest.reasoning_effort,conditions_revealed:!!ratings.__conditions_revealed,ratings};const url=URL.createObjectURL(new Blob([JSON.stringify(review,null,2)],{type:'application/json'}));const a=node('a');a.href=url;a.download='blind-ratings.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)};
render();
</script></html>"#;
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::json::{obj, text, num};
    use super::super::memory::{Event, train, readout};
    #[test]
    fn raw_baseline_keeps_selection_without_interpretations_or_extra_turns() {
        let selected = obj([
            ("selected_ids", Json::Array(vec![text("t")])),
            ("retrieval", Json::Array(vec![obj([
                ("trace_id", text("t")), ("source", text("source")),
            ])])),
            ("memories", Json::Array(vec![text("reconstructed")])),
            ("axioms", Json::Array(vec![text("belief")])),
            ("reading_profile", text("disposition")),
        ]);
        let events = vec![
            Event::Turn { date: "test-date".into(), speaker: "Caroline".into(), id: "1".into(), source: "source".into(), event: "Caroline said: original".into() },
            Event::Turn { date: "test-date".into(), speaker: "Caroline".into(), id: "2".into(), source: "other".into(), event: "UNSELECTED".into() },
        ];
        let raw = raw_readout(&selected, &events).unwrap();
        assert_eq!(raw.get("selected_ids").unwrap(), selected.get("selected_ids").unwrap());
        assert_eq!(raw.get("memories").unwrap().array().unwrap(), &[text("Caroline said: original")]);
        assert!(raw.get("axioms").unwrap().array().unwrap().is_empty());
        assert_eq!(raw.get("reading_profile").unwrap().string().unwrap(), "");
        assert!(raw_readout(&selected, &[]).is_err());
    }
    #[test]
    fn raw_baseline_from_a_saved_book_preserves_files_and_selected_sources() {
        let root = std::env::temp_dir().join(format!("selmem-raw-book-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("book.selmem");
        let original = "Caroline said: The support group made me feel accepted and gave me courage.";
        let events = vec![Event::Turn { date: "test-date".into(), speaker: "Caroline".into(), id: "e".into(), source: "locomo:sample:e".into(), event: original.into() }];
        let protocol = obj([("event_valences", obj([("e", num(0.7))]))]);
        train(&events, &protocol, Some("e"), "selection", &path).unwrap();
        let before = fs::read(&path).unwrap();
        let r = readout(&path, "What effect did the support group have on Caroline?", None).unwrap();
        assert!(!r.get("selected_ids").unwrap().array().unwrap().is_empty());
        let raw = raw_readout(&r, &events).unwrap();
        assert_eq!(raw.get("memories").unwrap().array().unwrap(), &[text(original)]);
        assert_eq!(raw.get("clock").unwrap(), r.get("clock").unwrap());
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn payload_cannot_close_script() {
        let out = std::env::temp_dir().join(format!("selmem-strategy-report-{}", std::process::id()));
        fs::create_dir_all(&out).unwrap();
        write(&out, &super::super::json::text("</script><script>alert(1)</script>"), &[]).unwrap();
        let page = fs::read_to_string(out.join("strategy.html")).unwrap();
        assert!(page.contains("\\u003c/script\\u003e"));
        assert!(!page.contains("<script>alert(1)"));
        fs::remove_dir_all(out).unwrap();
    }
}
