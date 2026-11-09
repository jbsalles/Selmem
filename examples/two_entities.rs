use selmem::EncodeInput;
use selmem::{fingerprint, singularity_distance, Channel, EntityProfile, SelectiveMemory};

struct Scene {
    event: &'static str,
    valence: f32,
    arousal: f32,
    disgust: f32,
    self_relevance: f32,
    schema: Option<&'static str>,
    channel: Channel,
    utility: f32,
    permanence: f32,
}

fn scenes() -> Vec<Scene> {
    vec![
        Scene {
            event: "You stayed. Rain on the window, and you did not look for an excuse to leave.",
            valence: 0.72,
            arousal: 0.55,
            disgust: 0.0,
            self_relevance: 0.9,
            schema: Some("loyalty"),
            channel: Channel::Selfhood,
            utility: 0.4,
            permanence: 0.4,
        },
        Scene {
            event:
                "You cancelled at the last moment, for no reason, after I had prepared everything.",
            valence: -0.62,
            arousal: 0.7,
            disgust: 0.48,
            self_relevance: 0.85,
            schema: Some("abandon"),
            channel: Channel::Selfhood,
            utility: 0.4,
            permanence: 0.3,
        },
        Scene {
            event: "We talked about almost nothing for an hour. That was enough.",
            valence: 0.4,
            arousal: 0.25,
            disgust: 0.0,
            self_relevance: 0.6,
            schema: Some("presence"),
            channel: Channel::Selfhood,
            utility: 0.4,
            permanence: 0.0,
        },
        Scene {
            event: "You laughed at what I had told you in confidence.",
            valence: -0.55,
            arousal: 0.65,
            disgust: 0.52,
            self_relevance: 0.8,
            schema: Some("humiliation"),
            channel: Channel::Selfhood,
            utility: 0.4,
            permanence: 0.0,
        },
        Scene {
            event: "The appointment is Tuesday at 10, room B.",
            valence: 0.0,
            arousal: 0.1,
            disgust: 0.0,
            self_relevance: 0.1,
            schema: Some("agenda"),
            channel: Channel::World,
            utility: 0.9,
            permanence: 0.9,
        },
        Scene {
            event: "A throwaway remark about the weather.",
            valence: 0.05,
            arousal: 0.05,
            disgust: 0.0,
            self_relevance: 0.05,
            schema: None,
            channel: Channel::Selfhood,
            utility: 0.05,
            permanence: 0.0,
        },
    ]
}

fn run(mut mem: SelectiveMemory) -> SelectiveMemory {
    println!("\n=== {} ===", mem.profile.name);
    for s in scenes() {
        let mut input = EncodeInput::new(s.event);
        input.valence = s.valence;
        input.arousal = s.arousal;
        input.disgust = s.disgust;
        input.self_relevance = s.self_relevance;
        input.schema = s.schema.map(|x| x.to_string());
        input.channel = s.channel;
        input.utility = s.utility;
        input.permanence = s.permanence;
        let d = mem.live_with(input);
        let mark = if d.kept { "kept" } else { "dropped at once" };
        let preview: String = s.event.chars().take(64).collect();
        println!("  [{mark} | S={:.2}] {preview}", d.score);
    }
    let report = mem.sleep();
    println!(
        "  -- sleep -- sculpted={} merged={} extinguished={} cold={} myth={} axioms={}",
        report.sculpted.len(),
        report.merged,
        report.extinguished,
        report.cold,
        report.myth,
        report.axioms.len()
    );
    for ax in mem.who_am_i() {
        println!("  axiom ({:.2}): {}", ax.strength, ax.statement);
    }
    println!("  -- recall: that evening --");
    for rec in mem.remember("that evening, you and I") {
        println!("  • {}", rec.narrative);
        println!("    [{}]", rec.disclaimer);
    }
    mem
}

fn main() {
    let claire = run(SelectiveMemory::new(EntityProfile::tender("Claire")));
    let silas = run(SelectiveMemory::new(EntityProfile::austere("Silas")));
    let d = singularity_distance(&fingerprint(&claire), &fingerprint(&silas));
    println!("\nClaire–Silas singularity distance: {d:.3}  (0 = identical, 1 = disjoint lives)");
}
