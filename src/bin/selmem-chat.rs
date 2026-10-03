use std::env;
use std::io::{self, BufRead, Write};

use selmem::{Config, EntityProfile, HttpEmbedder, HttpNarrator, SelectiveMemory};

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg = Config::get();
    let path = cfg.resolve_or(flag(&args, "--path"), "path", "claire.db");
    let name = cfg.resolve_or(flag(&args, "--name"), "name", "Claire");
    let kind = cfg.resolve_or(flag(&args, "--profile"), "profile", "tender");
    let key = cfg.resolve(flag(&args, "--api-key"), "api_key");

    let profile = if kind == "austere" {
        EntityProfile::austere(name)
    } else {
        EntityProfile::tender(name)
    };
    let mut mem = SelectiveMemory::open(&path, profile).expect("memory");
    let mouth = match cfg.mouth(flag(&args, "--llm"), flag(&args, "--model"), flag(&args, "--plug"), "llama3") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    if let Some(mouth) = mouth {
        if let Some(n) = HttpNarrator::parse(&mouth.url, mouth.model, mouth.api_key.or(key.clone())) {
            mem = mem.with_narrator(Box::new(n));
            eprintln!("LLM voice attached");
        }
    }
    if let Some(url) = cfg.resolve(flag(&args, "--embed"), "embed") {
        if let Some(e) = HttpEmbedder::parse(&url, "text-embedding-3-small", key.clone()) {
            mem = mem.with_embedder(Box::new(e));
            eprintln!("HTTP embeddings attached");
        }
    }

    eprintln!("{} écoute. /sleep /who /mood /talk /forget /quit", mem.profile.name);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            "/quit" | "/exit" => break,
            "/sleep" => {
                let snap = mem.talk.turns.clone();
                let topic = mem.talk.topic.clone();
                let (kept, _) = mem.keep_sitting();
                mem.clear_talk();
                let r = mem.sleep();
                mem.fade_sitting();
                for t in &snap {
                    mem.talk.record(&t.user, &t.reply);
                }
                mem.talk.topic = topic;
                let _ = mem.save();
                println!("(kept sitting {})", kept);
                println!(
                    "(nuit) sculpted={} merged={} extinguished={} axioms={}",
                    r.sculpted.len(),
                    r.merged,
                    r.extinguished,
                    r.axioms.len()
                );
            }
            "/who" => {
                for a in mem.who_am_i() {
                    println!("· {}", a.statement);
                }
                if mem.who_am_i().is_empty() {
                    println!("(pas encore d'axiome)");
                }
            }
            "/mood" => {
                println!(
                    "mood v={:.2} a={:.2} d={:.2}",
                    mem.mood.valence, mem.mood.arousal, mem.mood.disgust
                );
            }
            "/talk" => {
                mem.refresh_talk();
                match mem.talk.topic.as_deref() {
                    Some(t) => println!("fil: {t}"),
                    None => println!("(pas de fil)"),
                }
                for t in &mem.talk.turns {
                    println!("  H: {}", t.user);
                    println!("  S: {}", t.reply);
                }
            }
            "/forget" => {
                mem.clear_talk();
                println!("(fil oublié)");
            }
            _ => {
                let reply = mem.speak(line);
                let _ = mem.save();
                println!("{reply}");
            }
        }
        let _ = stdout.flush();
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find_map(|w| {
        if w[0] == name {
            Some(w[1].clone())
        } else {
            None
        }
    })
}
