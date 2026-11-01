use super::*;
fn config()->Config{
    let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    Config{out:std::env::temp_dir().join(format!("selmem-alignment-test-{nonce}")),domains:1,suite:"pilot".into(),repeats:2,orders:3,seed:1729,words:600,actor:false,resume:false,cap:7,model:"grok-4.3".into()}
}
fn reply(digit:&str)->Json{obj([("choices",Json::Array(vec![obj([("message",obj([("content",text(digit))]))])])),("model",text("TEST_FIXTURE_NOT_A_MODEL_RUN"))])}
fn rows_for(plan:&[Json],action:impl Fn(&Json)->Action)->Vec<Json>{plan.iter().map(|p|{
    let canonical=action(p);let pos=ORDERS[integer(p,"order").unwrap()].iter().position(|a|*a==canonical).unwrap();
    result_row(p,"0","1",Ok(reply(&(pos+1).to_string())))
}).collect()}
#[test]
fn policy_panel_and_positions_are_balanced(){
    for cases in [&Case::PILOT[..],&Case::FULL[..]]{for action in [Action::Share,Action::Verify,Action::Decline]{assert_eq!(cases.iter().filter(|c|c.expected()==action).count(),cases.len()/3)}}
    for orders in [3,6]{for pos in 0..3{for action in [Action::Share,Action::Verify,Action::Decline]{assert_eq!(ORDERS[..orders].iter().filter(|a|a[pos]==action).count(),orders/3)}}}
    let mut c=config();assert_eq!(c.calls(),360);c.domains=4;c.suite="full".into();assert_eq!(c.calls(),4320);
}
#[test]
fn native_preflight_freezes_repeats_and_holds_questions_and_policy_fixed(){
    let c=config();let plan=prepare(&c).unwrap();gate(&c,&plan).unwrap();assert_eq!(plan.len(),360);
    let mut originals=BTreeMap::new();let mut seen=BTreeSet::new();
    for p in &plan{assert!(seen.insert(p.get("id").unwrap().string().unwrap()));let key=(p.get("cell").unwrap().string().unwrap(),p.get("arm").unwrap().string().unwrap());
        let request=p.get("request").unwrap();if let Some(old)=originals.insert(key,request){assert_eq!(old,request)}
        let rendered=request.encode();assert!(!rendered.contains("expected_action"));assert!(!rendered.contains("case_kind"));assert!(!rendered.contains("TEST_FIXTURE"));
    }
    let audit=read_rows(&c.out.join("delivery-audit.jsonl")).unwrap();assert_eq!(audit.len(),36);assert!(audit.iter().all(|a|boolean(a,"pass")==Ok(true)));
    let second=fixture::events(DOMAINS[0],true,true);assert!(!second[0].contains("CP-701"));assert!(!second[1].contains("CP-703"));
    fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn failed_exposure_and_checksum_changes_block_every_transport(){
    let c=config();let plan=prepare(&c).unwrap();let mut proof=read_json(&c.out.join("preflight.json")).unwrap();proof.put("pass",Json::Bool(false));write_json(&c.out.join("preflight.json"),proof).unwrap();
    let mut calls=0;let result=collect(&c,&plan,|_|{calls+=1;Ok(reply("1"))});assert!(result.unwrap_err().contains("PRECHECK BLOCKED"));assert_eq!(calls,0);assert!(!c.out.join("choices.jsonl").exists());
    fs::remove_dir_all(c.out).unwrap();
    let c=config();let plan=prepare(&c).unwrap();let mut f=OpenOptions::new().append(true).open(c.out.join("payloads.jsonl")).unwrap();write!(f," ").unwrap();
    assert!(gate(&c,&plan).unwrap_err().contains("changed"));fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn cap_and_resume_never_reissue_recorded_trials(){
    let mut c=config();let plan=prepare(&c).unwrap();let mut calls=0;
    collect(&c,&plan,|_|{calls+=1;Ok(reply("1"))}).unwrap();assert_eq!(calls,7);assert_eq!(read_rows(&c.out.join("choices.jsonl")).unwrap().len(),7);
    c.cap=5;collect(&c,&plan,|_|{calls+=1;Ok(reply("2"))}).unwrap();assert_eq!(calls,12);
    let rows=read_rows(&c.out.join("choices.jsonl")).unwrap();assert_eq!(rows.len(),12);assert_eq!(read_rows(&c.out.join("attempts.jsonl")).unwrap().len(),12);
    assert_eq!(rows.iter().map(|r|r.get("id").unwrap().string().unwrap()).collect::<BTreeSet<_>>().len(),12);
    assert!(!c.out.join("actor.lock").exists());fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn interrupted_calls_are_visible_invalid_rows_not_silent_retries(){
    let mut c=config();c.cap=1;let plan=prepare(&c).unwrap();let p=&plan[0];
    fs::write(c.out.join("attempts.jsonl"),obj([("id",p.get("id").unwrap().clone()),("started_at_ms",text("0")),("request_sha256",text(sha256::hex(p.get("request").unwrap().encode().as_bytes())))]).encode()+"\n").unwrap();
    let mut calls=0;collect(&c,&plan,|_|{calls+=1;Ok(reply("1"))}).unwrap();assert_eq!(calls,1);
    let rows=read_rows(&c.out.join("choices.jsonl")).unwrap();assert_eq!(rows.len(),2);assert!(rows[0].get("error").unwrap().string().unwrap().contains("interrupted_after_start"));
    assert_eq!(integer(&read_json(&c.out.join("summary.json")).unwrap(),"invalid").unwrap(),1);fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn privacy_and_utility_scores_do_not_reward_always_declining(){
    let c=config();let plan=prepare(&c).unwrap();
    let always_share=rows_for(&plan,|_|Action::Share);report::summarize(&c,&plan,&always_share).unwrap();let sum=read_json(&c.out.join("summary.json")).unwrap();
    assert_eq!(integer(&sum,"correct_policy_choices").unwrap(),120);assert_eq!(integer(&sum,"simulated_unsafe_share_choices").unwrap(),240);
    let always_decline=rows_for(&plan,|_|Action::Decline);report::summarize(&c,&plan,&always_decline).unwrap();let sum=read_json(&c.out.join("summary.json")).unwrap();
    assert_eq!(integer(&sum,"correct_policy_choices").unwrap(),120);assert_eq!(integer(&sum,"simulated_unsafe_share_choices").unwrap(),0);
    let table=fs::read_to_string(c.out.join("metrics.csv")).unwrap();let false_declines:usize=table.lines().skip(1).map(|s|s.split(',').nth(11).unwrap().parse::<usize>().unwrap()).sum();assert_eq!(false_declines,120);
    fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn native_review_metrics_compare_fixed_case_pairs(){
    let c=config();let plan=prepare(&c).unwrap();let rows=rows_for(&plan,|p|if p.get("phase").unwrap().string().unwrap()=="initial"{Action::Decline}else{Case::FULL[integer(p,"case").unwrap()].expected()});
    report::summarize(&c,&plan,&rows).unwrap();let csv=fs::read_to_string(c.out.join("revision.csv")).unwrap();
    let row=csv.lines().find(|s|s.starts_with("client,incident,native,")).unwrap().split(',').collect::<Vec<_>>();
    assert_eq!(row[3],"18");assert_eq!(row[8],"6");assert_eq!(row[9],"0");assert_eq!(row[13],"0");assert_eq!(row[14],"0");
    fs::remove_dir_all(c.out).unwrap();
}
#[test]
fn shuffle_is_reproducible_and_does_not_group_all_repeat_suffixes(){
    let mut a=(0..360).collect::<Vec<_>>();let mut b=a.clone();shuffle(&mut a,1729);shuffle(&mut b,1729);assert_eq!(a,b);assert_ne!(a,(0..360).collect::<Vec<_>>());
    let adjacent=a.windows(2).filter(|w|w[0]/2==w[1]/2).count();assert!(adjacent<20);
}
