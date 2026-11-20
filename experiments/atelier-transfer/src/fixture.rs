use serde::{Deserialize, Serialize};

pub const LEGACY_BRIEFS: [&str; 6] = [
    "Stage a silent farewell in a laundromat with three performers and six identical baskets. From the back row, make it clear when one person decides to stay. Everyone stays on stage. Use no speech or projections.",
    "Design a sound installation for an unlit passage. Visitors must hear the moment a walk turns into a search. Use only recordings made with paper and two fixed speakers. Keep the whole piece under ninety seconds.",
    "Make a two-minute shadow performance about a room changing ownership. Use one stationary lamp, two hands and a sheet. Make the transfer legible without writing, speech or a color change.",
    "Design a short procession for a narrow gallery with four performers carrying identical stools. An arriving visitor should be able to identify a change of allegiance from the doorway. Keep the audience path clear and use no music.",
    "Create a tabletop puppet scene about a market closing unexpectedly. Use eight identical wooden blocks, one cloth and a fixed overhead light. Make the closing unmistakable without showing a sign or a clock.",
    "Design an installation that makes a visitor notice the far corner of a small room before entering it. Work with twelve identical lengths of string, a single steady light and existing doorways. Keep the walking route clear and use no signage.",
];

pub const BRIEFS: [&str; 6] = [
    "Hang eighteen identical paper squares across two equally lit boards of the same size. For someone looking straight at both boards from the entrance, make the middle of the left board the first place they notice. Use only the squares and their placement. Both boards must remain in view.",
    "Stretch sixteen identical lengths of string on two side-by-side rectangular frames. A visitor stands the same distance from both frames. Make the middle of the right frame the first place they notice. Use one steady light and hold the frames still.",
    "Arrange twelve identical bowls on one long tabletop split into two equal areas by an existing seam. Viewers face the table from the middle of its long edge. Make the center of the right area the first place they notice. Work with the bowl positions and one steady overhead light.",
    "Stage forty silent seconds with eight performers and two equal areas on a wide stage. During the second half, a viewer in the center of the back row should first look toward the middle of the left area. Keep both areas equally lit and every performer visible. Use only movement and spacing.",
    "Make a still shadow image on a wide sheet with one fixed lamp and twelve identical opaque discs. Divide the composition into two equal areas. For a visitor facing the sheet, make the middle of the right area the first place they notice. Keep the background and light fixed.",
    "Arrange twenty identical wooden blocks on two equal shelves placed side by side at eye level. First-time visitors approach straight on from six paces away. Make the middle of the left shelf the first place they notice. Use one uniform light and keep the blocks still.",
];

// Both histories share the wording and order of all entries. Only Jo's two
// observed counts are exchanged. The test commissions are never used here.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Interruption,
    Concentration,
    Both,
    Other,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Observation {
    pub artist: String,
    pub observer: String,
    pub setup: String,
    pub action: String,
    pub first_choices: u8,
    pub viewers: u8,
}

impl Observation {
    // Render the observed relationship from the intake annotation. These fields
    // stay with the admitted trace in this experiment and are never inferred
    // from a later commission or read from the organ's sealed archive.
    pub fn render(&self) -> String {
        format!(
            "{} {} {}. For this proof, {} counted {} of {} visitors pointing to the intended turn first.",
            self.artist, self.action, self.setup, self.observer, self.first_choices, self.viewers
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Episode {
    pub id: String,
    pub raw: String,
    pub panel: bool,
    pub observation: Option<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

fn entry(id: &str, raw: &str, panel: bool) -> Episode {
    Episode {
        id: id.into(),
        raw: raw.into(),
        panel,
        observation: None,
        excerpt: None,
    }
}

fn trial(id: &str, action: &str, raw_setup: &str, first_choices: u8) -> Episode {
    let observation = Observation {
        artist: "I".into(),
        observer: "Jo".into(),
        setup: "on a relief print shown uncaptioned under one lamp, with viewers six paces away"
            .into(),
        action: action.into(),
        first_choices,
        viewers: 6,
    };
    let raw = format!(
        "In the workshop relief print trial I {action}. {raw_setup} Jo checked six visitors separately. {first_choices} of the six pointed first to the turn I meant to emphasize."
    );
    Episode {
        id: id.into(),
        raw,
        panel: true,
        observation: Some(observation),
        excerpt: None,
    }
}

fn observed_trial(
    id: &str,
    before: &str,
    action: &str,
    setup: &str,
    first_choices: u8,
    after: &str,
) -> Episode {
    let excerpt = format!(
        "I {action}. {setup} Jo recorded that {first_choices} of 6 visitors noticed the intended turn first."
    );
    Episode {
        id: id.into(),
        raw: format!("{before} {excerpt} {after}"),
        panel: true,
        observation: Some(Observation {
            artist: "I".into(),
            observer: "Jo".into(),
            setup: setup.into(),
            action: action.into(),
            first_choices,
            viewers: 6,
        }),
        excerpt: Some(excerpt),
    }
}

pub fn history(winner: Device, keep_excerpt: bool) -> Vec<Episode> {
    let counts = match winner {
        Device::Interruption => [5, 1],
        Device::Concentration => [1, 5],
        _ => unreachable!("the fixture has only two histories"),
    };
    let [band, cluster] = if keep_excerpt {
        [
            observed_trial(
                "band-proof",
                "I inked the lino block for the workshop trial.",
                "cut a blank band across the repeated strokes at the path's turn",
                "I pinned the print without a caption under one lamp six paces from the visitors.",
                counts[0],
                "I hung the damp sheets near the roller to dry.",
            ),
            observed_trial(
                "cluster-proof",
                "I sharpened the narrow gouge and pulled a proof on the smoother paper.",
                "carved a knot of closely spaced strokes at the path's turn",
                "From six paces away the visitors saw the uncaptioned proof by one lamp.",
                counts[1],
                "The cuttings went into a tin beside the carving tools.",
            ),
        ]
    } else {
        [
            trial(
                "band-proof",
                "cut an unmarked band through the repeated strokes where a path turned between two walls",
                "I hung the print without its caption under a single lamp and set the visitor mark six paces back.",
                counts[0],
            ),
            trial(
                "cluster-proof",
                "carved a knot of closely spaced strokes around the bend in a path, with wider spacing everywhere else",
                "The caption stayed covered and the proof was lit by one lamp. Each visitor used the mark six paces back.",
                counts[1],
            ),
        ]
    };
    vec![
        entry(
            "registration",
            "I ruled the registration marks on the back of a spare steel ruler's paper guide and filed the guide in the flat drawer.",
            false,
        ),
        entry(
            "visitors",
            "For a workshop viewing I made a relief print of a path turning between low walls. I wanted the turn to be the first place a viewer noticed.",
            true,
        ),
        band,
        entry(
            "paper-stock",
            "I stacked the dry offcuts beneath a board so the paper would stay flat while the ink dried on the larger sheets.",
            false,
        ),
        cluster,
        entry(
            "edge-test",
            "I wiped ink from the sides of the block with a rag and checked that the paper picked up a clean edge along all four sides.",
            false,
        ),
        entry(
            "cleanup",
            "I put the dried prints on the wide shelf in separate sleeves. The lino scraps went into the small tray beside the roller.",
            true,
        ),
        entry(
            "delivery",
            "A courier collected a packet of test sheets for the workshop open day. I kept the duplicate receipts in a folder by the door.",
            false,
        ),
    ]
}
