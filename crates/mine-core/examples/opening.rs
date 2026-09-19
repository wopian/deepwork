//! Deterministic opening acceptance. No grants, debug digging, or hidden ore targeting.
use mine_core::{materials, Action, Game};
fn main() {
    let cat = materials();
    let count = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(30);
    let mode = std::env::args().nth(2).unwrap_or("guided".into());
    assert!(["guided", "adverse"].contains(&mode.as_str()));
    let mut failed = false;
    for seed in 0..count {
        let mut g = Game::new(seed, 1);
        let buy = |g: &mut Game, id: &str| {
            g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "buy".into(),
                target: id.into(),
                value: 0,
            })
            .is_ok()
        };
        if mode == "adverse" {
            buy(&mut g, "housing");
            g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "pin".into(),
                target: "furnace".into(),
                value: 0,
            })
            .unwrap();
        } else {
            buy(&mut g, "worker");
        }
        let mut conveyor = None;
        let mut shaft = None;
        let mut first_sale = None;
        let mut first_iron = None;
        let mut furnace = None;
        let mut branch = None;
        for second in 1..=if mode == "adverse" { 1800 } else { 600 } {
            // The guide pins the furnace; its actual quote controls the purchase.
            if g.level("furnace") == 0 && g.purchase_blocker("furnace").is_none() {
                if g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "buy".into(),
                    target: "furnace".into(),
                    value: 0,
                })
                .is_ok()
                {
                    furnace = Some(second);
                }
            }
            if g.level("furnace") > 0 && g.level("conveyor") == 0 && buy(&mut g, "conveyor") {
                conveyor = Some(second);
            }
            if g.level("furnace") > 0 && g.pinned.as_deref() != Some("shaft") {
                g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "pin".into(),
                    target: "shaft".into(),
                    value: 0,
                })
                .unwrap();
            }
            if g.level("conveyor") > 0 && g.level("shaft") == 0 && buy(&mut g, "shaft") {
                shaft = Some(second);
            }
            g.second(&cat, false);
            if g.sold_mass > 0 {
                first_sale.get_or_insert(second);
            }
            if g.collection.contains("iron") {
                first_iron.get_or_insert(second);
            }
            if g.workings.passages.iter().any(|p| p.feet[0] != 256) {
                branch.get_or_insert(second);
            }
        }
        let pass = first_sale.is_some_and(|s| s <= 60)
            && first_iron.is_some_and(|s| s <= if mode == "guided" { 300 } else { 1800 })
            && branch.is_some();
        failed |= !pass;
        println!(
            "{}",
            serde_json::json!({"seed":seed,"mode":mode,"pass":pass,"conveyor":conveyor,"shaft":shaft,"sale":first_sale,"furnace":furnace,"iron":first_iron,"branch":branch,"cells":g.excavated,"depth":g.depth(),"credits":g.credits,"status":g.workings.status,"stages":g.stages})
        );
    }
    if failed {
        std::process::exit(1);
    }
}
