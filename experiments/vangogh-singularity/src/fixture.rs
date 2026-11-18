//! Held-out visual briefs. The first option is the preregistered tonal choice.
pub const BRIEFS: [(&str, &str, &str); 12] = [
    ("Paint a laundromat at closing. The seating is empty and the washing machines must remain recognizable.", "Build the room from close neutral values and let light model the worn seats.", "Set saturated blue machines against a single orange band of wall so the color opposition carries the room."),
    ("Design a poster for a late ferry with no lettering and one passenger seen from behind.", "Model the passenger and rail in restrained grays and browns, with the brightest value on the departing shore.", "Use a flat violet sea and a yellow passenger shape to make separation register through color."),
    ("Paint a worker repairing a torn umbrella under a market awning. Keep the action readable at a distance.", "Make the hands the lightest passage and model the fabric with adjacent earth tones.", "Put an intense red umbrella against a green awning and flatten the hand shapes so the color boundary leads to the action."),
    ("Paint a flooded underpass using three paints. Show that the air is cold without showing people.", "Use a narrow blue-gray range with dark structural masses and soft reflected light.", "Set high-chroma turquoise water against a flat coral retaining wall to suggest unease through color tension."),
    ("Design a single-panel image of a greenhouse whose plants were recently removed.", "Use muted brown-green values and render the ghost rectangles left on the benches by the pots.", "Make the empty benches a saturated violet field against an acidic yellow roof and use color to make the absence conspicuous."),
    ("Paint an improvised breakfast on a metal tray in a dim break room.", "Arrange the bread and cup in a close tonal range and model their weight under one light.", "Use a blue tray and orange bread as broad adjacent shapes, with minimal internal modeling."),
    ("Design a small image of a stair landing that feels welcoming despite its bare walls.", "Set soft ochre and gray value gradations around the stair rail, with light pooling on the most worn step.", "Paint a flat red stair against a clear green wall and use their color contrast to pull the eye upward."),
    ("Paint two empty work gloves left on a concrete bench. Convey the fatigue of the person who left them.", "Use subdued earthen tones and close observation of the creases and weight of the gloves.", "Give the gloves a saturated yellow and place them on a violet bench so the color opposition carries the fatigue."),
    ("Paint a bus shelter on a hot afternoon. Show nobody inside and include no lettering.", "Build the heat from compressed middle values and a pale sky looming above dense shadows.", "Oppose a flat blue shelter with a strong orange pavement so the relationship between hues supplies the heat."),
    ("Design a still life from a red spool, a steel needle, and a scrap of fabric. Make the needle the point of attention.", "Subdue the spool and use a single high-value glint on the needle against a dark fold.", "Place the needle on the boundary between red and green so the color break locates it."),
    ("Paint a row of damp coats on a community center rack. Make the drying process legible.", "Give the wet cloth closely spaced dark values and lift the dry edges gradually through the fabric texture.", "Use strongly contrasting hue bands at the wet-dry boundary and minimize tonal modeling."),
    ("Design a pair of interior paintings of a station booth before and after its last shift. Keep the furniture fixed.", "Shift the direction and value of the same restrained lighting across the furniture.", "Keep the value structure close and switch which of two saturated complementary colors dominates the booth."),
];

use crate::Result;
use roxmltree::{Document, Node};
use serde::Serialize;
use std::{fs, path::Path};

pub const PASSAGES: usize = 768;
const WINDOWS: [(&str, &str, &str); 2] = [
    ("Nuenen", "1883-12-01", "1885-11-30"),
    ("Arles", "1888-02-01", "1889-05-31"),
];

#[derive(Serialize)]
pub struct Passage {
    letter: String,
    date: String,
    paragraph: usize,
    pub text: String,
}

fn body_text(node: Node<'_, '_>, out: &mut String) {
    if node.is_text() {
        out.push_str(node.text().unwrap_or(""));
        return;
    }
    match node.tag_name().name() {
        "note" | "ptr" | "figure" | "figDesc" | "del" => return,
        "choice" => {
            if let Some(child) = node
                .children()
                .find(|n| n.has_tag_name("reg") || n.has_tag_name("corr"))
                .or_else(|| node.children().find(Node::is_element))
            {
                body_text(child, out);
            }
            return;
        }
        "lb" | "pb" => out.push(' '),
        _ => {}
    }
    for child in node.children() {
        body_text(child, out);
    }
}

pub fn histories(dir: &Path) -> Result<[Vec<Passage>; 2]> {
    let mut candidates = [Vec::new(), Vec::new()];
    let mut paths = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        let Some(id) = path.file_stem().and_then(|n| n.to_str()) else {
            continue;
        };
        if !id.starts_with("let") || path.extension().and_then(|s| s.to_str()) != Some("xml") {
            continue;
        }
        let xml = fs::read_to_string(&path)?;
        let doc = Document::parse(&xml)?;
        let Some(sent) = doc
            .descendants()
            .find(|n| n.has_tag_name("correspAction") && n.attribute("type") == Some("sent"))
        else {
            continue;
        };
        let field = |tag| sent.children().find(|n| n.has_tag_name(tag));
        if field("rs").and_then(|n| n.text()) != Some("Vincent van Gogh") {
            continue;
        }
        let Some(date) = field("date").and_then(|n| n.attribute("when")) else {
            continue;
        };
        if date.len() != 10 {
            continue;
        }
        let Some(place) = field("placeName").and_then(|n| n.text()) else {
            continue;
        };
        let Some(index) = WINDOWS
            .iter()
            .position(|&(p, from, to)| p == place && from <= date && date <= to)
        else {
            continue;
        };
        let Some(body) = doc.descendants().find(|n| {
            n.has_tag_name("div")
                && n.attribute("type") == Some("translation")
                && n.attribute(("http://www.w3.org/XML/1998/namespace", "lang")) == Some("en")
        }) else {
            continue;
        };
        for (paragraph, node) in body
            .descendants()
            .filter(|n| {
                n.has_tag_name("p")
                    && !n.ancestors().skip(1).any(|a| {
                        a.has_tag_name("note") || a.has_tag_name("figure") || a.has_tag_name("p")
                    })
            })
            .enumerate()
        {
            let mut raw = String::new();
            body_text(node, &mut raw);
            let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            if !(12..=76).contains(&text.split_whitespace().count()) {
                continue;
            }
            let text = text
                .replace("Vincent van Gogh", "the painter")
                .replace("Nuenen", "this town")
                .replace("Arles", "this town");
            candidates[index].push(Passage {
                letter: id.into(),
                date: date.into(),
                paragraph,
                text,
            });
        }
    }
    for (index, entries) in candidates.iter_mut().enumerate() {
        entries.sort_by(|a, b| {
            (&a.date, &a.letter, a.paragraph).cmp(&(&b.date, &b.letter, b.paragraph))
        });
        if entries.len() < PASSAGES {
            return Err(format!(
                "{} has only {} eligible paragraphs",
                WINDOWS[index].0,
                entries.len()
            )
            .into());
        }
        let len = entries.len();
        *entries = std::mem::take(entries)
            .into_iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                (((i + 1) * PASSAGES / len) > (i * PASSAGES / len)).then_some(entry)
            })
            .collect();
    }
    Ok(candidates)
}
