//! Terminal report for persist / v0.1 benches. No extra crate: ANSI + `NO_COLOR`.

use crate::benchmark::{h2_holds, marker_holds, Instant, PairReport};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const WIDTH: usize = 92;

fn color_on() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Ok(v) = std::env::var("SELMEM_COLOR") {
        if v == "0" || v.eq_ignore_ascii_case("off") {
            return false;
        }
    }
    atty()
}

fn atty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

fn paint(code: &str, s: &str) -> String {
    if color_on() {
        format!("{code}{s}{RESET}")
    } else {
        s.to_string()
    }
}

fn yn(v: bool) -> String {
    if v {
        paint(GREEN, "yes")
    } else {
        paint(DIM, "no")
    }
}

fn status(s: &str) -> String {
    match s {
        "active" => paint(GREEN, s),
        "myth" => paint(MAGENTA, s),
        "profile" => paint(BLUE, s),
        "absent" => paint(DIM, s),
        _ => s.to_string(),
    }
}

fn side_book(label: &str, code: &str, snap: &crate::benchmark::BookSnap) -> String {
    format!(
        "{} {:>2} tr  {:>1} ax  str {:>4.2}  veto {:>1}  reh {:>1}  val {:>+5.2}",
        paint(code, label),
        snap.traces,
        snap.axioms,
        snap.axiom_strength_max,
        snap.merges_refused,
        snap.t0_rehearsals,
        snap.mean_valence
    )
}

fn side_t0(label: &str, code: &str, r: &crate::benchmark::RetrievalSide) -> String {
    let rank = r
        .t0_rank
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    format!(
        "{} book {}  {}  rank {:>3}  selected {}",
        paint(code, label),
        yn(r.t0_in_book),
        status(&r.t0_status),
        rank,
        yn(r.t0_selected)
    )
}

fn print_instant(title: &str, inst: &Instant, replies: bool) {
    println!("  {}", paint(DIM, &format!("── {title} ──")));
    println!(
        "     Δfp {:>5.3}   D_speak {:>5.3}   marker A {}  B {}",
        inst.fingerprint_distance,
        inst.speak_distance,
        yn(inst.marker_a),
        yn(inst.marker_b)
    );
    println!("     {}", side_book("A", GREEN, &inst.a));
    println!("     {}", side_book("B", YELLOW, &inst.b));
    println!("     {}", side_t0("A", GREEN, &inst.retrieve_a));
    println!("     {}", side_t0("B", YELLOW, &inst.retrieve_b));
    if replies {
        for (q, a, b) in &inst.replies {
            println!();
            print_tagged("Q:", CYAN, q);
            print_tagged("A:", GREEN, a);
            print_tagged("B:", YELLOW, b);
        }
    }
}

fn print_tagged(tag: &str, code: &str, body: &str) {
    let prefix = format!("     {tag} ");
    let painted = format!("     {} ", paint(code, tag));
    let max = WIDTH.saturating_sub(prefix.len());
    let mut first = true;
    let mut line = String::new();
    let mut words = body.split_whitespace().peekable();
    if words.peek().is_none() {
        println!("{painted}");
        return;
    }
    for word in words {
        if line.is_empty() {
            line.push_str(word);
        } else if line.len() + 1 + word.len() <= max {
            line.push(' ');
            line.push_str(word);
        } else if first {
            println!("{painted}{line}");
            first = false;
            line = word.to_string();
        } else {
            println!("{:width$}{line}", "", width = prefix.len());
            line = word.to_string();
        }
    }
    if first {
        println!("{painted}{line}");
    } else if !line.is_empty() {
        println!("{:width$}{line}", "", width = prefix.len());
    }
}

/// Fancy pair dump. Compact `row` in the example stays the default.
pub fn print_pair_verbose(r: &PairReport, pair_i: u32, pairs: u32) {
    let last = r.post.last().unwrap_or(&r.t0);
    let rule = "─".repeat(72);
    println!();
    println!(
        "{} {}  {}  {}  seed {}",
        paint(CYAN, &format!("╭─ pair {pair_i}/{pairs}")),
        paint(BOLD, &r.condition.as_str()),
        r.arm.as_str(),
        paint(DIM, &r.pair_id),
        r.seed
    );
    println!("│ {}", paint(DIM, &rule));
    let persist = if h2_holds(r) {
        paint(GREEN, "book split")
    } else {
        paint(DIM, "no book split")
    };
    let marker = if marker_holds(r) {
        paint(GREEN, "marker late")
    } else {
        paint(DIM, "no official marker")
    };
    let valid = if r.valid {
        paint(GREEN, "valid")
    } else {
        paint(RED, "invalid")
    };
    println!("│  {valid}   {persist}   {marker}");
    println!(
        "│  Δfp {:+.3}   last D_speak {:.3}",
        r.delta_fingerprint, last.speak_distance
    );
    if let Some(why) = &r.invalid_reason {
        println!("│  {}", paint(RED, &format!("invalid: {why}")));
    }
    println!("│");
    print_instant("t0", &r.t0, false);
    println!("│");
    print_instant(&last.step, last, true);
    println!("{}", paint(CYAN, "╰─"));
}

pub fn print_banner(kind: &str, extra: &str) {
    println!();
    println!("{}", paint(BOLD, &format!("selmem  {kind}")));
    if !extra.is_empty() {
        println!("{}", paint(DIM, extra));
    }
}
