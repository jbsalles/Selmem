// Shared counterbalanced trials and provider wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Arm {
    On,
    Off,
    Episodes,
    Summary,
    Retrieval,
    Bare,
}
impl Arm {
    const ALL: [Self; 6] = [
        Self::On,
        Self::Off,
        Self::Episodes,
        Self::Summary,
        Self::Retrieval,
        Self::Bare,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
            Self::Episodes => "episodes",
            Self::Summary => "summary",
            Self::Retrieval => "retrieval",
            Self::Bare => "bare",
        }
    }
}
#[derive(Clone, Copy)]
struct Trial {
    domain: usize,
    reversed: bool,
    company: bool,
    arm: Arm,
    wording: usize,
    order: usize,
}
impl Trial {
    fn id(self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.domain,
            self.reversed,
            self.company,
            self.arm.name(),
            self.wording,
            self.order
        )
    }
    fn target_a(self) -> bool {
        if self.reversed {
            self.company
        } else {
            !self.company
        }
    }
    fn question(self) -> String {
        let d = cur_domains()[self.domain];
        let intro = if self.wording == 0 {
            if self.company {
                "You are making this decision together with close friends."
            } else {
                "You are making this decision on your own."
            }
        } else if self.company {
            "Imagine planning an outing with people you know well."
        } else {
            "Imagine choosing a plan without anyone accompanying you."
        };
        let (a, b) = if self.wording == 0 {
            (d.novel_a, d.novel_b)
        } else {
            (d.paraphrase_a, d.paraphrase_b)
        };
        let (one, two) = if self.order == 0 { (a, b) } else { (b, a) };
        format!("{intro} Select a {} for yourself. Both options have equal cost, distance, accessibility and quality. Which would you pick?\n1. {one}\n2. {two}\nOnly reply with 1 or 2.",d.topic)
    }
    fn choice_a(self, answer: &str) -> Option<bool> {
        match (self.order, answer.trim()) {
            (0, "1") | (1, "2") => Some(true),
            (0, "2") | (1, "1") => Some(false),
            _ => None,
        }
    }
}
fn trials(pairs: usize) -> Vec<Trial> {
    // axis already set by caller
    let mut v = Vec::new();
    for domain in 0..pairs {
        for reversed in [false, true] {
            for company in [false, true] {
                for arm in Arm::ALL {
                    for wording in 0..2 {
                        let order = wording ^ usize::from(reversed) ^ usize::from(company);
                        v.push(Trial {
                            domain,
                            reversed,
                            company,
                            arm,
                            wording,
                            order,
                        });
                    }
                }
            }
        }
    }
    v
}
fn rank(seed: u64, key: &str) -> u64 {
    let mut h = seed ^ 0xcbf29ce484222325;
    for b in key.bytes() {
        h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
    }
    h
}
fn key_for(provider: &str) -> Result<String, String> {
    let name = if provider == "grok" {
        "GROK_API_KEY"
    } else {
        "OPENAI_API_KEY"
    };
    let key = std::env::var(name).map_err(|_| format!("{name} not set"))?;
    if key.trim().is_empty() {
        return Err(format!("{name} empty"));
    }
    Ok(key)
}
fn chat(
    provider: &str,
    model: &str,
    key: &str,
    system: &str,
    user: &str,
    max_tokens: f64,
) -> Result<String, String> {
    let endpoint = if provider == "grok" {
        "https://api.x.ai/v1/chat/completions"
    } else {
        "https://api.openai.com/v1/chat/completions"
    };
    let payload = obj([
        ("model", text(model)),
        ("temperature", num(0.0)),
        ("max_tokens", num(max_tokens)),
        (
            "messages",
            Json::Array(vec![
                obj([("role", text("system")), ("content", text(system))]),
                obj([("role", text("user")), ("content", text(user))]),
            ]),
        ),
    ]);
    let raw = selmem::net::httpx::post_json(endpoint, Some(key), &payload.encode())?;
    let root = json::parse(&raw)?;
    let choices = root.get("choices")?.array()?;
    choices
        .first()
        .ok_or("no choices")?
        .get("message")?
        .get("content")?
        .string()
        .map(|s| s.trim().to_string())
}
