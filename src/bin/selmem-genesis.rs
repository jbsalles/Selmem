//! Curated GENESIS qualification and cumulative-learning experiments.
#[path="selmem-genesis/http.rs"] mod http;
#[path="selmem-locomo/json.rs"] mod json;
#[path="selmem-locomo/sha256.rs"] mod sha256;
#[path="selmem-genesis/native.rs"] mod native;
#[path="selmem-genesis/progress.rs"] mod progress;
#[path="selmem-genesis/runner.rs"] mod runner;
#[path="selmem-genesis/transport.rs"] mod transport;
#[path="selmem-genesis/positive_control.rs"] mod positive_control;
#[path="selmem-genesis/learning.rs"] mod learning;
#[path="selmem-genesis/sleep_resume.rs"] mod sleep_resume;
#[path="selmem-genesis/sleep_quality.rs"] mod sleep_quality;
#[path="selmem-genesis/creativity.rs"] mod creativity;
#[path="selmem-genesis/creative_life.rs"] mod creative_life;
#[path="selmem-genesis/creative_preview.rs"] mod creative_preview;
#[path="selmem-genesis/tooling/mod.rs"] mod tooling;
#[cfg(test)] #[path="selmem-genesis/lifecycle_tests.rs"] mod lifecycle_tests;
#[cfg(test)] #[path="selmem-genesis/tests.rs"] mod tests;
fn cli(mut args:Vec<String>)->Result<(),String>{
    if args.is_empty() || args.iter().any(|a|a=="--help"){
        println!("GENESIS: positive | learning | sleep | creative | creative-causal | creative-life | creative-life-causal | creative-preview\n./run.sh run --release --bin selmem-genesis -- positive --config .selmem --llm grok --model grok-4.3 --out DIR --max-http-calls 60\n./run.sh run --release --bin selmem-genesis -- learning --config .selmem --llm grok --model grok-4.3 --source QUALIFIED_DIR --out DIR --observation-evidence on|off --max-http-calls 800\nsleep --source EVIDENCE_ON_DIR --locomo FIXTURE.json --out DIR --config .selmem --llm grok --max-http-calls 3000 [--resume-from DIR] [--prepare-only]\ncreative --locomo FIXTURE.json --out DIR [--mock] [--prepare-only] [--repeats 3] [--context-words 600] [--max-http-calls 3000]\ncreative-causal --source CREATIVE_DIR --plan PLAN.json --out DIR [--mock]\ncreative-preview --fixture FIXTURE.json --out DIR [--resume] [--prepare-only] [--mock]\nOffline tools: tools prepare-creative|prepare-preview|prepare-life|prepare-sleep|rate-creative|rate-preview|rate-life|code-mechanisms|combine-ratings|life-profile\nAll collection commands support --mock. Mock runs are technical rehearsals. Outputs must be fresh.");return Ok(());
    }
    let command=args.remove(0);
    if command=="tools"{return tooling::cli(args);}
    if !["positive","learning","sleep","creative","creative-causal","creative-life","creative-life-causal","creative-preview"].contains(&command.as_str()){return Err(format!("retired or unknown command {command}; use positive|learning|sleep|creative|creative-causal --help"));}
    if let Some(i)=args.iter().position(|a|a=="--config"){
        let path=args.get(i+1).ok_or("--config requires a path")?;
        selmem::Config::load_path(std::path::Path::new(path)).map_err(|e|format!("runtime config: {e}"))?;
    }
    match command.as_str(){"creative-preview"=>creative_preview::cli(args),"creative-life"=>creative_life::cli(args),"creative-life-causal"=>creative_life::causal_cli(args),"positive"=>positive_control::cli(args),"learning"=>learning::cli(args),"sleep"=>sleep_quality::cli(args),"creative"=>creativity::cli(args,false),"creative-causal"=>creativity::cli(args,true),_=>unreachable!()}
}
fn main(){if let Err(e)=cli(std::env::args().skip(1).collect()){progress::line(format!("ERROR {e}"));eprintln!("GENESIS: {e}");std::process::exit(1);}}
