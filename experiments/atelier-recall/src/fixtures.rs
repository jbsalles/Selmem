use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Interruption,
    Concentration,
    Both,
    Other,
}

pub struct Workshop {
    pub id: &'static str,
    pub medium: &'static str,
    target: &'static str,
    observer: &'static str,
    changes: [&'static str; 2],
    pub briefs: &'static [&'static str],
}

pub const CALIBRATION: Workshop = Workshop {
    id: "printmaking",
    medium: "linocut",
    target: "the turn",
    observer: "Jo",
    changes: [
        "I cut a blank band across repeated strokes at the path's turn.",
        "I cut a knot of closely spaced strokes at the path's turn.",
    ],
    briefs: &[
        "Arrange eighteen identical paper squares across two equally lit boards so the middle of the left board draws the first glance.",
        "Use sixteen strings in total across two side-by-side rectangular frames. Make the middle of the right frame draw the first glance.",
        "Use twelve bowls in total on one table split into two equal fields. Make the center of the right field draw the first glance.",
        "Direct eight performers in two equal stage areas. During the silent second half, draw the first glance to the middle of the left area. Keep everyone visible.",
        "Use twelve identical discs in total and a fixed lamp. Make the middle of the right half of a wide shadow screen draw the first glance.",
        "Arrange twenty wooden blocks in total across two equal shelves. Make the middle of the left shelf draw the first glance.",
    ],
};

pub const HOLDOUT: [Workshop; 2] = [
    Workshop {
        id: "weaving",
        medium: "woven linen sampler",
        target: "the crossing",
        observer: "Ari",
        changes: [
            "I broke the regular diagonal stitches with an open band at the crossing.",
            "I placed the diagonal stitches closer together at the crossing.",
        ],
        briefs: &[
            "Place exactly ten cards on a felt hanging divided into two equal fields. Use every card once. Direct the first glance to the upper-right target on the left field.",
            "On a long bench, place exactly fourteen identical wooden knobs across two equal zones. Use every knob once. Direct the first glance to the middle of the right zone.",
            "Arrange exactly twelve identical paper arcs in one window with two equally lit sections. Use every arc once. Direct the first glance to the lower-left target on the right section.",
        ],
    },
    Workshop {
        id: "glass",
        medium: "etched glass panel",
        target: "the central junction",
        observer: "Remy",
        changes: [
            "I left a clear channel through the repeated etched lines at the central junction.",
            "I drew the etched lines closer together at the central junction.",
        ],
        briefs: &[
            "Place exactly sixteen identical pegs on a board divided into two equal regions. Use every peg once. Direct the first glance to the bottom corner of the left region.",
            "Mount exactly eight identical clay discs across two equally lit rectangular niches. Use every disc once. Direct the first glance to the center of the right niche.",
            "Place exactly twelve identical paper fans across two panels of the same size. Use every fan once. Direct the first glance to the top center of the left panel.",
        ],
    },
];

#[derive(Serialize)]
pub struct Episode {
    pub id: String,
    pub day: usize,
    pub report: String,
    pub notes: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_and_result: Option<String>,
}

impl Episode {
    pub fn text(&self) -> String {
        format!("{} {}", self.report, self.notes)
    }
}

pub const NIGHTS: usize = 8;

pub fn history(workshop: &Workshop, mirror: usize, distractors: usize) -> Vec<Episode> {
    let mut events = Vec::new();
    for (i, change) in workshop.changes.iter().enumerate() {
        let successes = if i == mirror { 5 } else { 1 };
        let action_and_result = format!(
            "{change} {} recorded that {successes} of 6 visitors noticed {} first.",
            workshop.observer, workshop.target,
        );
        events.push(Episode {
            id: format!("trial-{}", i + 1),
            day: 0,
            report: format!(
                "The maker said: I showed the {} straight on without a caption. {action_and_result}",
                workshop.medium,
            ),
            notes: [
                "I used the back press that morning and cleaned the tools just before lunch. I recorded the material in the studio inventory and set the spare pieces aside.",
                "I started from the second sample on the rack and put it away that afternoon. The studio invoice and the delivery slip went in the wooden drawer beside the tools.",
            ][i],
            action_and_result: Some(action_and_result),
        });
    }
    let materials = [
        "matte paper",
        "wood",
        "cotton",
        "clay",
        "acrylic",
        "metal",
        "cork",
        "felt",
    ];
    let notes = [
        (
            "I tested how readers noticed price labels first at the shop counter",
            "The count was recorded after staff pointed out the rack.",
        ),
        (
            "I arranged identical pieces along a shelf and tested which stacks stayed stable",
            "The notes mention a first glance but record no viewer choice.",
        ),
        (
            "I moved a lamp over the left display and asked which corner looked brighter",
            "The panel had a caption and the light position changed during the test.",
        ),
        (
            "I drew a blank band to reserve room for a shipping label",
            "The visitors reviewed the label after standing close to the sample.",
        ),
        (
            "I gathered strokes at a border to check whether the pattern survived trimming",
            "The report concerns which sample was easiest to cut.",
        ),
        (
            "I hung squares beside an open doorway and tracked where visitors waited",
            "The traffic count used each person's last position before they left.",
        ),
        (
            "I compared the right and left fields after outlining the target in red",
            "All viewers saw the target marked before the session began.",
        ),
        (
            "I arranged identical parts on an open table to guide volunteers packing boxes",
            "The first entry in the record is a count of boxes completed.",
        ),
    ];
    for i in 0..distractors {
        let (activity, detail) = notes[i % notes.len()];
        let material = materials[(i / notes.len()) % materials.len()];
        events.push(Episode {
            id: format!("note-{:03}", i + 1),
            day: 1 + i * NIGHTS / distractors,
            report: format!(
                "The maker said: At the {} workshop, {activity} while working with {material}.",
                workshop.medium
            ),
            notes: detail,
            action_and_result: None,
        });
    }
    events
}
