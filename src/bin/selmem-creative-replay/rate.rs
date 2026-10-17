use super::*;
pub fn template(dir:&Path,out:&Path)->Result<()>{
    let rows=load(&dir.join("blind-proposals.json"))?;
    fresh(out,&arr(rows.array()?.iter().map(|p|obj([("blind_id",p.get("blind_id").unwrap().clone()),("judge",text("REPLACE_WITH_INDEPENDENT_JUDGE_ID")),("valid",Json::Null),("utility",Json::Null),("originality",Json::Null),("rationale",text(""))]))))
}
pub fn analyze(dir:&Path,ratings:&Json)->Result<Json>{
    let run=load(&dir.join("replay.json"))?;
    require(!boolean(&run,"mock")?,"mock outputs are not experimental evidence")?;
    let proposals=load(&dir.join("all-proposals.json"))?;
    let proposals=proposals.array()?;
    require(proposals.len()==72,"72 proposals required")?;
    let ids:BTreeSet<_>=proposals.iter().map(|p|s(p,"blind_id").unwrap()).collect();
    let mut by_id:BTreeMap<String,Vec<(String,f64,f64)>>=BTreeMap::new();
    let mut pairs=BTreeSet::new();
    let mut judges=BTreeSet::new();
    for r in ratings.array()?{
        let id=s(r,"blind_id")?;
        let judge=s(r,"judge")?;
        require(ids.contains(id),"unknown blind ID")?;
        require(!judge.trim().is_empty()&&!judge.contains("REPLACE"),"real judge identifier required")?;
        require(pairs.insert((id.to_string(),judge.to_string())),"duplicate judge/proposal rating")?;
        let valid=boolean(r,"valid")?;
        let utility=n(r,"utility")?;
        let originality=n(r,"originality")?;
        require([utility,originality].iter().all(|v|v.is_finite()&&(0.0..=4.0).contains(v)),"scores must be in 0..4")?;
        require(!s(r,"rationale")?.trim().is_empty(),"rating rationale required")?;
        require(valid||(utility==0.0&&originality==0.0),"constraint failure must score zero")?;
        judges.insert(judge.to_string());
        by_id.entry(id.into()).or_default().push((judge.into(),utility,originality));
    }
    require(judges.len()>=2,"at least two independent judges required")?;
    let mut aggregates:BTreeMap<(String,String),Vec<(f64,f64)>>=BTreeMap::new();
    for p in proposals{
        let id=s(p,"blind_id")?;
        let rows=by_id.get(id).ok_or("missing proposal ratings")?;
        let present:BTreeSet<_>=rows.iter().map(|r|r.0.clone()).collect();
        require(present==judges,"every judge must rate every proposal")?;
        let count=rows.len() as f64;
        aggregates.entry((s(p,"history")?.into(),s(p,"condition")?.into())).or_default().push((rows.iter().map(|r|r.1).sum::<f64>()/count,rows.iter().map(|r|r.2).sum::<f64>()/count));
    }
    let means: BTreeMap<_,_>=aggregates.iter().map(|(k,rows)|(k.clone(),(rows.iter().map(|r|r.0).sum::<f64>()/rows.len() as f64,rows.iter().map(|r|r.1).sum::<f64>()/rows.len() as f64))).collect();
    let mut comparisons=vec![];
    for history in ["conv-26","conv-30"]{
        let full=means.get(&(history.into(),"after_full_guarded".into())).ok_or("missing full arm")?;
        for condition in CONDITIONS.into_iter().filter(|c|*c!="after_full_guarded"){
            let base=means.get(&(history.into(),condition.into())).ok_or("missing comparator")?;
            comparisons.push(obj([("history",text(history)),("comparator",text(condition)),("utility_delta",num(full.0-base.0)),("originality_delta",num(full.1-base.1))]));
        }
    }
    Ok(obj([("status",text("rated_exploratory")),("judges",arr(judges.into_iter().map(text))),("comparisons",arr(comparisons)),("limitation",text("Two histories, three tasks, two repeats: descriptive comparisons only; no causal or significance claim. Judge independence must be established outside this tool."))]))
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn refuses_mock_evidence(){
        let d=std::env::current_dir().unwrap().join(format!("rate-test-{}",std::process::id()));
        fs::create_dir(&d).unwrap();
        fresh(&d.join("replay.json"),&obj([("mock",Json::Bool(true))])).unwrap();
        assert!(analyze(&d,&arr([])).unwrap_err().contains("mock"));
        fs::remove_dir_all(d).unwrap();
    }
}
#[cfg(test)]mod coverage_tests{
    use super::*;
    #[test]fn requires_complete_independent_ratings_and_constraint_zero(){
        let dir=std::env::current_dir().unwrap().join(format!("ratings-test-{}",std::process::id()));
        fs::create_dir(&dir).unwrap();
        fresh(&dir.join("replay.json"),&obj([("mock",Json::Bool(false))])).unwrap();
        let mut proposals=vec![];
        let mut ratings=vec![];
        for history in ["conv-26","conv-30"]{
            for condition in CONDITIONS{
                for i in 0..6{
                    let id=format!("{history}-{condition}-{i}");
                    proposals.push(obj([("blind_id",text(&id)),("history",text(history)),("condition",text(condition))]));
                    for judge in ["one","two"]{
                        ratings.push(obj([("blind_id",text(&id)),("judge",text(judge)),("valid",Json::Bool(true)),("utility",num(3)),("originality",num(2)),("rationale",text("Synthetic unit-test rating"))]));
                    }
                }
            }
        }
        fresh(&dir.join("all-proposals.json"),&arr(proposals)).unwrap();
        let report=analyze(&dir,&arr(ratings.clone())).unwrap();
        assert_eq!(array(&report,"comparisons").unwrap().len(),10);
        let mut missing=ratings.clone();
        missing.pop();
        assert!(analyze(&dir,&arr(missing)).is_err());
        ratings[0].put("valid",Json::Bool(false));
        assert!(analyze(&dir,&arr(ratings)).unwrap_err().contains("zero"));
        fs::remove_dir_all(dir).unwrap();
    }
}
